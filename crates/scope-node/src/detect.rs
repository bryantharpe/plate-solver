//! The console's cedar detector, preserving full-frame coordinates and ordering.
use crate::GrayFrame;
use scope_link::proto::Centroid;
use serde::{Deserialize, Serialize};

/// Cedar defaults are independently pinned to plate-solver in integration tests.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetectSettings {
    pub sigma: f64,
    pub noise_estimate: Option<f64>,
    pub binning: usize,
    pub normalize_rows: bool,
    pub detect_hot_pixels: bool,
    pub return_binned: bool,
    pub use_binned_for_star_candidates: bool,
}
impl DetectSettings {
    pub fn cedar() -> Self {
        Self {
            sigma: 8.0,
            noise_estimate: None,
            binning: 2,
            normalize_rows: true,
            detect_hot_pixels: true,
            return_binned: false,
            use_binned_for_star_candidates: true,
        }
    }
}
impl Default for DetectSettings {
    fn default() -> Self {
        Self::cedar()
    }
}

/// The port supplies gray pixels; star-detection supplies brightness order.
pub fn detect(frame: &GrayFrame, settings: &DetectSettings) -> Vec<Centroid> {
    star_detection::detect_stars(
        &frame.pixels,
        frame.width as usize,
        frame.height as usize,
        settings.sigma,
        settings.binning,
        settings.normalize_rows,
        settings.detect_hot_pixels,
    )
    .into_iter()
    .take(200)
    .map(|s| Centroid {
        x: s.x,
        y: s.y,
        brightness: s.brightness,
    })
    .collect()
}
