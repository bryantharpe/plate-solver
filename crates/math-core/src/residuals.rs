//! Solution residual statistics: RMSE, P90E, and MAXE in arcseconds.

use crate::{angular_distance, UnitVector};

/// Residual statistics for a set of matched image/catalog star pairs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidualStats {
    /// Root-mean-square error in arcseconds.
    pub rmse: f64,
    /// 90th-percentile error in arcseconds.
    pub p90e: f64,
    /// Maximum error in arcseconds.
    pub maxe: f64,
}

/// Compute residual statistics between matched image and catalog unit vectors.
///
/// For each matched pair the per-star angle is computed as the central angle
/// `2·arcsin(d/2)` where `d` is the Euclidean chord distance between the unit
/// vectors. The returned values are in arcseconds:
///
/// * `RMSE = rad2deg(sqrt(mean(angle²)))·3600`
/// * `P90E` is the 90th percentile of the per-star angles, in arcseconds.
/// * `MAXE` is the largest per-star angle, in arcseconds.
///
/// The input slices must be the same length. An empty match list yields
/// `RMSE = P90E = MAXE = 0.0`.
pub fn residual_stats(image: &[UnitVector], catalog: &[UnitVector]) -> ResidualStats {
    assert_eq!(
        image.len(),
        catalog.len(),
        "image and catalog match lists must have the same length"
    );

    if image.is_empty() {
        return ResidualStats {
            rmse: 0.0,
            p90e: 0.0,
            maxe: 0.0,
        };
    }

    let mut angles: Vec<f64> = image
        .iter()
        .zip(catalog.iter())
        .map(|(img, cat)| angular_distance(*img, *cat))
        .collect();

    angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let maxe_rad = angles[angles.len() - 1];

    let mean_sq = angles.iter().map(|a| a * a).sum::<f64>() / angles.len() as f64;
    let rmse_rad = mean_sq.sqrt();

    let p90_idx = ((angles.len() as f64) * 0.9).ceil() as usize - 1;
    let p90e_rad = angles[p90_idx.min(angles.len() - 1)];

    let rad_to_arcsec = 180.0 / std::f64::consts::PI * 3600.0;

    ResidualStats {
        rmse: rmse_rad * rad_to_arcsec,
        p90e: p90e_rad * rad_to_arcsec,
        maxe: maxe_rad * rad_to_arcsec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn empty_match_list_yields_zero() {
        let stats = residual_stats(&[], &[]);
        assert_eq!(stats.rmse, 0.0);
        assert_eq!(stats.p90e, 0.0);
        assert_eq!(stats.maxe, 0.0);
    }

    #[test]
    fn identical_vectors_yield_zero() {
        let v = UnitVector::from_radec(1.0, 0.5);
        let stats = residual_stats(&[v, v, v], &[v, v, v]);
        assert!(stats.rmse.abs() < 1e-12);
        assert!(stats.p90e.abs() < 1e-12);
        assert!(stats.maxe.abs() < 1e-12);
    }

    #[test]
    fn rmse_reported_in_arcseconds() {
        // Build two stars separated by exactly 1 arcsecond.
        let arcsec = 1.0_f64.to_radians() / 3600.0;
        let a = UnitVector::from_radec(0.0, 0.0);
        let b = UnitVector::from_radec(arcsec, 0.0);

        // Use many identical pairs so RMSE equals the single angle.
        let image = vec![a; 10];
        let catalog = vec![b; 10];
        let stats = residual_stats(&image, &catalog);

        assert!((stats.rmse - 1.0).abs() < 1e-9, "rmse = {}", stats.rmse);
        assert!((stats.p90e - 1.0).abs() < 1e-9, "p90e = {}", stats.p90e);
        assert!((stats.maxe - 1.0).abs() < 1e-9, "maxe = {}", stats.maxe);
    }

    #[test]
    fn p90e_is_ninetieth_percentile() {
        // Ten stars with linearly increasing separations from 1 to 10 arcsec.
        let image: Vec<UnitVector> = (0..10)
            .map(|i| UnitVector::from_radec(0.0, (i as f64) * 1e-4))
            .collect();
        let catalog: Vec<UnitVector> = (0..10)
            .map(|_| UnitVector::from_radec(0.0, 0.0))
            .collect();

        let stats = residual_stats(&image, &catalog);

        // 90th percentile of 10 sorted values: ceil(10 * 0.9) - 1 = 8 -> 9th value (index 8).
        // The 9th smallest separation corresponds to index 8 (1-based: 9th star).
        let expected_p90_rad = angular_distance(image[8], catalog[8]);
        let expected_p90_arcsec = expected_p90_rad.to_degrees() * 3600.0;

        assert!((stats.p90e - expected_p90_arcsec).abs() < 1e-9);
        assert!(stats.p90e <= stats.maxe);
    }

    #[test]
    fn maxe_is_largest_angle() {
        let a = UnitVector::from_radec(0.0, 0.0);
        let b = UnitVector::from_radec(0.01, 0.0);
        let c = UnitVector::from_radec(0.02, 0.0);

        let image = vec![a, a, a];
        let catalog = vec![a, b, c];
        let stats = residual_stats(&image, &catalog);

        let expected_max = angular_distance(a, c).to_degrees() * 3600.0;
        assert!((stats.maxe - expected_max).abs() < 1e-9);
    }

    #[test]
    fn antipodal_residuals_clamped() {
        let a = UnitVector::from_radec(0.0, 0.0);
        let b = UnitVector::from_radec(PI, 0.0);
        let stats = residual_stats(&[a], &[b]);

        assert!((stats.maxe - 180.0 * 3600.0).abs() < 1e-9);
        assert!(stats.p90e <= stats.maxe);
    }
}
