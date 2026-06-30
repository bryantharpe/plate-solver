## Why

This is the product's core: given centroids and an approximate FOV, find where the camera
points — RA, Dec, Roll, refined FOV and distortion — with no prior attitude. It ties together
`ps-core` (math), `ps-db` (lookup + nearby stars), and `ps-detect` (centroids) into the two-stage
geometric-hash match: cheap candidate generation, then authoritative attitude-based verification
with a binomial false-alarm test. This change specifies `ps-solve`. Grounded in
`reference-solutions/docs/06-plate-solving.md` (+ `docs/02` §8–10, `docs/08` defaults).

## What Changes

- Introduce the `ps-solve` crate and the `plate-solver` capability: `solve_from_centroids` (the
  engine) and a `solve_from_image` wrapper — preparation, image-pattern iteration, candidate-key
  generation, verification, refinement, outputs, and status codes — following the **cedar** path
  (breadth-first search, timeout/cancel, cluster-bust, `det(R)<0` reject, nearest-first keys).
- Establish a **parity** contract: reference images solve to RA/Dec within a few arcsec of cedar
  with identical matched catalog IDs.

## Capabilities

### New Capabilities

- `plate-solver`: lost-in-space identification + attitude recovery — prep, candidate generation,
  verification (SVD attitude + binomial false-alarm), refinement (attitude/FOV/distortion),
  outputs, and status codes.

### Modified Capabilities

(none.)

## Impact

- New crate `ps-solve` depending on `ps-core`, `ps-db`, and (for `solve_from_image`) `ps-detect`.
- The engine behind `ps-grpc`'s `SolveFromCentroids` / `SolveFromImage`.
- tetra3's `pattern_checking_stars` (C(8,4) of the 8 brightest) and distortion-range search are
  reference-only; v1 implements the cedar breadth-first + scalar/None distortion path.
