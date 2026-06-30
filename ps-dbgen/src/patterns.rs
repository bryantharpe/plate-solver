use std::collections::HashSet;

use kiddo::{KdTree, SquaredEuclidean};

/// Number of pattern fields for the full celestial sphere at a given FOV.
/// Reference: ceil(4*pi / fov^2)
pub fn num_fields_for_sky(fov_rad: f64) -> usize {
    (4.0 * std::f64::consts::PI / (fov_rad * fov_rad)).ceil() as usize
}

/// Iterator yielding the 2*n+1 unit vectors of a Fibonacci sphere lattice.
///
/// Reference: fov_util.py fibonacci_sphere_lattice
pub fn fibonacci_sphere_lattice(n: usize) -> impl Iterator<Item = [f32; 3]> {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0; // golden ratio
    let golden_angle_incr = 2.0 * std::f64::consts::PI * (1.0 - 1.0 / phi);

    let n_isize = n as isize;
    (-n_isize..=n_isize).map(move |i| {
        let i_f64 = i as f64;
        let z = i_f64 / (n as f64 + 0.5);
        let radius = (1.0 - z * z).sqrt();
        let theta = golden_angle_incr * i_f64;
        [
            (theta.cos() * radius) as f32,
            (theta.sin() * radius) as f32,
            z as f32,
        ]
    })
}

/// Breadth-first combinations of size 4 from a sorted index sequence.
///
/// Yields [usize; 4] in breadth-first order (brightest bias).
/// This is a pure Rust recursive implementation matching the Python reference:
///   def breadth_first_combinations(sequence, r):
///     if r == 1: for item in sequence: yield (item,)
///     index = r - 1
///     while index < len(sequence):
///         right_most = sequence[index]
///         for prefix in breadth_first_combinations(sequence[:index], r-1):
///             yield prefix + (right_most,)
///         index += 1
pub fn breadth_first_combinations_4(sequence: &[usize]) -> impl Iterator<Item = [usize; 4]> + '_ {
    let mut results = Vec::new();
    bfc_4_recursive(sequence, &mut results);
    results.into_iter()
}

fn bfc_4_recursive(sequence: &[usize], out: &mut Vec<[usize; 4]>) {
    if sequence.len() < 4 {
        return;
    }
    // index starts at r-1 = 3
    let mut index = 3usize;
    while index < sequence.len() {
        let right_most = sequence[index];
        // Generate all combinations of size 3 from sequence[..index]
        let prefix_seq = &sequence[..index];
        for prefix3 in bfc_3_recursive(prefix_seq) {
            out.push([prefix3[0], prefix3[1], prefix3[2], right_most]);
        }
        index += 1;
    }
}

fn bfc_3_recursive(sequence: &[usize]) -> Vec<[usize; 3]> {
    let mut results = Vec::new();
    if sequence.len() < 3 {
        return results;
    }
    let mut index = 2usize;
    while index < sequence.len() {
        let right_most = sequence[index];
        let prefix_seq = &sequence[..index];
        for prefix2 in bfc_2_recursive(prefix_seq) {
            results.push([prefix2[0], prefix2[1], right_most]);
        }
        index += 1;
    }
    results
}

fn bfc_2_recursive(sequence: &[usize]) -> Vec<[usize; 2]> {
    let mut results = Vec::new();
    if sequence.len() < 2 {
        return results;
    }
    let mut index = 1usize;
    while index < sequence.len() {
        let right_most = sequence[index];
        let prefix_seq = &sequence[..index];
        for item in prefix_seq.iter().copied() {
            results.push([item, right_most]);
        }
        index += 1;
    }
    results
}

/// Enumerate all patterns for one FOV scale.
///
/// `vectors`: unit vectors of thinned pattern stars, in brightness (mag-ascending) order.
///            Index into this slice = global star index.
/// `pattern_fov`: FOV in radians for this scale.
/// `lattice_field_oversampling`: typically 100.
/// `patterns_per_lattice_field`: typically 50.
///
/// Returns a deduplicated `HashSet<[usize; 4]>` of sorted 4-tuples (global star indices).
pub fn enumerate_patterns(
    vectors: &[[f32; 3]],
    pattern_fov: f64,
    lattice_field_oversampling: usize,
    patterns_per_lattice_field: usize,
) -> HashSet<[usize; 4]> {
    let fov_angle = pattern_fov / 2.0;
    // chord = 2·sin(angle/2), matching tetra3.py _distance_from_angle
    let fov_chord = (2.0 * (fov_angle / 2.0).sin()) as f32;
    let sq_fov_chord = fov_chord * fov_chord;

    // Build KD-tree over all vectors (item = index as u64)
    let mut tree: KdTree<f32, 3> = KdTree::with_capacity(vectors.len());
    for (idx, &vector) in vectors.iter().enumerate() {
        tree.add(&vector, idx as u64);
    }

    let n = num_fields_for_sky(pattern_fov) * lattice_field_oversampling;

    let mut pattern_list: HashSet<[usize; 4]> = HashSet::new();

    for center in fibonacci_sphere_lattice(n) {
        // Query stars within fov_chord of this lattice field center
        let results: Vec<_> = tree.within_unsorted::<SquaredEuclidean>(&center, sq_fov_chord);

        // Convert to global indices and sort ascending (brightness order)
        let mut field_stars: Vec<usize> = results.iter().map(|nn| nn.item as usize).collect();
        field_stars.sort();

        if field_stars.len() < 4 {
            continue;
        }

        let mut count = 0;
        for pattern in breadth_first_combinations_4(&field_stars) {
            pattern_list.insert(pattern);
            count += 1;
            if count >= patterns_per_lattice_field {
                break;
            }
        }
    }

    pattern_list
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Helper to create a unit vector at (ra, dec) in radians.
    fn vec_at(ra: f64, dec: f64) -> [f32; 3] {
        let cos_dec = dec.cos();
        [
            (ra.cos() * cos_dec) as f32,
            (ra.sin() * cos_dec) as f32,
            dec.sin() as f32,
        ]
    }

    #[test]
    fn test_num_fields_for_sky() {
        // fov = 1 rad -> ceil(4*pi / 1) = ceil(12.566) = 13
        let n = num_fields_for_sky(1.0);
        assert_eq!(n, 13);

        // fov = PI/6 (~0.5236 rad) -> ceil(4*pi / (PI/6)^2) = ceil(4*pi * 36 / pi^2) = ceil(144/pi) ≈ ceil(45.84) = 46
        let n = num_fields_for_sky(PI / 6.0);
        assert_eq!(n, 46);
    }

    #[test]
    fn test_fibonacci_sphere_lattice_count() {
        // fibonacci_sphere_lattice(n) yields 2*n+1 vectors
        for n in [0, 1, 5, 10] {
            let count = fibonacci_sphere_lattice(n).count();
            assert_eq!(count, 2 * n + 1, "n={}", n);
        }
    }

    #[test]
    fn test_fibonacci_sphere_lattice_unit_vectors() {
        for v in fibonacci_sphere_lattice(5) {
            let mag_sq = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64;
            assert!(
                (mag_sq - 1.0).abs() < 1e-5,
                "vector magnitude squared {:.8}, expected ~1.0",
                mag_sq
            );
        }
    }

    #[test]
    fn test_breadth_first_combinations_4_basic() {
        let seq = vec![0, 1, 2, 3, 4];
        let combos: Vec<_> = breadth_first_combinations_4(&seq).collect();
        // C(5,4) = 5
        assert_eq!(combos.len(), 5);

        // Breadth-first order:
        // index starts at 3 (r-1=3), right_most=seq[3]=3
        //   prefixes from seq[:3]=[0,1,2], r=3: only [0,1,2] -> [0,1,2,3]
        // index=4, right_most=seq[4]=4
        //   prefixes from seq[:4]=[0,1,2,3], r=3:
        //     index=2, right_most=2, prefixes from [0,1], r=2: [0,1] -> [0,1,2] -> [0,1,2,4]
        //     index=3, right_most=3, prefixes from [0,1,2], r=2:
        //       index=1, right_most=1, prefixes from [0], r=1: [0] -> [0,1] -> [0,1,3] -> [0,1,3,4]
        //       index=2, right_most=2, prefixes from [0,1], r=1: [0],[1] -> [0,2],[1,2] -> [0,2,3,4],[1,2,3,4]
        let expected = vec![
            [0, 1, 2, 3],
            [0, 1, 2, 4],
            [0, 1, 3, 4],
            [0, 2, 3, 4],
            [1, 2, 3, 4],
        ];
        assert_eq!(combos, expected);
    }

    #[test]
    fn test_breadth_first_combinations_4_exactly_4() {
        let seq = vec![10, 20, 30, 40];
        let combos: Vec<_> = breadth_first_combinations_4(&seq).collect();
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0], [10, 20, 30, 40]);
    }

    #[test]
    fn test_breadth_first_combinations_4_fewer_than_4() {
        let seq = vec![1, 2, 3];
        let combos: Vec<_> = breadth_first_combinations_4(&seq).collect();
        assert_eq!(combos.len(), 0);
    }

    #[test]
    fn test_enumerate_patterns_field_radius_boundary() {
        // Verify the chord formula is 2·sin(fov_angle/2), NOT 2·sin(fov_angle) (2× too large).
        //
        // Strategy: use `lattice_field_oversampling=0` so n=0 → fibonacci_sphere_lattice(0)
        // yields exactly ONE center at (1,0,0) = RA=0,Dec=0. This eliminates ambiguity from
        // other lattice centers that could "bridge" a gap.
        //
        // pattern_fov=0.2 → fov_angle=0.1 → correct chord=2·sin(0.05)≈0.09983
        //                                     wrong chord=2·sin(0.10)≈0.19933
        //
        // Stars 0–4: tight cluster near (1,0,0), chord from center < 0.005 → inside both radii.
        // Star 5: at RA=0.11 rad → chord from (1,0,0) ≈ 2·sin(0.055)≈0.1098
        //   0.1098 > 0.09983 → outside correct field ✓
        //   0.1098 < 0.19933 → inside wrong field  ✓ (test would fail with wrong formula)
        let mut vectors: Vec<[f32; 3]> = (0..5).map(|i| vec_at(0.001 * i as f64, 0.0)).collect();
        vectors.push(vec_at(0.11, 0.0)); // star 5: just outside correct chord

        let pattern_fov = 0.2_f64;
        // oversampling=0 → n=0 → 1 center at (1,0,0)
        let result = enumerate_patterns(&vectors, pattern_fov, 0, 50);

        for pat in &result {
            assert!(
                !pat.contains(&5usize),
                "star 5 is outside the correct field radius and must not appear in patterns, \
                 but appeared in {:?} — chord formula may be 2×sin(fov_angle) instead of 2×sin(fov_angle/2)",
                pat
            );
        }
        // Sanity: the 5 near stars yield C(5,4)=5 combos.
        assert!(
            !result.is_empty(),
            "expected patterns from the 5 near stars"
        );
        assert_eq!(result.len(), 5, "C(5,4)=5 patterns expected");
    }

    #[test]
    fn test_enumerate_patterns_deterministic() {
        // 10 stars spread across a small sky region, same FOV.
        // Run enumerate_patterns twice and assert the two HashSets are equal.
        let vectors: Vec<[f32; 3]> = (0..10)
            .map(|i| {
                let ra = i as f64 * 0.3;
                let dec = 0.1 * (i as f64 % 5.0);
                vec_at(ra, dec)
            })
            .collect();

        // Use a large FOV to ensure patterns are found
        let fov = PI / 3.0; // 60 degrees
        let result1 = enumerate_patterns(&vectors, fov, 10, 50);
        let result2 = enumerate_patterns(&vectors, fov, 10, 50);

        assert!(!result1.is_empty(), "result should be non-empty");
        assert_eq!(result1, result2, "results should be deterministic");
    }

    #[test]
    fn test_enumerate_patterns_dedup() {
        // Use a very dense sky (10 stars tightly clustered, large FOV) so the same
        // 4-tuple would appear in multiple lattice fields.
        // C(10, 4) = 210 possible unique patterns.
        // With dedup, the final count must be <= 210 (and likely much less due to
        // patterns_per_lattice_field limiting per-field collection).
        let vectors: Vec<[f32; 3]> = (0..10)
            .map(|i| vec_at(0.001 * i as f64, 0.001 * i as f64))
            .collect();

        let fov = PI / 4.0; // 45 degrees — large enough to cover all stars
        let result = enumerate_patterns(&vectors, fov, 10, 50);

        let max_possible = 210; // C(10, 4) = 210
        assert!(
            result.len() <= max_possible,
            "dedup should limit to at most C(10,4)={}, got {}",
            max_possible,
            result.len()
        );

        // With patterns_per_lattice_field=50 and multiple lattice fields covering
        // the same stars, we expect dedup to have happened.
        // The result should be a proper subset of all C(10,4) combinations
        // because breadth_first order means the same first 50 combos get inserted
        // repeatedly across fields.
        assert!(
            result.len() < max_possible,
            "dedup should produce fewer than C(10,4) patterns"
        );
    }
}
