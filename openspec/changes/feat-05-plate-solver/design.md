## Context

`ps-solve` is the engine that turns centroids + an approximate FOV into an attitude. It composes
`ps-core` (vectors, attitude, key/hash, false-alarm), `ps-db` (candidate lookup, nearby stars),
and optionally `ps-detect` (for `solve_from_image`). The algorithm is the two-stage geometric
hash match: cheap candidate generation (Stage A), authoritative attitude-based verification
(Stage B). We implement the **cedar** variant throughout (`docs/06`, `docs/08`).

## Goals / Non-Goals

**Goals:**
- `solve_from_centroids` with cedar behavior: cluster-busting, breadth-first image-pattern
  search, nearest-first candidate keys, pre-filters, SVD attitude with `det(R)<0` reject,
  diagonal-FOV projection + unique matching, binomial acceptance, full refinement, status codes.
- Bounded latency via `solve_timeout` (default 5000 ms) and cooperative cancellation.
- Parity with cedar on reference images (RA/Dec arcsec, identical matched IDs).

**Non-Goals:**
- tetra3's `pattern_checking_stars` C(8,4) search and distortion **range** search — reference
  only; v1 does breadth-first and scalar/None distortion.
- Tracking mode (solving with a prior attitude) — lost-in-space only.
- Detection and DB generation (other crates).

## Decisions

- **Cedar path throughout.** Breadth-first over all cluster-busted centroids (timeout-bounded)
  is more thorough than "8 brightest only"; nearest-first key ordering finds the answer sooner;
  `det(R)<0` reject is a cheap exact false-positive filter; the `2×` nearby-star fudge factor
  raises match counts. These are the differences that make cedar robust (doc 08 §3).
- **First-acceptable-solution, not best-over-all.** The binomial false-alarm test is the
  acceptance gate; the first pattern that passes wins (matching the reference and bounding work).
- **Statistical acceptance, not fixed count.** The binomial test (with the `−2` DoF and
  `/num_patterns` Bonferroni correction) adapts the required match count to field density — the
  single most important robustness property; never replace it with a fixed threshold.
- **Cooperative cancellation + timeout** are first-class (operational need for an embedded star
  tracker); both surface as status codes rather than errors.
- **Default parameters** follow cedar (doc 08 §2): `match_radius=0.01`, `match_threshold=1e-5`,
  `match_max_error=0.002` (clamped ≥ DB `pattern_max_error`), `solve_timeout=5000`.

## Risks / Trade-offs

- [Breadth-first blow-up on dense/foreign fields] → `solve_timeout` + nearest-first ordering +
  cluster-busting bound the search; return `TIMEOUT` rather than spin.
- [SVD/attitude parity] → reuse `ps-core` Wahba; validate `R`, RA/Dec/Roll against captured
  reference solves within tolerance; rely on `det(R)<0` reject for reflection ambiguity.
- [Match-radius unit confusion] → `match_radius` is a fraction of width; multiply by `width` at
  match time, as in the reference.
- [Distortion estimation conditioning] → least-squares `(f,k)` solve only when distortion is not
  a fixed scalar; document the linear system from `ps-core` §8.1.

## Migration Plan

Greenfield. Parity harness: run captured reference centroids + FOV through `solve_from_centroids`
and assert RA/Dec within arcsec and identical matched catalog IDs vs cedar's recorded solution.

## Open Questions

- Whether to expose a continuous/streaming solve for video (post-v1). Leaning: out of scope;
  lost-in-space single-shot only for v1 (consistent with PRD non-goals).
