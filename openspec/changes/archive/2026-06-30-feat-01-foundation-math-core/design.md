## Context

`math-core` is the `ps-core` crate: the deterministic geometric and statistical kernel every
other crate depends on. It has no I/O, no async, and no platform code. Its single hard
constraint is **numerical parity** with the Python reference (`reference-solutions/`): if the
angle convention, hash packing, SVD sign handling, or false-alarm statistic drift from the
reference, no dependent feature can reproduce reference attitudes/IDs. All behavior is grounded
in `docs/02-coordinate-systems-and-math.md`.

## Goals / Non-Goals

**Goals:**
- One canonical implementation of: vector↔RA/Dec, `2·arcsin(d/2)` angle, pinhole
  project/unproject, single-parameter distortion, Wahba/SVD attitude + RA/Dec/Roll, edge-ratio
  key + 64-bit hash + table index + open-addressing, centroid ordering, FOV refinement,
  binomial false-alarm test, residuals.
- f64 compute throughout; f32 storage parity for DB vectors.
- A parity-test harness comparing against captured reference values within tolerance.

**Non-Goals:**
- No image handling, catalog parsing, DB I/O, solving loop, or networking (those are
  `ps-detect`/`ps-db`/`ps-dbgen`/`ps-solve`/`ps-grpc`).
- No fisheye/multi-parameter distortion (single radial `k` only).
- No `arccos`-based angle path (the `2·arcsin` form is mandatory).

## Decisions

- **Linear algebra / SVD → `nalgebra`.** It provides fixed-size 3×3 matrices and an `SVD` with
  the `U·Vᵀ` factors Wahba needs. Alternatives: `glam` (no SVD), hand-rolled Jacobi 3×3 SVD
  (more code, more risk). nalgebra is the lowest-risk parity choice; revisit only if it pulls
  unacceptable mobile weight.
- **f64 compute, f32 storage.** Match NumPy float64 math for parity; store DB unit vectors as
  f32 to mirror the reference `star_table` dtype and halve DB size.
- **`det(R) < 0` reject, not sign-flip.** A true match never yields a reflection, so cedar's
  cheap reject is exact and avoids a conditional sign flip of `U`'s last column. We adopt the
  reject (the general sign-flip is documented but unused).
- **`_MAGIC_RAND = 2654435761` (`⌊2³²/φ⌋`).** Knuth multiplicative hash for the quadratic-probe
  index; overflow wraps mod 2⁶⁴ intentionally. Linear-probe tables skip the multiply (prime
  size disperses keys).
- **Angle = `2·arcsin(d/2)` everywhere.** Pattern edges, residuals, and FOV math share the one
  helper so tolerances line up; this is a correctness invariant, not a style choice.
- **Pattern key quantization** uses `pattern_bins = round(1/(4·pattern_max_error))`; the `1/4`
  absorbs worst-case error propagation in a normalized ratio.

## Risks / Trade-offs

- [SVD sign / NumPy divergence] → Validate Wahba against captured reference `R` matrices within
  1e-9; rely on `det(R)<0` reject to neutralize reflection ambiguity.
- [Float summation order vs NumPy] → Keep operation order close to the reference; assert parity
  with relative tolerances rather than bit-exactness.
- [Distortion Newton non-convergence at extreme `k`] → bound `maxiter=30`, `tol=1e-6`; document
  that v1 targets realistic small `|k|`.
- [64-bit overflow in `key_hash`] → intended wrap; pin to `u64` arithmetic so behavior matches
  the reference `uint64`.

## Migration Plan

Greenfield crate; no migration. Parity harness consumes captured reference vectors/outputs
committed as fixtures (or generated offline from `reference-solutions/`), kept out of the
runtime path.

## Open Questions

- Whether to expose both quadratic and linear index functions from `ps-core` or let `ps-db`
  own the linear-probe variant (leaning: `ps-core` exposes both index functions; `ps-db` owns
  table sizing). Resolved in `feat-03-pattern-database`.
