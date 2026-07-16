//! Density thinning (cluster-buster) for pattern stars.
//!
//! Walks the catalog brightest-first and keeps a star only if no already-kept
//! star lies within a separation radius. This prevents dense clusters from
//! dominating the pattern budget and bounds the number of patterns per FOV.

use crate::catalog::CatalogEntry;
use math_core::{angular_distance, UnitVector};

/// Compute the minimum separation for a target star density.
///
/// `separation = 0.6 * fov / sqrt(stars_per_fov)` where `fov` is in degrees.
pub fn separation_for_density(fov_deg: f64, stars_per_fov: f64) -> f64 {
    0.6 * fov_deg / stars_per_fov.sqrt()
}

/// Greedy brightest-first density thinning.
///
/// `entries` must already be sorted by ascending magnitude (brightest first).
/// Returns the indices of kept stars in the original catalog order, still sorted
/// by brightness because the input is sorted.
///
/// For each star, the function checks whether any already-kept star is within
/// `separation_deg` on the celestial sphere. If not, the star is kept.
///
/// This implementation uses a direct angular scan. Catalogs are small enough at
/// generation time that the O(n²) scan is acceptable and fully deterministic.
pub fn thin_by_density(entries: &[CatalogEntry], separation_deg: f64) -> Vec<usize> {
    let separation_rad = separation_deg.to_radians();
    let mut kept: Vec<usize> = Vec::new();
    let vectors: Vec<UnitVector> = entries
        .iter()
        .map(|e| UnitVector::from_radec(e.ra, e.dec))
        .collect();

    for (i, v) in vectors.iter().enumerate() {
        let mut too_close = false;
        for &k in &kept {
            if angular_distance(*v, vectors[k]) < separation_rad {
                too_close = true;
                break;
            }
        }
        if !too_close {
            kept.push(i);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogEntry, CatalogId};

    fn entry(ra_deg: f64, dec_deg: f64, mag: f64, id: u32) -> CatalogEntry {
        CatalogEntry {
            ra: ra_deg.to_radians(),
            dec: dec_deg.to_radians(),
            mag,
            id: CatalogId::Hip(id),
            pm_ra: None,
            pm_dec: None,
        }
    }

    #[test]
    fn separation_formula_matches_reference() {
        let sep = separation_for_density(10.0, 150.0);
        let expected = 0.6 * 10.0 / 150.0f64.sqrt();
        assert!((sep - expected).abs() < 1e-12);
    }

    #[test]
    fn cluster_keeps_only_brightest() {
        // Six stars: five in a tight cluster, one far away. The brightest overall
        // (id 5, mag 0.5) is inside the cluster and is processed first, so it is
        // kept and excludes its neighbors.
        let mut entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(0.001, 0.0, 2.0, 2),
            entry(0.002, 0.0, 3.0, 3),
            entry(0.003, 0.0, 4.0, 4),
            entry(0.004, 0.0, 0.5, 5), // brightest overall
            entry(10.0, 10.0, 5.0, 6), // far away
        ];
        entries.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap());
        let kept = thin_by_density(&entries, 0.005);
        let kept_ids: Vec<u32> = kept
            .iter()
            .map(|i| match entries[*i].id {
                CatalogId::Hip(n) => n,
                _ => panic!("unexpected id"),
            })
            .collect();
        // Brightest overall (#5) and the outlier (#6) should be kept.
        assert_eq!(kept_ids.len(), 2);
        assert_eq!(kept_ids[0], 5);
        assert_eq!(kept_ids[1], 6);
    }

    #[test]
    fn wide_separation_keeps_many() {
        let entries = vec![
            entry(0.0, 0.0, 1.0, 1),
            entry(5.0, 0.0, 2.0, 2),
            entry(10.0, 0.0, 3.0, 3),
        ];
        let kept = thin_by_density(&entries, 1.0);
        assert_eq!(kept.len(), 3);
    }
}
