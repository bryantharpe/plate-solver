use std::f64::consts::PI;

use crate::catalog::StarRecord;

/// Compute an automatic limiting magnitude for a star catalogue.
///
/// Given a set of star records, this estimates the faintest magnitude
/// needed so that `verification_stars_per_fov` stars are available in
/// every `min_fov_rad` field of view across the whole sky.
///
/// The algorithm mirrors `Tetra3.generate_database()` (lines 1114-1142):
/// a 100-bin magnitude histogram is built, then walked cumulatively until
/// enough stars to cover the sky are collected.
///
/// # Arguments
/// * `stars` – parsed star records (already filtered/sorted by the parsers).
/// * `min_fov_rad` – minimum field-of-view in radians; must be > 0.
/// * `verification_stars_per_fov` – target number of verification stars per FOV.
///
/// # Returns
/// The limiting magnitude.  Returns `0.0` when `stars` is empty.
pub fn auto_limiting_magnitude(
    stars: &[StarRecord],
    min_fov_rad: f64,
    verification_stars_per_fov: usize,
) -> f64 {
    if stars.is_empty() {
        return 0.0;
    }

    // num_fovs = ceil(4 * PI / (min_fov * min_fov))
    let num_fovs = (4.0 * PI / (min_fov_rad * min_fov_rad)).ceil() as usize;
    let total_stars_needed = num_fovs as f64 * verification_stars_per_fov as f64 * 0.7;

    // Build magnitude histogram (100 bins)
    let mags: Vec<f64> = stars.iter().map(|s| s.mag).collect();
    let mag_min = mags.iter().cloned().fold(f64::INFINITY, f64::min);
    let mag_max = mags.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (mag_min - mag_max).abs() < f64::EPSILON {
        // All same magnitude – single bin, cumulative hits immediately.
        return mag_max;
    }

    let bin_width = (mag_max - mag_min) / 100.0;
    let mut counts = [0u64; 100];
    for &mag in &mags {
        let idx = (((mag - mag_min) / bin_width) as usize).min(99);
        counts[idx] += 1;
    }

    // Cumulative sum – find first bin where cumulative > total_stars_needed
    let mut cumulative: u64 = 0;
    for (i, &c) in counts.iter().enumerate() {
        cumulative += c;
        if cumulative as f64 > total_stars_needed {
            return mag_min + (i as f64) * bin_width; // left edge of this bin
        }
    }

    // Fallback: should not be reached unless total_stars_needed exceeds all stars.
    mag_max
}
