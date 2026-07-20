//! Spatial-index tests for the star KD-tree and the `nearby_stars` query.
//!
//! These tests use a fixture with enough stars that a linear scan and a real
//! KD-tree are distinguishable, and verify the public API contract: the index
//! is built at load time, radius queries use the chord metric, and results are
//! returned brightest-first.

use std::time::Instant;

use math_core::{angular_distance, UnitVector};
use pattern_database::{load_from_path, load_mmap, PatternDatabase};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn load_spatial() -> PatternDatabase {
    load_from_path(&fixture("spatial.npz")).expect("load spatial.npz")
}

/// Brute-force radius query against the star table, used as the oracle.
fn brute_force(db: &PatternDatabase, boresight: UnitVector, radius: f64) -> Vec<usize> {
    let mut found: Vec<usize> = (0..db.num_stars)
        .filter(|i| {
            let v = db.star_vector(pattern_database::StarId(*i)).unwrap();
            angular_distance(boresight, v) <= radius + 1e-12
        })
        .collect();
    // Brightness order is index order.
    found.sort();
    found
}

#[test]
fn load_builds_kdtree() {
    let db = load_spatial();
    assert_eq!(db.star_kdtree.size(), db.num_stars);
}

#[test]
fn mmap_load_builds_kdtree() {
    let db = load_mmap(&fixture("spatial.npz")).expect("mmap spatial.npz");
    assert_eq!(db.star_kdtree.size(), db.num_stars);
}

#[test]
fn nearby_stars_returns_brightest_first() {
    let db = load_spatial();
    let boresight = db.star_vector(pattern_database::StarId(0)).unwrap();
    let radius = 15.0_f64.to_radians();

    let nearby = db.nearby_stars(boresight, radius);
    assert!(
        nearby.len() > 3,
        "fixture should contain enough stars to detect order errors"
    );

    // Verify indices are strictly increasing (brightness order).
    for window in nearby.windows(2) {
        assert!(
            window[0] < window[1],
            "nearby_stars must be brightest-first; got {:?}",
            nearby
        );
    }

    // Verify the same set as the brute-force oracle.
    let expected = brute_force(&db, boresight, radius);
    assert_eq!(nearby, expected);
}

#[test]
fn nearby_stars_matches_brute_force_for_several_radii() {
    let db = load_spatial();
    let boresight = UnitVector::from_radec(1.0, 0.3);

    for radius_deg in [0.0_f64, 1.0, 5.0, 15.0, 90.0, 180.0] {
        let radius = radius_deg.to_radians();
        let got = db.nearby_stars(boresight, radius);
        let expected = brute_force(&db, boresight, radius);
        assert_eq!(
            got, expected,
            "mismatch at radius {radius_deg} deg"
        );
    }
}

#[test]
fn nearby_stars_empty_radius_yields_only_boresight() {
    let db = load_spatial();
    let boresight = db.star_vector(pattern_database::StarId(42)).unwrap();
    let got = db.nearby_stars(boresight, 0.0);
    assert_eq!(got, vec![42]);
}

#[test]
fn nearby_stars_all_inclusive_radius_returns_every_star() {
    let db = load_spatial();
    let boresight = UnitVector::from_radec(0.0, 0.0);
    let got = db.nearby_stars(boresight, std::f64::consts::PI);
    assert_eq!(got.len(), db.num_stars);
    let expected: Vec<usize> = (0..db.num_stars).collect();
    assert_eq!(got, expected);
}

#[test]
fn kdtree_query_scales_sublinearly() {
    // Build a sequence of synthetic catalogs and measure query time. For a
    // fixed angular radius on a uniform sphere, the number of stars inside the
    // cap grows linearly with N. A linear scan's query time tracks that growth.
    // A KD-tree query time should grow sublinearly because it prunes most nodes.
    let mut last_count: Option<usize> = None;
    let mut last_time: Option<std::time::Duration> = None;
    let mut prev_n: usize = 0;

    // Fixed 5° radius: the cap covers a constant solid angle, so result count
    // is proportional to N.
    let radius = 5.0_f64.to_radians();
    let boresight = UnitVector::from_radec(1.0, 0.3);

    for n in [400_usize, 1600, 6400, 25600] {
        let db = synthetic_database(n);

        // Average several runs to reduce timing noise.
        let runs = 20;
        for _ in 0..runs {
            let _ = db.nearby_stars(boresight, radius);
        }
        let start = Instant::now();
        for _ in 0..runs {
            let _ = db.nearby_stars(boresight, radius);
        }
        let elapsed = start.elapsed();
        let per_query = elapsed / runs as u32;

        let count = db.nearby_stars(boresight, radius).len();

        if let (Some(prev_count), Some(prev_time)) = (last_count, last_time) {
            let count_ratio = (count as f64).max(1.0) / (prev_count as f64).max(1.0);
            let time_ratio = per_query.as_secs_f64() / prev_time.as_secs_f64();
            // A linear scan would show time_ratio ≈ count_ratio. A KD-tree
            // should be well below that.
            assert!(
                time_ratio < count_ratio * 0.9,
                "query time scaled too fast: N {prev_n} -> {n}, result count {prev_count} -> {count} (ratio {count_ratio:.2}), per-query time ratio {time_ratio:.2}"
            );
        }

        last_count = Some(count);
        last_time = Some(per_query);
        prev_n = n;
    }
}

/// Build a simple in-memory database with `n` stars on a uniform-ish sphere.
fn synthetic_database(n: usize) -> PatternDatabase {
    use pattern_database::{CatalogId, DatabaseProperties};

    let mut star_table = Vec::with_capacity(n * 6);
    let mut star_catalog_ids = Vec::with_capacity(n);

    // Golden-spiral distribution to avoid pathological clustering.
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    for i in 0..n {
        let y = 1.0 - (i as f64 * 2.0) / ((n - 1) as f64);
        let theta = golden_angle * i as f64;
        let r = (1.0 - y * y).sqrt();
        let x = r * theta.cos();
        let z = r * theta.sin();
        // RA/Dec from unit vector.
        let ra = z.atan2(x);
        let dec = y.asin();
        let mag = (i as f64) * 0.01; // brightest-first by index order
        star_table.extend([ra as f32, dec as f32, x as f32, y as f32, z as f32, mag as f32]);
        star_catalog_ids.push(CatalogId::Hip(i as u32 + 1));
    }

    let vectors: Vec<UnitVector> = (0..n)
        .map(|i| {
            let base = i * 6;
            UnitVector {
                x: star_table[base + 2] as f64,
                y: star_table[base + 3] as f64,
                z: star_table[base + 4] as f64,
            }
        })
        .collect();

    PatternDatabase {
        star_table,
        num_stars: n,
        pattern_catalog: Vec::new(),
        pattern_largest_edge: Vec::new(),
        pattern_key_hashes: Vec::new(),
        star_catalog_ids,
        properties: DatabaseProperties::default(),
        star_kdtree: pattern_database::StarKdTree::new(&vectors),
    }
}
