use serde::{Deserialize, Serialize};

/// Packed properties record stored with a pattern database.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DatabaseProperties {
    /// Pattern mode, e.g. `"edge_ratio"`.
    pub pattern_mode: String,
    /// Number of stars in a pattern (typically 4).
    pub pattern_size: u32,
    /// Quantization bins used for pattern keys.
    pub pattern_bins: u32,
    /// Maximum allowed pattern error.
    pub pattern_max_error: f64,
    /// Maximum field of view in degrees.
    pub max_fov: f64,
    /// Minimum field of view in degrees.
    pub min_fov: f64,
    /// Name of the source star catalog.
    pub star_catalog: String,
    /// Equinox epoch of the catalog.
    pub epoch_equinox: f64,
    /// Proper-motion epoch of the catalog.
    pub epoch_proper_motion: f64,
    /// Number of verification stars expected per FOV.
    pub verification_stars_per_fov: u32,
    /// Legacy alias for `verification_stars_per_fov`.
    #[serde(alias = "catalog_stars_per_fov")]
    pub catalog_stars_per_fov: u32,
    /// Maximum (brightest-limit) magnitude included.
    pub star_max_magnitude: f64,
    /// Legacy alias for `star_max_magnitude`.
    #[serde(alias = "star_min_magnitude")]
    pub star_min_magnitude: f64,
    /// Hash-table type: `"quadratic_probe"` or `"linear_probe"`.
    pub hash_table_type: String,
    /// Number of patterns stored.
    pub num_patterns: u64,
}

impl DatabaseProperties {
    /// Apply legacy fallbacks for older databases that omitted some fields.
    pub fn apply_legacy_fallbacks(&mut self, catalog_rows: usize) {
        if self.num_patterns == 0 && catalog_rows > 0 {
            self.num_patterns = (catalog_rows / 2) as u64;
        }
        if self.min_fov == 0.0 {
            self.min_fov = self.max_fov;
        }
        if self.verification_stars_per_fov == 0 {
            self.verification_stars_per_fov = self.catalog_stars_per_fov;
        }
        if self.star_max_magnitude == 0.0 {
            self.star_max_magnitude = self.star_min_magnitude;
        }
    }
}
