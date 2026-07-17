//! Configuration for database generation.

/// Parameters controlling catalog loading and preprocessing.
#[derive(Clone, Debug)]
pub struct GenerationConfig {
    /// Target epoch for proper-motion propagation. `None` disables propagation.
    pub epoch_proper_motion: Option<f64>,
    /// Maximum visual magnitude to retain. `None` triggers auto derivation.
    pub star_max_magnitude: Option<f64>,
    /// Smallest field of view (degrees).
    pub min_fov: f64,
    /// Stars used to verify each field.
    pub verification_stars_per_fov: f64,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            epoch_proper_motion: Some(current_year()),
            star_max_magnitude: None,
            min_fov: 0.5,
            verification_stars_per_fov: 150.0,
        }
    }
}

/// Return the current calendar year as a floating-point value.
pub fn current_year() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    // Approximate: 1970 + seconds / seconds_per_year.
    1970.0 + now / (365.25 * 24.0 * 3600.0)
}
