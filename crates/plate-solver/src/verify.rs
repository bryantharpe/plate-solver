//! Verification stage: attitude, projection/match, and false-alarm acceptance.
//!
//! Implements the authoritative check that turns a catalog pattern candidate into
//! an accepted match. The three substages are deliberately sequential:
//!
//! 1. **Attitude** — pair the 4 image and 4 catalog pattern stars by centroid-distance
//!    order, estimate a coarse FOV from the largest-edge ratio, and solve Wahba's problem
//!    with the existing SVD solver. Reflections (`det(R) < 0`) are rejected.
//! 2. **Projection and match** — gather catalog stars within the diagonal FOV of the
//!    implied boresight (`R` row 0), derotate and project them to pixels, keep only
//!    in-frame stars, trim to the brightest `2·num_centroids`, and match them 1:1 to
//!    image centroids within `match_radius·width`.
//! 3. **False-alarm acceptance** — run the binomial test; accept the first candidate
//!    whose probability is below the Bonferroni-corrected threshold.
//!
//! Out of scope (see `refine`): re-fit over all matches, RA/Dec/Roll
//! extraction, FOV/distortion refinement, residuals, and solution assembly.

use crate::status::{MatchResult, SolveContext, VerificationOutcome};
use math_core::{
    attitude::solve_attitude, binomial::false_alarm_test, fov::diagonal_fov,
    pattern::order_pattern_by_centroid_distance, PinholeCamera, UnitVector,
};
use pattern_database::{Candidate, PatternDatabase};
use std::borrow::Borrow;

/// Verify a single candidate against the current context.
///
/// This is the integration point called by `candidates::verify_candidate`. It runs
/// the full verification pipeline and returns `Accepted` only when the candidate
/// survives attitude solving, projection/match, and the binomial false-alarm test.
pub fn verify_candidate<D: Borrow<PatternDatabase>>(
    ctx: &SolveContext<D>,
    candidate: &Candidate,
    pattern_indices: [usize; 4],
    image_vectors: &[UnitVector],
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
) -> MatchResult {
    match verify_candidate_with_outcome(
        ctx,
        candidate,
        pattern_indices,
        image_vectors,
        centroids,
        width,
        height,
    ) {
        Some(outcome) if outcome.accepted => MatchResult::Accepted,
        _ => MatchResult::Rejected,
    }
}

/// Verify a single candidate and return the full outcome for refinement.
///
/// Returns `None` if the candidate is rejected before the projection/match stage.
pub fn verify_candidate_with_outcome<D: Borrow<PatternDatabase>>(
    ctx: &SolveContext<D>,
    candidate: &Candidate,
    pattern_indices: [usize; 4],
    image_vectors: &[UnitVector],
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
) -> Option<VerificationOutcome> {
    let db = ctx.database();
    // 1. Coarse FOV (radians), exactly as cedar: scale the caller's estimate by the
    //    ratio of catalog to image largest-edge *angles*, or, with no estimate, solve
    //    the focal length from the largest pixel distance.
    let pattern_centroids: [(f64, f64); 4] = pattern_indices.map(|i| centroids[i]);
    let catalog_largest_edge = candidate.edges[5];
    let fov = if ctx.fov_estimate.is_some() {
        let initial: [UnitVector; 4] = pattern_indices.map(|i| image_vectors[i]);
        let image_largest_angle = largest_angle(&initial);
        catalog_largest_edge / image_largest_angle * ctx.fov_initial.to_radians()
    } else {
        let d = largest_pixel_edge(&pattern_centroids);
        let f = d / 2.0 / (catalog_largest_edge / 2.0).tan();
        2.0 * ((width / 2.0) / f).atan()
    };
    if !(fov.is_finite() && fov > 0.0 && fov < std::f64::consts::PI) {
        return None;
    }
    let camera = PinholeCamera::new(width, height, fov);

    // 2. Recompute the pattern vectors at the coarse FOV, pair by centroid order.
    let image_pattern: [UnitVector; 4] = {
        let v = camera.unproject(&pattern_centroids);
        std::array::from_fn(|m| v[m].expect("pinhole unprojection is never zero-length"))
    };
    let catalog_pattern: [UnitVector; 4] = std::array::from_fn(|m| {
        db.star_vector(pattern_database::StarId(candidate.star_indices[m]))
            .expect("candidate star index valid")
    });
    let io = order_pattern_by_centroid_distance(&image_pattern);
    let co = order_pattern_by_centroid_distance(&catalog_pattern);
    let image_ordered: Vec<UnitVector> = (0..4).map(|m| image_pattern[io[m]]).collect();
    let catalog_ordered: Vec<UnitVector> = (0..4).map(|m| catalog_pattern[co[m]]).collect();

    // 3. Attitude; reflections are rejected inside `solve_attitude`.
    let rotation = solve_attitude(&image_ordered, &catalog_ordered)?;

    // 4. Catalog stars within HALF the diagonal FOV of the boresight (cedar passes
    //    fov_diagonal/2 as the radius), brightest first.
    let boresight = UnitVector {
        x: rotation.rows[0][0],
        y: rotation.rows[0][1],
        z: rotation.rows[0][2],
    };
    let nearby = db.nearby_stars(boresight, diagonal_fov(fov, width, height) / 2.0);

    // 5. Derotate and project; keep in-frame; trim to 2 * num_centroids.
    let derotated: Vec<UnitVector> = nearby
        .iter()
        .map(|&idx| {
            rotation.rotate(
                db.star_vector(pattern_database::StarId(idx))
                    .expect("valid star index"),
            )
        })
        .collect();
    let (projected, in_frame) = camera.project(&derotated);
    let n = centroids.len();
    let kept: Vec<usize> = in_frame.iter().copied().take(2 * n).collect();
    let kept_px: Vec<(f64, f64)> = kept.iter().map(|&k| projected[k]).collect();

    // 6. Unique 1:1 matches within match_radius * width, cedar semantics.
    let pairs = find_centroid_matches(centroids, &kept_px, ctx.match_radius * width);

    // 7. Binomial false-alarm test. `ctx.match_threshold` is already divided by
    //    num_patterns, and `false_alarm_test` divides again, so undo it here.
    let num_patterns = ctx.props.num_patterns as usize;
    let raw_threshold = ctx.match_threshold * num_patterns.max(1) as f64;
    let result = false_alarm_test(
        n,
        kept.len(),
        pairs.len(),
        ctx.match_radius,
        raw_threshold,
        num_patterns,
    );
    if !result.accepted {
        return None;
    }

    let matched_image_indices: Vec<usize> = pairs.iter().map(|&(i, _)| i).collect();
    let matched_catalog_ids: Vec<usize> = pairs.iter().map(|&(_, k)| nearby[kept[k]]).collect();
    let matched_centroids: Vec<(f64, f64)> = matched_image_indices
        .iter()
        .map(|&i| centroids[i])
        .collect();
    let matched_image_vectors: Vec<UnitVector> = camera
        .unproject(&matched_centroids)
        .into_iter()
        .map(|v| v.expect("never zero-length"))
        .collect();
    let matched_stars: Vec<UnitVector> = matched_catalog_ids
        .iter()
        .map(|&s| db.star_vector(pattern_database::StarId(s)).expect("valid"))
        .collect();
    Some(VerificationOutcome {
        accepted: true,
        rotation: Some(rotation),
        matched_centroids,
        matched_image_vectors,
        matched_stars,
        matched_catalog_ids,
        match_probability: Some(result.prob),
        coarse_fov: fov,
        matched_image_indices,
    })
}

/// Largest central angle between any two of the four vectors.
fn largest_angle(v: &[UnitVector; 4]) -> f64 {
    let mut best: f64 = 0.0;
    for i in 0..4 {
        for j in (i + 1)..4 {
            best = best.max(math_core::angular_distance(v[i], v[j]));
        }
    }
    best
}

/// Unique 1:1 matches within radius `r` pixels, with cedar's tie-breaking:
/// all pairs closer than `r`; keep, per catalog star, the lowest image index; then,
/// per image star, the lowest catalog index. Returns `(image, catalog)` pairs sorted
/// by image index.
fn find_centroid_matches(
    image: &[(f64, f64)],
    catalog: &[(f64, f64)],
    r: f64,
) -> Vec<(usize, usize)> {
    use std::collections::BTreeMap;
    let r2 = r * r;
    let mut per_catalog: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, a) in image.iter().enumerate() {
        for (c, b) in catalog.iter().enumerate() {
            let dy = a.0 - b.0;
            let dx = a.1 - b.1;
            if dy * dy + dx * dx < r2 {
                per_catalog.entry(c).or_insert(i);
            }
        }
    }
    let mut per_image: BTreeMap<usize, usize> = BTreeMap::new();
    for (c, i) in per_catalog {
        per_image.entry(i).or_insert(c);
    }
    per_image.into_iter().collect()
}

/// Pair the four image and four catalog pattern stars by centroid-distance order.
///
/// Both patterns are ordered independently using the same deterministic rule, so the
/// m-th image star corresponds to the m-th catalog star.
#[allow(dead_code)]
fn ordered_pattern_pair(
    image_vectors: &[UnitVector],
    pattern_indices: [usize; 4],
    candidate: &Candidate,
    db: &pattern_database::PatternDatabase,
) -> (Vec<UnitVector>, Vec<UnitVector>) {
    // Image pattern vectors are referenced by the caller-supplied indices.
    let image_pattern: [UnitVector; 4] = std::array::from_fn(|m| image_vectors[pattern_indices[m]]);
    let image_order = order_pattern_by_centroid_distance(&image_pattern);

    // Catalog pattern vectors from the candidate star indices.
    let catalog_pattern: [UnitVector; 4] = std::array::from_fn(|m| {
        db.star_vector(pattern_database::StarId(candidate.star_indices[m]))
            .expect("candidate star index valid")
    });
    let catalog_order = order_pattern_by_centroid_distance(&catalog_pattern);

    let image_ordered: Vec<UnitVector> = (0..4).map(|m| image_pattern[image_order[m]]).collect();
    let catalog_ordered: Vec<UnitVector> =
        (0..4).map(|m| catalog_pattern[catalog_order[m]]).collect();

    (image_ordered, catalog_ordered)
}

/// Largest Euclidean distance between any pair of centroids in pixels.
fn largest_pixel_edge(centroids: &[(f64, f64)]) -> f64 {
    let mut max_edge = 0.0;
    for i in 0..centroids.len() {
        for j in (i + 1)..centroids.len() {
            let dy = centroids[i].0 - centroids[j].0;
            let dx = centroids[i].1 - centroids[j].1;
            let d = (dy * dy + dx * dx).sqrt();
            if d > max_edge {
                max_edge = d;
            }
        }
    }
    max_edge
}

/// Match projected catalog stars to image centroids uniquely within a pixel radius.
///
/// Returns the number of matched pairs plus the matched centroid, image vector,
/// catalog vector, and catalog-id lists. Each centroid and each catalog star may
/// participate in at most one match. Greedy nearest-first matching is used: for each
/// projected catalog star (in brightness order), find the closest unmatched centroid
/// within `radius_px`; if found, record the match.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn match_projected_to_centroids(
    projected: &[(f64, f64)],
    in_frame: &[usize],
    nearby: &[usize],
    db: &pattern_database::PatternDatabase,
    max_catalog_stars: usize,
    centroids: &[(f64, f64)],
    image_vectors: &[UnitVector],
    radius_px: f64,
) -> (
    usize,
    Vec<(f64, f64)>,
    Vec<UnitVector>,
    Vec<UnitVector>,
    Vec<usize>,
) {
    let radius2 = radius_px * radius_px;
    let mut centroid_matched = vec![false; centroids.len()];
    let mut match_count = 0;

    let mut matched_centroids = Vec::new();
    let mut matched_image_vectors = Vec::new();
    let mut matched_stars = Vec::new();
    let mut matched_catalog_ids = Vec::new();

    for &idx in in_frame.iter().take(max_catalog_stars) {
        let (py, px) = projected[idx];
        let mut best_centroid: Option<usize> = None;
        let mut best_dist2 = f64::INFINITY;

        for (c_idx, &(cy, cx)) in centroids.iter().enumerate() {
            if centroid_matched[c_idx] {
                continue;
            }
            let dy = py - cy;
            let dx = px - cx;
            let dist2 = dy * dy + dx * dx;
            if dist2 <= radius2 && dist2 < best_dist2 {
                best_dist2 = dist2;
                best_centroid = Some(c_idx);
            }
        }

        if let Some(c_idx) = best_centroid {
            centroid_matched[c_idx] = true;
            match_count += 1;
            matched_centroids.push(centroids[c_idx]);
            matched_image_vectors.push(image_vectors[c_idx]);
            matched_stars.push(
                db.star_vector(pattern_database::StarId(nearby[idx]))
                    .expect("matched catalog star index valid"),
            );
            matched_catalog_ids.push(nearby[idx]);
        }
    }

    (
        match_count,
        matched_centroids,
        matched_image_vectors,
        matched_stars,
        matched_catalog_ids,
    )
}
