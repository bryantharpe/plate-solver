## Why

Every downstream feature — star detection, the pattern database, the solver, the gRPC
service — rests on a small, shared set of geometric and statistical primitives. If these are
not defined once, exactly, and to numerical parity with the Python reference (tetra3/cedar),
the rest of the system cannot match the reference outputs. This change specifies that shared
math toolbox (`ps-core`) as the foundation all other crates depend on. Grounded in
`reference-solutions/docs/02-coordinate-systems-and-math.md` (and concepts in `docs/01`).

## What Changes

- Introduce the `ps-core` crate and the `math-core` capability: coordinate conversions,
  angular distance, pinhole projection, lens distortion, Wahba/SVD attitude and RA/Dec/Roll
  extraction, the edge-ratio pattern key and its 64-bit hash + open-addressed table index,
  centroid-distance pattern ordering, the binomial false-alarm test, and residual statistics.
- Fix the binding numerical conventions (the `2·arcsin(d/2)` angle form, `(y,x)` pixel
  convention with `(0.5,0.5)` pixel-center, f64 compute / f32 storage, `_MAGIC_RAND`).
- Establish **numerical parity within tolerance** vs the Python reference as a testable
  correctness contract for this and every dependent capability.

## Capabilities

### New Capabilities

- `math-core`: the shared geometric + statistical primitives used by detection, database,
  solver, and service — vectors/angles, projection, distortion, attitude, pattern key &
  hashing, ordering, false-alarm test, residuals.

### Modified Capabilities

(none — this is the foundation change.)

## Impact

- New crate `ps-core` (workspace root). Dependencies: `nalgebra` (3×3 SVD, fixed-size linear
  algebra). No I/O, no async, no platform code — pure, deterministic functions.
- Depended upon by `ps-detect`, `ps-db`, `ps-dbgen`, `ps-solve`, `ps-grpc`, `ps-mobile`.
- Sets the parity-test harness pattern (compare against captured reference values) reused by
  all later features.
