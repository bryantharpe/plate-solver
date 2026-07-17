//! Solve inputs and defaults.
//!
//! Defines the public entry-point types: detection parameters, solve options,
//! and the wrapper that turns an image into centroids before solving.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::CancellationToken;

/// Detection parameters passed through to `star_detection::detect_stars`.
///
/// All fields are optional so callers can override only the values they care
/// about. Unset fields use the upstream defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetectParams {
    /// Detection threshold in units of estimated RMS noise.
    pub sigma: Option<f64>,
    /// Explicit noise estimate; when `None`, noise is estimated from the image.
    pub noise_estimate: Option<f64>,
    /// Binning factor (1, 2, 4, or 8).
    pub binning: Option<usize>,
    /// Shift each row's dark level to a bias of 2.0 before binning.
    pub normalize_rows: Option<bool>,
    /// Reject isolated single-pixel spikes.
    pub detect_hot_pixels: Option<bool>,
    /// Return the binned detection image alongside stars.
    pub return_binned: Option<bool>,
    /// Use the binned image for star candidate generation.
    pub use_binned_for_star_candidates: Option<bool>,
}

impl DetectParams {
    /// Default detection parameters matching the upstream cedar-detect convention.
    pub fn default_cedar() -> Self {
        Self {
            sigma: Some(8.0),
            noise_estimate: None,
            binning: Some(1),
            normalize_rows: Some(false),
            detect_hot_pixels: Some(true),
            return_binned: Some(false),
            use_binned_for_star_candidates: Some(false),
        }
    }

    /// Resolve effective values, using upstream defaults for unset fields.
    pub(crate) fn resolve(&self) -> ResolvedDetectParams {
        ResolvedDetectParams {
            sigma: self.sigma.unwrap_or(8.0),
            noise_estimate: self.noise_estimate,
            binning: self.binning.unwrap_or(1),
            normalize_rows: self.normalize_rows.unwrap_or(false),
            detect_hot_pixels: self.detect_hot_pixels.unwrap_or(true),
            return_binned: self.return_binned.unwrap_or(false),
            use_binned_for_star_candidates: self.use_binned_for_star_candidates.unwrap_or(false),
        }
    }
}

impl Default for DetectParams {
    fn default() -> Self {
        Self::default_cedar()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedDetectParams {
    pub sigma: f64,
    pub noise_estimate: Option<f64>,
    pub binning: usize,
    pub normalize_rows: bool,
    pub detect_hot_pixels: bool,
    pub return_binned: bool,
    pub use_binned_for_star_candidates: bool,
}

/// Options controlling the solve.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    /// Optional initial FOV estimate in degrees.
    pub fov_estimate: Option<f64>,
    /// Maximum allowed FOV error in degrees.
    pub fov_max_error: f64,
    /// Matching radius as a fraction of image width.
    pub match_radius: f64,
    /// Base acceptance threshold for the false-alarm test.
    pub match_threshold: f64,
    /// Maximum allowed pattern error.
    pub match_max_error: f64,
    /// Solve timeout in milliseconds.
    pub solve_timeout_ms: u64,
    /// Optional scalar distortion coefficient.
    pub distortion: Option<f64>,
    /// Cancellation token.
    pub cancel: Option<CancellationToken>,
}

impl SolveOptions {
    /// Default solve options.
    pub fn default_cedar() -> Self {
        Self {
            fov_estimate: None,
            fov_max_error: 10.0,
            match_radius: 0.01,
            match_threshold: 1e-5,
            match_max_error: 0.002,
            solve_timeout_ms: 5000,
            distortion: Some(0.0),
            cancel: None,
        }
    }
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self::default_cedar()
    }
}
