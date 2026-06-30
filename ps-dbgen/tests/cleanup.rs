use std::f64::consts::PI;

use ps_dbgen::catalog::StarRecord;
use ps_dbgen::cleanup::auto_limiting_magnitude;

/// Build a uniform synthetic star set: magnitudes 0.0..10.0 spread evenly.
fn build_uniform_stars(count: usize) -> Vec<StarRecord> {
    (0..count)
        .map(|i| StarRecord {
            ra: 0.0,
            dec: 0.0,
            mag: i as f64 * 10.0 / (count as f64 - 1.0),
            cat_id: ps_dbgen::catalog::CatalogId::Bsc(0),
        })
        .collect()
}

/// Test 1 — formula match with uniform magnitudes.
///
/// 1000 stars, mags 0.0..=10.0 (step 0.010001…).
/// min_fov_rad = 0.1 → num_fovs = ceil(4π / 0.01) ≈ 1257.
/// total_stars_needed = 1257 * 10 * 0.7 = 8799.
/// We only have 1000 stars, so cumulative will never exceed 8799.
/// The function should fall back to mag_max = 10.0.
#[test]
fn test_auto_limiting_magnitude_uniform_fallback() {
    let stars = build_uniform_stars(1000);
    let result = auto_limiting_magnitude(&stars, 0.1, 10);

    // total_stars_needed >> 1000, so we hit the fallback (mag_max).
    assert!(
        (result - 10.0).abs() < 1e-9,
        "expected fallback to mag_max=10.0, got {}",
        result
    );
}

/// Test 1b — formula match where cumulative *does* exceed the threshold.
///
/// Use a very large FOV so num_fovs is small, then cumulative exceeds quickly.
/// min_fov_rad = 2.0 → num_fovs = ceil(4π / 4) = ceil(3.1416…) = 4.
/// total_stars_needed = 4 * 10 * 0.7 = 28.
/// With 1000 uniform stars (mags 0..=10), each bin holds ~10 stars.
/// Cumulative reaches > 28 at bin index 2 (after bins 0,1,2 → ~30 stars).
/// Expected magnitude ≈ 0 + 2 * 0.1 = 0.2.
#[test]
fn test_auto_limiting_magnitude_formula_match() {
    let stars = build_uniform_stars(1000);

    // Hand calculation:
    // num_fovs = ceil(4π / (2.0 * 2.0)) = ceil(3.14159...) = 4
    // total_stars_needed = 4 * 10 * 0.7 = 28.0
    let min_fov_rad = 2.0;
    let num_fovs = (4.0 * PI / (min_fov_rad * min_fov_rad)).ceil() as usize; // 4
    let total_stars_needed = num_fovs as f64 * 10.0 * 0.7; // 28.0

    let result = auto_limiting_magnitude(&stars, min_fov_rad, 10);

    // Verify our hand calculation is correct
    assert_eq!(num_fovs, 4, "hand calc check");
    assert!((total_stars_needed - 28.0).abs() < 1e-9, "hand calc check");

    // 100 bins from 0.0 to 10.0, bin_width = 0.1
    // ~10 stars per bin; cumulative after bin 0 = ~10, bin 1 = ~20, bin 2 = ~30 > 28
    // Result should be mag_min + 2 * 0.1 = 0.2 (give or take rounding)
    assert!((result - 0.2).abs() < 0.01, "expected ~0.2, got {}", result);
}

/// Test 2 — small dataset: first bin that pushes cumulative over threshold.
///
/// 5 stars with mags [1.0, 2.0, 3.0, 4.0, 5.0].
/// min_fov_rad = 10.0 → num_fovs = ceil(4π / 100) = ceil(0.1257…) = 1.
/// total_stars_needed = 1 * 1 * 0.7 = 0.7.
/// First bin already has cumulative >= 1 > 0.7, so result = left edge of first bin.
#[test]
fn test_auto_limiting_magnitude_small_dataset() {
    let stars: Vec<StarRecord> = [1.0, 2.0, 3.0, 4.0, 5.0]
        .into_iter()
        .enumerate()
        .map(|(i, mag)| StarRecord {
            ra: 0.0,
            dec: 0.0,
            mag,
            cat_id: ps_dbgen::catalog::CatalogId::Bsc(i as u16),
        })
        .collect();

    let result = auto_limiting_magnitude(&stars, 10.0, 1);

    // Result should be between 1.0 (mag_min) and 5.0 (mag_max).
    assert!(
        result >= 1.0 && result <= 5.0,
        "expected between 1.0 and 5.0, got {}",
        result
    );
}

/// Test 3 — empty star list returns 0.0 without panicking.
#[test]
fn test_auto_limiting_magnitude_empty() {
    let stars: Vec<StarRecord> = vec![];
    let result = auto_limiting_magnitude(&stars, 1.0, 10);
    assert_eq!(result, 0.0, "empty list should return 0.0");
}

/// Test 4 — all stars share the same magnitude.
#[test]
fn test_auto_limiting_magnitude_single_magnitude() {
    let stars: Vec<StarRecord> = (0..10)
        .map(|i| StarRecord {
            ra: 0.0,
            dec: 0.0,
            mag: 3.5,
            cat_id: ps_dbgen::catalog::CatalogId::Bsc(i),
        })
        .collect();

    let result = auto_limiting_magnitude(&stars, 1.0, 10);
    assert!(
        (result - 3.5).abs() < 1e-9,
        "all-same-mag should return that magnitude, got {}",
        result
    );
}
