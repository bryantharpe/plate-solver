## Why

The solver consumes brightest-first `(y, x)` centroids. Producing them fast and robustly on
real camera frames — with hot pixels, trails, bright interlopers (moon, streetlights), and
spatially varying background — is exactly what cedar-detect does, in Rust, at <10 ms per
megapixel on Raspberry-Pi-4B-class hardware. This change specifies the `ps-detect` crate as a
faithful Rust re-implementation of the cedar-detect pipeline. Grounded in
`reference-solutions/docs/04-star-detection-cedar-detect.md`.

## What Changes

- Introduce the `ps-detect` crate and the `star-detection` capability: 8-bit grayscale input →
  brightest-first `(y, x)` centroids, via noise estimation, an optional binning cascade, a
  one-pass 1-D row gate, hot-pixel classification, blob formation, a 2-D gate, sub-pixel
  centroiding, and brightness ordering.
- Fix the public entry points and parameters (`sigma`, `binning`, `detect_hot_pixels`,
  `normalize_rows`) and the `(0.5,0.5)` pixel-center output convention.
- Establish a **parity** contract: centroids match cedar-detect on the reference test images
  within tolerance (≈±0.1 px, same brightness ranking).

## Capabilities

### New Capabilities

- `star-detection`: the cedar-detect Rust detector — image → brightest-first `(y,x)` centroids,
  with noise estimation, binning, 1-D/2-D gating, hot-pixel rejection, sub-pixel centroiding,
  and brightness ordering.

### Modified Capabilities

(none.)

## Impact

- New crate `ps-detect` depending on `ps-core` (pixel conventions) and the `image` crate (0.25)
  for 8-bit grayscale I/O. Optional pluggable SIMD binner.
- Output feeds `ps-solve` directly and is exposed over `ExtractCentroids` in `ps-grpc`.
- The original tetra3 Python detector (threshold + connected-components + moments, doc 03) is
  **reference-only / a non-goal** for v1.
