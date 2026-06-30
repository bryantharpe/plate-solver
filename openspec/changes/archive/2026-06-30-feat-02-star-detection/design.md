## Context

`ps-detect` re-implements `cedar-detect/src/{algorithm,image_funcs,histogram_funcs}.rs` in our
workspace. The reference is already Rust, so this is the lowest-risk port: the design move is to
preserve its structure — one cheap raster pass emits a few hundred/thousand candidates, then
expensive 2-D scrutiny runs only on those. All behavior is grounded in
`docs/04-star-detection-cedar-detect.md`.

## Goals / Non-Goals

**Goals:**
- A `get_stars_from_image(image, noise, sigma, normalize_rows, binning, detect_hot_pixels,
  return_binned)` entry point returning brightest-first `StarDescription`s in input coordinates.
- `estimate_noise_from_image`, `estimate_background_from_image_region`, and
  `summarize_region_of_interest` helpers (the latter for auto-exposure/focus apps).
- <10 ms/Mpx on RPi-4B-class hardware; centroid parity with the reference within ±0.1 px.

**Non-Goals:**
- General source extraction, multi-bit pipelines, or distortion correction (solver's job).
- The tetra3 Python detector (doc 03) — reference-only.
- gRPC transport (that is `feat-06-grpc-service`); this crate is a pure library.

## Decisions

- **Port structure 1:1 from the Rust reference.** Keep the same pass ordering, integer gate
  arithmetic, and box definitions so parity is mechanical rather than re-derived.
- **Integer gate math.** Precompute `sigma_noise_2`/`sigma_noise_3` as integers and keep the
  hot loop in integer comparisons; the `×2`/`×4` factors keep border/neighbor estimates exact
  without division.
- **Pluggable binner.** Expose a `set_binner` hook (default scalar 2×2 average) so a SIMD/NEON
  implementation can be installed on device without touching detection logic.
- **`image` crate (0.25) for I/O**, matching the reference; detection operates on a borrowed
  `GrayImage` row-major buffer (also satisfies the shared-memory zero-copy path used by the
  service).
- **8-bit only.** Higher bit-depth is converted at the boundary; documented as a limitation.

## Risks / Trade-offs

- [Centroid drift vs reference] → Reproduce quadratic-interpolation edge cases (equal-run
  midpoint, edge-peak) exactly; assert ±0.1 px on test_data.
- [Hot-pixel misclassification of single-pixel stars] → Document the "defocus slightly"
  requirement; hot-pixel detection needs the full-res image.
- [Row-normalization sensor specificity] → `normalize_rows` is an IMX296-style fix; keep it
  opt-in and only active when binning.
- [Performance on mobile without SIMD] → binner is pluggable; the scalar default still meets
  budget at typical phone resolutions; revisit in `feat-07-mobile-runtime`.

## Migration Plan

Greenfield crate. Parity fixtures are the reference `test_data` images and cedar-detect's own
centroid outputs (captured offline), kept as test assets.

## Open Questions

- Whether to expose `binning ∈ {1,2,4,8}` fully or cap at `{1,2,4}` to match the service proto's
  documented `{2,4}`. Leaning: support all four in the library; the service validates its own
  subset (resolved in `feat-06-grpc-service`).
