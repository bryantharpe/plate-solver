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
//! Out of scope (owned by the refinement bead): re-fit over all matches, RA/Dec/Roll
//! extraction, FOV/distortion refinement, residuals, and solution assembly.

use crate::status::{MatchResult, SolveContext, VerificationOutcome};
use math_core::{
    attitude::solve_attitude,
    binomial::false_alarm_test,
    fov::{diagonal_fov, estimate_fov},
    pattern::order_pattern_by_centroid_distance,
    PinholeCamera, UnitVector,
};
use pattern_database::Candidate;

/// Verify a single candidate against the current context.
///
/// This is the integration point called by `candidates::verify_candidate`. It runs
/// the full verification pipeline and returns `Accepted` only when the candidate
/// survives attitude solving, projection/match, and the binomial false-alarm test.
pub fn verify_candidate(
    ctx: &SolveContext,
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
pub fn verify_candidate_with_outcome(
    ctx: &SolveContext,
    candidate: &Candidate,
    pattern_indices: [usize; 4],
    image_vectors: &[UnitVector],
    centroids: &[(f64, f64)],
    width: f64,
    height: f64,
) -> Option<VerificationOutcome> {
    // 1. Attitude: pair image/catalog stars by centroid-distance order.
    let (image_pattern, catalog_pattern) =
        ordered_pattern_pair(image_vectors, pattern_indices, candidate, &ctx.db);

    // 2. Coarse FOV from largest-edge ratio (or focal length when no estimate).
    let pattern_centroids: [(f64, f64); 4] = std::array::from_fn(|m| centroids[pattern_indices[m]]);
    let image_largest_edge = largest_pixel_edge(&pattern_centroids);
    let catalog_largest_edge = candidate.edges[5]; // largest of the six sorted edges
    let fov = estimate_fov(
        Some(ctx.fov_initial),
        image_largest_edge,
        catalog_largest_edge,
        width,
    );

    // 3. Solve SVD attitude. Reject reflections.
    let rotation = solve_attitude(&image_pattern, &catalog_pattern)?;

    // 4. Gather nearby catalog stars within diagonal FOV of the boresight.
    let boresight = UnitVector {
        x: rotation.rows[0][0],
        y: rotation.rows[0][1],
        z: rotation.rows[0][2],
    };
    let radius = diagonal_fov(fov, width, height);
    let nearby = ctx.db.nearby_stars(boresight, radius);

    // 5. Derotate and project catalog stars to pixels; keep in-frame.
    let camera = PinholeCamera::new(width, height, fov);
    let catalog_vectors: Vec<UnitVector> = nearby
        .iter()
        .map(|&idx| {
            ctx.db
                .star_vector(pattern_database::StarId(idx))
                .expect("valid star index")
        })
        .collect();
    let derotated: Vec<UnitVector> = catalog_vectors
        .iter()
        .map(|v| rotation.rotate(*v))
        .collect();
    let (projected, in_frame) = camera.project(&derotated);

    // 6. Trim to brightest 2·num_centroids (brightness order is index order).
    let max_catalog_stars = (centroids.len() * 2).min(in_frame.len());

    // 7. Match projected catalog stars to image centroids uniquely within match_radius·width.
    let match_radius_px = ctx.match_radius * width;
    let (matches, matched_centroids, matched_image_vectors, matched_stars, matched_catalog_ids) =
        match_projected_to_centroids(
            &projected,
            &in_frame,
            &nearby,
            &ctx.db,
            max_catalog_stars,
            centroids,
            image_vectors,
            match_radius_px,
        );

    // 8. Binomial false-alarm acceptance.
    let n = centroids.len();
    let nc = max_catalog_stars;
    let m = matches;
    let result = false_alarm_test(
        n,
        nc,
        m,
        ctx.match_radius,
        ctx.match_threshold,
        ctx.props.num_patterns as usize,
    );

    if result.accepted {
        Some(VerificationOutcome {
            accepted: true,
            rotation: Some(rotation),
            matched_centroids,
            matched_image_vectors,
            matched_stars,
            matched_catalog_ids,
            match_probability: Some(result.prob),
            coarse_fov: fov,
        })
    } else {
        None
    }
}

/// Pair the four image and four catalog pattern stars by centroid-distance order.
///
/// Both patterns are ordered independently using the same deterministic rule, so the
/// m-th image star corresponds to the m-th catalog star.
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
