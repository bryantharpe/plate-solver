//! # ps-core — deterministic math kernel for the lost-in-space plate solver
//!
//! `ps-core` is the geometric/statistical core every other crate depends on.
//! It has no I/O, no async, and no platform code. Its single hard constraint is
//! **numerical parity** with the Python reference (`reference-solutions/`,
//! distilled in `reference-solutions/docs/02-coordinate-systems-and-math.md`).
//!
//! The math primitives (celestial vectors, angular distance, pinhole
//! projection, radial distortion, Wahba/SVD attitude, edge-ratio pattern key +
//! hashing, FOV refinement, false-alarm test, residuals) are implemented in
//! later tasks (MC2–MC7). This module currently only fixes the binding
//! conventions every primitive must honour.
//!
//! ## Binding conventions
//!
//! - **f64 compute, f32 storage.** All math is computed in `f64` to match the
//!   reference's NumPy `float64` path. Database unit vectors are *stored* as
//!   `f32` (the reference `star_table` dtype), which halves DB size; the
//!   f64↔f32 boundary is a parity-relevant detail, never an accuracy shortcut
//!   in compute.
//!
//! - **Pixel coordinates are `(y, x)`.** `y` is the row index increasing
//!   downward, `x` the column index increasing rightward, origin at the
//!   top-left corner. `size = (height, width)`; the image centre is
//!   `[height/2, width/2]`.
//!
//! - **`(0.5, 0.5)` is the centre of the top-left pixel.** The integer pixel
//!   index of a coordinate is `floor(coord)`; centroid computations therefore
//!   carry a `+0.5` offset.
//!
//! - **Camera-frame vectors are `(i, j, k)`:** `i` = boresight / optical axis
//!   (component 0), `j` = image-x / horizontal (1), `k` = image-y / vertical
//!   (2).
//!
//! - **Angles use `2·asin(d/2)`,** never `arccos(u·v)`: the central angle for a
//!   chord distance `d` between unit vectors is `2·asin(d/2)` (inverse
//!   `d = 2·sin(angle/2)`). This form is well-conditioned at small angles and
//!   is used uniformly for pattern edges, residuals, and FOV math.
//!
//! - **Celestial unit vector** for `(RA, Dec)` in radians:
//!   `x = cos(RA)·cos(Dec)`, `y = sin(RA)·cos(Dec)`, `z = sin(Dec)`; inverse
//!   `RA = atan2(y, x) mod 2π`, `Dec = asin(z)`.
//!
//! - **Pattern quantisation:** `pattern_bins = round(1 / (4·pattern_max_error))`.
//!
//! Golden parity fixtures captured from the reference live under
//! `ps-core/tests/fixtures/` (regenerate with
//! `tools/parity/.venv/bin/python tools/parity/capture_core.py`).

pub mod angle;
pub mod attitude;
pub mod celestial;
pub mod distortion;
pub mod pattern;
pub mod projection;
