//! Database properties record.
//!
//! Mirrors the packed props_packed structured array written by the upstream
//! generators. Legacy fallbacks are applied at load time, not here.

use serde::{Deserialize, Serialize};

/// Properties carried with every pattern database.
///
/// All angular fields are in degrees; epoch_equinox is a year (e.g. 2000);
/// epoch_proper_motion is the year to which proper motions were propagated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseProperties {
    /// Method used to identify star patterns; always edge_ratio.
    pub pattern_mode: String,
    /// Hash-table probing strategy: quadratic_probe or linear_probe.
    pub hash_table_type: String,
    /// Number of stars in each pattern (always 4 for edge_ratio).
    pub pattern_size: u16,
    /// Number of quantization bins per dimension.
    pub pattern_bins: u16,
    /// Maximum allowed pattern error.
    pub pattern_max_error: f32,
    /// Maximum horizontal FOV the database supports, in degrees.
    pub max_fov: f32,
    /// Minimum horizontal FOV the database supports, in degrees.
    pub min_fov: f32,
    /// Name of the source star catalog (e.g. hip_main).
    pub star_catalog: String,
    /// Epoch of the catalog coordinate system, usually 2000.
    pub epoch_equinox: u16,
    /// Year to which proper motions were propagated.
    pub epoch_proper_motion: f32,
    /// Number of verification stars per FOV-sized region.
    pub verification_stars_per_fov: u16,
    /// Dimmest apparent magnitude retained in the database.
    pub star_max_magnitude: f32,
    /// Number of patterns actually inserted into the hash table.
    pub num_patterns: u32,
}

impl DatabaseProperties {
    /// Return true when the table uses linear probing.
    pub fn linear_probe(&self) -> bool {
        self.hash_table_type == "linear_probe"
    }
}

impl Default for DatabaseProperties {
    fn default() -> Self {
        Self {
            pattern_mode: "edge_ratio".to_string(),
            hash_table_type: "quadratic_probe".to_string(),
            pattern_size: 4,
            pattern_bins: 250,
            pattern_max_error: 0.001,
            max_fov: 30.0,
            min_fov: 30.0,
            star_catalog: "unknown".to_string(),
            epoch_equinox: 2000,
            epoch_proper_motion: 2000.0,
            verification_stars_per_fov: 150,
            star_max_magnitude: 7.0,
            num_patterns: 0,
        }
    }
}
