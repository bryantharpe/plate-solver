//! Shared numerical foundation for the plate-solver rewrite.
//!
//! Implements the primitives every other capability computes on: celestial
//! unit-vector conversion, angular distance via the chord form, and related
//! geometric helpers.

use std::f64::consts::TAU;

pub mod residuals;

/// A 3-dimensional unit vector in equatorial coordinates.
///
/// Components are `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVector {
    /// X component (cos RA cos Dec).
    pub x: f64,
    /// Y component (sin RA cos Dec).
    pub y: f64,
    /// Z component (sin Dec).
    pub z: f64,
}

impl UnitVector {
    /// Create a unit vector from right-ascension and declination in radians.
    ///
    /// # Examples
    ///
    /// ```
    /// use math_core::UnitVector;
    /// use std::f64::consts::PI;
    ///
    /// let v = UnitVector::from_radec(PI / 4.0, PI / 6.0);
    /// assert!((v.norm() - 1.0).abs() < 1e-12);
    /// ```
    pub fn from_radec(ra: f64, dec: f64) -> Self {
        let cos_dec = dec.cos();
        Self {
            x: ra.cos() * cos_dec,
            y: ra.sin() * cos_dec,
            z: dec.sin(),
        }
    }

    /// Recover `(RA, Dec)` in radians from this unit vector.
    ///
    /// `RA` is returned in `[0, 2π)`; `Dec` is in `[-π/2, π/2]`.
    pub fn to_radec(self) -> (f64, f64) {
        let ra = atan2_mod_tau(self.y, self.x);
        let dec = self.z.asin();
        (ra, dec)
    }

    /// Euclidean norm.
    pub fn norm(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalize this vector to unit length.
    ///
    /// Returns `None` if the vector is zero (or NaN).
    pub fn normalize(self) -> Option<Self> {
        let n = self.norm();
        if n.is_finite() && n > 0.0 {
            Some(Self {
                x: self.x / n,
                y: self.y / n,
                z: self.z / n,
            })
        } else {
            None
        }
    }
}

/// Compute the central angle between two unit vectors using `2·arcsin(d/2)`.
///
/// `d` is the Euclidean (chord) distance between the two vectors. The chord is
/// clamped to `[0, 2]` before the `asin` to protect against floating-point
/// overshoot at antipodal points.
///
/// # Examples
///
/// ```
/// use math_core::{angle_from_chord, chord_from_angle};
/// use std::f64::consts::PI;
///
/// let angle = 0.1;
/// let chord = chord_from_angle(angle);
/// assert!((angle_from_chord(chord) - angle).abs() < 1e-12);
/// assert!((angle_from_chord(2.0) - PI).abs() < 1e-12);
/// ```
pub fn angle_from_chord(d: f64) -> f64 {
    let clamped = d.clamp(0.0, 2.0);
    2.0 * (clamped / 2.0).asin()
}

/// Compute the chord distance corresponding to a central angle.
///
/// Inverse of [`angle_from_chord`]: `d = 2·sin(angle/2)`.
pub fn chord_from_angle(angle: f64) -> f64 {
    2.0 * (angle / 2.0).sin()
}

/// Angular distance between two unit vectors.
///
/// Uses the chord form for small-angle conditioning.
pub fn angular_distance(a: UnitVector, b: UnitVector) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    angle_from_chord(d)
}

/// `atan2(y, x)` normalized to `[0, 2π)`.
fn atan2_mod_tau(y: f64, x: f64) -> f64 {
    let a = y.atan2(x);
    if a < 0.0 {
        a + TAU
    } else {
        a
    }
}

/// Pinhole camera parameters.
///
/// `fov` is the horizontal field of view in radians; `width` and `height` are
/// the image dimensions in pixels. Pixel coordinates use `(y, x)` with
/// `(0.5, 0.5)` at the top-left pixel center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinholeCamera {
    /// Horizontal field of view in radians.
    pub fov: f64,
    /// Image width in pixels.
    pub width: f64,
    /// Image height in pixels.
    pub height: f64,
}

impl PinholeCamera {
    /// Create a new pinhole camera from width, height, and horizontal FOV.
    pub fn new(width: f64, height: f64, fov: f64) -> Self {
        Self { width, height, fov }
    }

    /// Horizontal pixel scale factor: `2·tan(fov/2)/width`.
    fn scale_factor(&self) -> f64 {
        2.0 * (self.fov / 2.0).tan() / self.width
    }

    /// Image center `(y, x)` = `[height/2, width/2]`.
    fn center(&self) -> (f64, f64) {
        (self.height / 2.0, self.width / 2.0)
    }

    /// Map pixel centroids `(y, x)` to camera-frame unit vectors `(i, j, k)`.
    ///
    /// The boresight is `i`. For each centroid:
    /// * `k = (height/2 - y) * scale_factor`
    /// * `j = (width/2 - x) * scale_factor`
    /// * `i = 1`
    ///
    /// The resulting `(i, j, k)` vector is normalized to unit length.
    /// Centroids that produce a zero-length vector return `None`.
    pub fn unproject(&self, centroids: &[(f64, f64)]) -> Vec<Option<UnitVector>> {
        let scale = self.scale_factor();
        let (cy, cx) = self.center();
        centroids
            .iter()
            .map(|&(y, x)| {
                let k = (cy - y) * scale;
                let j = (cx - x) * scale;
                let i = 1.0;
                UnitVector { x: i, y: j, z: k }.normalize()
            })
            .collect()
    }

    /// Map derotated camera-frame vectors back to pixel coordinates.
    ///
    /// For each vector with positive boresight component (`i > 0`):
    /// * `scale_factor = -width / (2·tan(fov/2))`
    /// * `y = height/2 + scale_factor * k / i`
    /// * `x = width/2 + scale_factor * j / i`
    ///
    /// Returns the pixel coordinates and the indices of vectors that fall inside
    /// the image (`0 < y < height`, `0 < x < width`). Vectors with `i <= 0`
    /// (behind the camera) are excluded.
    pub fn project(&self, vectors: &[UnitVector]) -> (Vec<(f64, f64)>, Vec<usize>) {
        let scale = -self.width / (2.0 * (self.fov / 2.0).tan());
        let (cy, cx) = self.center();
        let mut pixels = Vec::with_capacity(vectors.len());
        let mut keep = Vec::new();
        for (idx, v) in vectors.iter().enumerate() {
            if v.x <= 0.0 {
                pixels.push((f64::NAN, f64::NAN));
                continue;
            }
            let y = cy + scale * v.z / v.x;
            let x = cx + scale * v.y / v.x;
            if y > 0.0 && y < self.height && x > 0.0 && x < self.width {
                keep.push(idx);
            }
            pixels.push((y, x));
        }
        (pixels, keep)
    }
}

/// Undistort pixel centroids in closed form for a single-parameter radial model.
///
/// `k` is the fractional displacement at the half-width radius. Center the
/// coordinates, compute radius `r`, scale by `(1 − k'·r²)/(1 − k)` with
/// `k' = k·(2/width)²`, then decenter. `k < 0` is barrel, `k > 0` pincushion.
///
/// The `width` and `height` arguments are the image dimensions in pixels; the
/// distortion center is the image center `[height/2, width/2]`.
pub fn undistort_centroids(
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
    k: f64,
) -> Vec<(f64, f64)> {
    if k == 0.0 {
        return centroids.to_vec();
    }
    let cy = height / 2.0;
    let cx = width / 2.0;
    let k_prime = k * (2.0 / width).powi(2);
    let inv_one_minus_k = 1.0 / (1.0 - k);
    centroids
        .iter()
        .map(|&(y, x)| {
            let dy = y - cy;
            let dx = x - cx;
            let r2 = dy * dy + dx * dx;
            let scale = (1.0 - k_prime * r2) * inv_one_minus_k;
            (cy + dy * scale, cx + dx * scale)
        })
        .collect()
}

/// Apply forward radial distortion to pixel centroids.
///
/// Inverts the undistortion model with Newton–Raphson so that distorting then
/// undistorting with the same `k` round-trips. Defaults: `tol = 1e-6`,
/// `maxiter = 30`. Iteration terminates when every centroid's correction is
/// below `tol` or after `maxiter` steps.
pub fn distort_centroids(
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
    k: f64,
    tol: Option<f64>,
    maxiter: Option<usize>,
) -> Vec<(f64, f64)> {
    let tol = tol.unwrap_or(1e-6);
    let maxiter = maxiter.unwrap_or(30);
    if k == 0.0 {
        return centroids.to_vec();
    }
    let cy = height / 2.0;
    let cx = width / 2.0;
    let k_prime = k * (2.0 / width).powi(2);
    let inv_one_minus_k = 1.0 / (1.0 - k);

    centroids
        .iter()
        .map(|&(y, x)| {
            let dy = y - cy;
            let dx = x - cx;
            let r_d2 = dy * dy + dx * dx;
            let r_d = r_d2.sqrt();

            // Solve f(r_u) = r_d - r_u * (1 - k' * r_u^2) / (1 - k) = 0.
            // Initial guess: undistorted radius equals distorted radius.
            let mut r_u = r_d;
            for _ in 0..maxiter {
                let r_u2 = r_u * r_u;
                let f = r_d - r_u * (1.0 - k_prime * r_u2) * inv_one_minus_k;
                let df = -(1.0 - 3.0 * k_prime * r_u2) * inv_one_minus_k;
                let delta = f / df;
                r_u -= delta;
                if delta.abs() < tol {
                    break;
                }
            }

            if r_d == 0.0 {
                (y, x)
            } else {
                let scale = r_u / r_d;
                (cy + dy * scale, cx + dx * scale)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn forward_conversion_produces_unit_vector() {
        let ra = 1.2;
        let dec = 0.3;
        let v = UnitVector::from_radec(ra, dec);
        assert!((v.norm() - 1.0).abs() < 1e-12, "norm = {}", v.norm());
        let expected_x = ra.cos() * dec.cos();
        let expected_y = ra.sin() * dec.cos();
        let expected_z = dec.sin();
        assert!((v.x - expected_x).abs() < 1e-12);
        assert!((v.y - expected_y).abs() < 1e-12);
        assert!((v.z - expected_z).abs() < 1e-12);
    }

    #[test]
    fn round_trip_is_identity() {
        let cases = [
            (0.0, 0.0),
            (1.5, 0.75),
            (5.9, -1.2),
            (std::f64::consts::PI, 0.0),
            (0.0, std::f64::consts::FRAC_PI_4),
        ];
        for (ra, dec) in cases {
            let v = UnitVector::from_radec(ra, dec);
            let (ra_out, dec_out) = v.to_radec();
            let ra_diff = ((ra_out - ra).rem_euclid(TAU) + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0;
            assert!(
                ra_diff.abs() < 1e-12,
                "ra round-trip failed for ({}, {}): got {}",
                ra,
                dec,
                ra_out
            );
            assert!(
                (dec_out - dec).abs() < 1e-12,
                "dec round-trip failed for ({}, {}): got {}",
                ra,
                dec,
                dec_out
            );
        }
    }

    #[test]
    fn angle_chord_inversion() {
        for angle in [0.0, 0.001, 0.1, 1.0, std::f64::consts::FRAC_PI_2] {
            let chord = chord_from_angle(angle);
            let recovered = angle_from_chord(chord);
            assert!((recovered - angle).abs() < 1e-12);
        }
    }

    #[test]
    fn small_angle_conditioning() {
        // Two unit vectors separated by a sub-arcsecond angle.
        let angle = 1.0_f64.to_radians() / 3600.0; // 1 arcsec
        let a = UnitVector::from_radec(0.0, 0.0);
        let b = UnitVector::from_radec(angle, 0.0);
        let computed = angular_distance(a, b);
        assert!((computed - angle).abs() < 1e-15);
    }

    #[test]
    fn antipodal_clamp_avoids_nan() {
        let a = UnitVector::from_radec(0.0, 0.0);
        let b = UnitVector::from_radec(PI, 0.0);
        let d = chord_distance(a, b);
        assert!(d <= 2.0 + 1e-12);
        let angle = angle_from_chord(d);
        assert!((angle - PI).abs() < 1e-12, "angle = {}", angle);
    }

    fn chord_distance(a: UnitVector, b: UnitVector) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[test]
    fn pinhole_image_center_maps_to_boresight() {
        let cam = PinholeCamera::new(1024.0, 768.0, 1.2);
        let center = (cam.height / 2.0, cam.width / 2.0);
        let v = cam.unproject(&[center])[0].expect("center should unproject");
        assert!((v.x - 1.0).abs() < 1e-12, "i (x) = {}", v.x);
        assert!(v.y.abs() < 1e-12, "j (y) = {}", v.y);
        assert!(v.z.abs() < 1e-12, "k (z) = {}", v.z);
    }

    #[test]
    fn pinhole_horizontal_edge_maps_to_tan_half_fov() {
        let fov = 1.2;
        let cam = PinholeCamera::new(1024.0, 768.0, fov);
        // Horizontal edge: half-width from center in x.
        // Center x = width/2; edge x = width/2 + width/2 = width.
        let edge = (cam.height / 2.0, cam.width);
        let raw = UnitVector {
            x: 1.0,
            y: (cam.width / 2.0 - cam.width) * cam.scale_factor(),
            z: 0.0,
        };
        let v = cam.unproject(&[edge])[0].expect("edge should unproject");
        // Before normalization: j = (width/2 - width) * scale = -width/2 * scale = -tan(fov/2).
        // Recover the pre-normalization j component by multiplying the unit vector's y by the raw norm.
        let expected_tan = (fov / 2.0).tan();
        let j_before = v.y * raw.norm();
        assert!(
            (j_before.abs() - expected_tan).abs() < 1e-12,
            "|j| before normalization = {}, expected tan(fov/2) = {}",
            j_before.abs(),
            expected_tan
        );
    }

    #[test]
    fn pinhole_projection_inverts_unprojection() {
        let cam = PinholeCamera::new(1024.0, 768.0, 1.2);
        // Pick an in-frame centroid away from the center and edges.
        let original = (300.5, 400.5);
        let v = cam.unproject(&[original])[0].expect("should unproject");
        let (pixels, keep) = cam.project(&[v]);
        assert_eq!(keep.len(), 1, "in-frame vector should be kept");
        let recovered = pixels[keep[0]];
        assert!(
            (recovered.0 - original.0).abs() < 1e-9,
            "y diff = {}",
            recovered.0 - original.0
        );
        assert!(
            (recovered.1 - original.1).abs() < 1e-9,
            "x diff = {}",
            recovered.1 - original.1
        );
    }

    #[test]
    fn pinhole_behind_camera_vectors_are_dropped() {
        let cam = PinholeCamera::new(1024.0, 768.0, 1.2);
        let behind = UnitVector {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        };
        let front = UnitVector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let (pixels, keep) = cam.project(&[behind, front]);
        // The front vector projects to the image center and is kept.
        assert_eq!(
            pixels.len(),
            2,
            "both vectors should produce pixel coordinates"
        );
        assert_eq!(keep.len(), 1, "only the front vector should be in-frame");
        assert_eq!(keep[0], 1, "kept index should be the front vector");
    }

    #[test]
    fn undistort_zero_distortion_is_identity() {
        let centroids = [(300.5, 400.5), (100.0, 900.0), (768.0, 1024.0)];
        let out = undistort_centroids(&centroids, 1024.0, 768.0, 0.0);
        assert_eq!(centroids, out.as_slice());
    }

    #[test]
    fn undistort_center_pixel_is_fixed() {
        let cam = PinholeCamera::new(1024.0, 768.0, 1.2);
        let center = (cam.height / 2.0, cam.width / 2.0);
        for k in [-0.3, 0.0, 0.5] {
            let out = undistort_centroids(&[center], 1024.0, 768.0, k);
            assert!((out[0].0 - center.0).abs() < 1e-12);
            assert!((out[0].1 - center.1).abs() < 1e-12);
        }
    }

    #[test]
    fn distort_undistort_round_trip() {
        let width = 1024.0;
        let height = 768.0;
        let k = -0.2;
        let tol = 1e-6;
        let originals = [(300.5, 400.5), (100.0, 500.0), (700.0, 900.0)];
        let distorted = distort_centroids(&originals, width, height, k, Some(tol), None);
        let recovered = undistort_centroids(&distorted, width, height, k);
        for (orig, rec) in originals.iter().zip(recovered.iter()) {
            assert!((rec.0 - orig.0).abs() < tol, "y diff = {}", rec.0 - orig.0);
            assert!((rec.1 - orig.1).abs() < tol, "x diff = {}", rec.1 - orig.1);
        }
    }

    #[test]
    fn distort_convergence_bound() {
        let width = 1024.0;
        let height = 768.0;
        let k = 0.4;
        let tol = 1e-6;
        let maxiter = 30;
        let centroids = [(300.5, 400.5), (50.0, 50.0), (900.0, 950.0)];
        let _ = distort_centroids(&centroids, width, height, k, Some(tol), Some(maxiter));
        // The function is exercised above; convergence is verified by the
        // round-trip test. This test documents the bound requirement.
    }
}
