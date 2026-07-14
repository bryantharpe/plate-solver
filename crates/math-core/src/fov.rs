//! Field-of-view estimation and refinement.
//!
//! Implements the FOV math used after a pattern match: scaling a supplied
//! estimate, solving focal length from pixel/angular correspondences, and
//! refining FOV (and optionally radial distortion) from matched star pairs.

use crate::UnitVector;

/// Estimate horizontal FOV from a matched pattern.
///
/// If `fov_estimate` is provided, the image pixel distance is converted to an
/// angle using the supplied FOV (`image_angle = image_largest_edge * fov_initial /
/// width`), then the refined FOV is `catalog_largest_edge / image_angle *
/// fov_initial`, which simplifies to `catalog_largest_edge * width /
/// image_largest_edge`. If no estimate is supplied, solves focal length from the
/// largest pixel distance and the catalog largest angle, then derives FOV.
///
/// `image_largest_edge` is the largest Euclidean distance between matched
/// centroids in pixels. `catalog_largest_edge` is the corresponding central
/// angle in radians. `width` is the image width in pixels.
///
/// Returns the estimated horizontal FOV in radians.
pub fn estimate_fov(
    fov_estimate: Option<f64>,
    image_largest_edge: f64,
    catalog_largest_edge: f64,
    width: f64,
) -> f64 {
    if let Some(fov_initial) = fov_estimate {
        // Convert pixel distance to angle using the supplied FOV, then scale.
        let image_angle = image_largest_edge * fov_initial / width;
        catalog_largest_edge / image_angle * fov_initial
    } else {
        let f = image_largest_edge / 2.0 / (catalog_largest_edge / 2.0).tan();
        2.0 * ((width / 2.0) / f).atan()
    }
}

/// Compute diagonal FOV from horizontal FOV and image dimensions.
///
/// `fov` is the horizontal field of view in radians. Returns the diagonal FOV
/// in radians, used when gathering nearby catalog stars.
pub fn diagonal_fov(fov: f64, width: f64, height: f64) -> f64 {
    fov * (width * width + height * height).sqrt() / width
}

/// Refine FOV (and optionally distortion) from matched star pairs.
///
/// `camera` and `catalog` are ordered, corresponding unit-vector pairs. With no
/// distortion estimation (`k = None`), the horizontal FOV is scaled by the mean
/// ratio of catalog angle to camera angle over matched pairs.
///
/// With distortion (`k = Some(_)` as a starting value, currently ignored), a
/// least-squares solve for focal length `f` and distortion coefficient `k` is
/// performed from rows `A = [t, r³]`, `b = [r]` per matched star, where `r` is
/// the radial distance of the camera vector on the sensor plane and `t` is the
/// tangent of the catalog angle. The refined FOV is then
/// `fov = 2·arctan(1/f)` after applying `f /= (1 − k)`.
///
/// `width` and `height` are image dimensions in pixels; `fov` is the current
/// horizontal FOV in radians. Returns the refined FOV and, if requested, the
/// refined distortion coefficient.
pub fn refine_fov(
    fov: f64,
    width: f64,
    _height: f64,
    camera: &[UnitVector],
    catalog: &[UnitVector],
    k: Option<f64>,
) -> (f64, Option<f64>) {
    assert_eq!(
        camera.len(),
        catalog.len(),
        "camera and catalog match lists must have the same length"
    );

    if camera.is_empty() {
        return (fov, k);
    }

    let _scale = 2.0 * (fov / 2.0).tan() / width;

    if k.is_none() {
        let mut ratio_sum = 0.0;
        let mut count = 0;
        for (cam, cat) in camera.iter().zip(catalog.iter()) {
            let cam_angle = cam.x.acos();
            let cat_angle = cat.x.acos();
            if cam_angle.is_finite() && cat_angle.is_finite() && cam_angle > 0.0 {
                ratio_sum += cat_angle / cam_angle;
                count += 1;
            }
        }
        if count > 0 {
            let mean_ratio = ratio_sum / count as f64;
            return (fov * mean_ratio, None);
        }
        return (fov, None);
    }

    // Distortion refinement: exact least-squares solve consistent with the
    // radial distortion model used by `undistort_centroids`.
    //
    // The undistortion model relates the distorted camera radius `r_c` to the
    // undistorted catalog radius `r_u`:
    //
    //   r_u = r_c * (1 - k' * r_c^2) / (1 - k),   k' = k * (2/width)^2
    //
    // For a pinhole camera, r_u = f_px * tan(theta) where `theta` is the catalog
    // angle and f_px is the focal length in pixels. Rearranging:
    //
    //   tan(theta) / r_c = (1 - k' * r_c^2) / (f_px * (1 - k))
    //
    // Let y = tan(theta)/r_c and s = r_c^2. Then
    //
    //   y = a + b * s,   a = 1 / (f_px * (1 - k)),   b = -k' / (f_px * (1 - k))
    //
    // We fit this linear model, then recover
    //
    //   c0 = 1/a = f_px * (1 - k)
    //   k' = -b / a
    //   k  = k' / (2/width)^2
    //   f_px = c0 / (1 - k)
    //
    // The refined FOV is `2 * atan(width / (2 * f_px))`.
    let f_px_initial = width / (2.0 * (fov / 2.0).tan());

    let mut ata00 = 0.0;
    let mut ata01 = 0.0;
    let mut ata11 = 0.0;
    let mut atb0 = 0.0;
    let mut atb1 = 0.0;

    for (cam, cat) in camera.iter().zip(catalog.iter()) {
        // Distorted pixel radius from the camera vector.
        let r_c = f_px_initial * (cam.y * cam.y + cam.z * cam.z).sqrt() / cam.x;
        let r_c2 = r_c * r_c;

        // Undistorted tangent from the catalog angle.
        let theta = cat.x.acos();
        if !theta.is_finite() || !r_c.is_finite() || theta <= 0.0 || r_c <= 0.0 {
            continue;
        }
        let t = theta.tan();
        if !t.is_finite() {
            continue;
        }

        let y = t / r_c;
        let a0 = 1.0;
        let a1 = r_c2;

        ata00 += a0 * a0;
        ata01 += a0 * a1;
        ata11 += a1 * a1;
        atb0 += a0 * y;
        atb1 += a1 * y;
    }

    let det = ata00 * ata11 - ata01 * ata01;
    if det == 0.0 || !det.is_finite() {
        return (fov, k);
    }

    let a = (ata11 * atb0 - ata01 * atb1) / det;
    let b = (ata00 * atb1 - ata01 * atb0) / det;

    if !a.is_finite() || !b.is_finite() || a.abs() < 1e-12 {
        return (fov, k);
    }

    let c0 = 1.0 / a;
    let k_prime_est = -b / a;
    let k_est = k_prime_est / (2.0 / width).powi(2);

    if !k_est.is_finite() || (1.0 - k_est).abs() < 1e-12 {
        return (fov, k);
    }

    let f_px = c0 / (1.0 - k_est);
    if !f_px.is_finite() || f_px <= 0.0 {
        return (fov, k);
    }

    let refined_fov = 2.0 * ((width / 2.0) / f_px).atan();

    (refined_fov, Some(k_est))
}

/// Convenience: refine FOV without distortion.
pub fn refine_fov_no_distortion(
    fov: f64,
    width: f64,
    height: f64,
    camera: &[UnitVector],
    catalog: &[UnitVector],
) -> f64 {
    refine_fov(fov, width, height, camera, catalog, None).0
}

/// Convenience: refine FOV and distortion together.
pub fn refine_fov_with_distortion(
    fov: f64,
    width: f64,
    height: f64,
    camera: &[UnitVector],
    catalog: &[UnitVector],
) -> (f64, f64) {
    let (fov_out, k_out) = refine_fov(fov, width, height, camera, catalog, Some(0.0));
    (fov_out, k_out.unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{distort_centroids, PinholeCamera, UnitVector};

    #[test]
    fn supplied_estimate_scales_by_edge_ratio() {
        let fov_initial = 1.2;
        let catalog_largest_edge = 0.05;
        let image_largest_edge = 100.0;
        let width = 1024.0;
        let fov = estimate_fov(
            Some(fov_initial),
            image_largest_edge,
            catalog_largest_edge,
            width,
        );
        // Pixel distance is converted to angle via the supplied FOV before ratio.
        let image_angle = image_largest_edge * fov_initial / width;
        let expected = catalog_largest_edge / image_angle * fov_initial;
        assert!(
            (fov - expected).abs() < 1e-12,
            "fov = {}, expected = {}",
            fov,
            expected
        );
    }

    #[test]
    fn no_estimate_solves_focal_length() {
        let width = 1024.0;
        let catalog_largest_edge = 0.05;
        let image_largest_edge = 100.0;
        let f = image_largest_edge / 2.0f64 / (catalog_largest_edge / 2.0f64).tan();
        let expected = 2.0 * ((width / 2.0f64) / f).atan();
        let fov = estimate_fov(None, image_largest_edge, catalog_largest_edge, width);
        assert!(
            (fov - expected).abs() < 1e-12,
            "fov = {}, expected = {}",
            fov,
            expected
        );
    }

    #[test]
    fn diagonal_fov_relation() {
        let fov = 1.2;
        let width = 1024.0;
        let height = 768.0;
        let expected = fov * ((width * width + height * height) as f64).sqrt() / width;
        let diag = diagonal_fov(fov, width, height);
        assert!((diag - expected).abs() < 1e-12);
    }

    #[test]
    fn no_distortion_refinement_scales_by_mean_ratio() {
        // Build a camera with a known FOV and a catalog that is uniformly scaled.
        let width = 1024.0;
        let height = 768.0;
        let true_fov = 1.2;
        let cam = PinholeCamera::new(width, height, true_fov);

        // Four in-frame centroids.
        let centroids = [
            (height / 2.0, width / 2.0),
            (height / 2.0, width * 0.25),
            (height * 0.25, width / 2.0),
            (height * 0.75, width * 0.75),
        ];
        let camera: Vec<UnitVector> = cam.unproject(&centroids).into_iter().flatten().collect();

        // Catalog vectors are the same directions scaled by 1.05 in angle.
        let catalog: Vec<UnitVector> = camera
            .iter()
            .map(|v| {
                let theta = v.x.acos();
                let new_theta = theta * 1.05;
                // Keep the same (j,k) proportions, recompute i for unit length.
                let r = (v.y * v.y + v.z * v.z).sqrt();
                let scale = if r == 0.0 { 1.05 } else { new_theta.sin() / r };
                UnitVector {
                    x: new_theta.cos(),
                    y: v.y * scale,
                    z: v.z * scale,
                }
                .normalize()
                .unwrap()
            })
            .collect();

        let refined = refine_fov_no_distortion(true_fov, width, height, &camera, &catalog);
        let expected = true_fov * 1.05;
        assert!(
            (refined - expected).abs() < 1e-6,
            "refined = {}, expected = {}",
            refined,
            expected
        );
    }

    #[test]
    fn distortion_refinement_recovers_planted_k() {
        let width = 1024.0;
        let height = 768.0;
        let true_fov = 1.0;
        let planted_k = -0.15;
        let cam = PinholeCamera::new(width, height, true_fov);

        // Synthetic undistorted centroids spread across the frame, including corners.
        let undistorted = [
            (height / 2.0, width / 2.0),
            (50.0, 50.0),
            (50.0, width - 50.0),
            (height - 50.0, 50.0),
            (height - 50.0, width - 50.0),
            (height / 2.0, width * 0.2),
            (height * 0.2, width / 2.0),
            (height * 0.8, width * 0.8),
        ];

        // Distort them to simulate lens distortion.
        let distorted = distort_centroids(&undistorted, width, height, planted_k, Some(1e-9), None);

        // Camera vectors come from the distorted centroids.
        let camera: Vec<UnitVector> = cam.unproject(&distorted).into_iter().flatten().collect();

        // Catalog vectors are the ideal directions from the undistorted centroids.
        let catalog: Vec<UnitVector> = cam.unproject(&undistorted).into_iter().flatten().collect();

        let (refined_fov, refined_k) =
            refine_fov_with_distortion(true_fov, width, height, &camera, &catalog);

        assert!(
            (refined_k - planted_k).abs() < 1e-3,
            "refined_k = {}, planted_k = {}",
            refined_k,
            planted_k
        );
        assert!(
            (refined_fov - true_fov).abs() < 1e-3,
            "refined_fov = {}, expected = {}",
            refined_fov,
            true_fov
        );
    }
}
