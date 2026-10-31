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
        assert_eq!(got, expected, "mismatch at radius {radius_deg} deg");
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
fn kdtree_query_beats_a_linear_scan() {
    // A KD-tree prunes, so at a size where a linear scan touches every star its
    // radius query must be clearly faster than that scan. One large catalog
    // timed against its own scan, in interleaved batches compared by median,
    // stays stable on a noisy CI runner. The previous version compared timing
    // ratios between catalogs as small as 400 stars, where a query returns 0 to
    // 2 stars in well under a microsecond, and noise alone failed it.
    let n = 51_200;
    let db = synthetic_database(n);
    let radius = 5.0_f64.to_radians();
    let boresight = UnitVector::from_radec(1.0, 0.3);

    let expected = brute_force(&db, boresight, radius);
    assert!(
        expected.len() > 50,
        "the cap should hold enough stars that both queries do real work; got {}",
        expected.len()
    );
    assert_eq!(db.nearby_stars(boresight, radius), expected);

    let batches = 9;
    let runs = 10;
    let mut tree_times = Vec::with_capacity(batches);
    let mut scan_times = Vec::with_capacity(batches);
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..runs {
            std::hint::black_box(db.nearby_stars(boresight, radius));
        }
        tree_times.push(start.elapsed());

        let start = Instant::now();
        for _ in 0..runs {
            std::hint::black_box(brute_force(&db, boresight, radius));
        }
        scan_times.push(start.elapsed());
    }
    tree_times.sort();
    scan_times.sort();
    let tree = tree_times[batches / 2];
    let scan = scan_times[batches / 2];

    // A degenerate tree that visits every node runs at about the scan's speed.
    // A working one is far faster; 3x leaves a wide margin for noise.
    assert!(
        tree * 3 < scan,
        "KD-tree query is not clearly faster than a linear scan at N {n}: median {tree:?} vs {scan:?} per {runs} queries"
    );
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
        star_table.extend([
            ra as f32, dec as f32, x as f32, y as f32, z as f32, mag as f32,
        ]);
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
