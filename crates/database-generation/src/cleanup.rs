//! Post-parse cleanup and magnitude limiting.

use crate::catalog::CatalogEntry;

/// Drop invalid entries, sort by ascending magnitude, and apply a magnitude limit.
///
/// If `limit` is `None`, no magnitude cut is applied (only cleanup/sort).
pub fn clean_and_limit(entries: &mut Vec<CatalogEntry>, limit: Option<f64>) {
    entries.retain(|e| !(e.ra == 0.0 && e.dec == 0.0));
    entries.sort_by(|a, b| {
        a.mag
            .partial_cmp(&b.mag)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(limit) = limit {
        entries.retain(|e| e.mag <= limit);
    }
}

/// Derive the automatic magnitude limit from a cumulative histogram.
///
/// `entries` must already be sorted by ascending magnitude.
/// Returns the smallest magnitude `m` such that the cumulative count of stars
/// with magnitude ≤ `m` first exceeds `total_stars_needed`.
pub fn derive_magnitude_limit(entries: &[CatalogEntry], total_stars_needed: f64) -> Option<f64> {
    if total_stars_needed <= 0.0 || entries.is_empty() {
        return None;
    }
    let needed = total_stars_needed.ceil() as usize;
    let mut count = 0usize;
    for entry in entries {
        count += 1;
        if count >= needed {
            return Some(entry.mag);
        }
    }
    None
}
