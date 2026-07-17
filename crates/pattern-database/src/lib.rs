//! Read-side sky index for the plate solver.
//!
//! This crate is a placeholder stub for the `pattern-database` capability. The
//! `plate-solver` crate depends on it for the database properties record and
//! candidate-lookup API described in `openspec/specs/pattern-database/spec.md`.
//! Full deserialization, memory-mapped loading, and KD-tree queries are owned by
//! downstream beads.

use math_core::UnitVector;

/// Database properties exposed to the solver.
#[derive(Clone, Debug, PartialEq)]
pub struct DatabaseProperties {
    /// Hash-table quantization bin count.
    pub pattern_bins: u32,
    /// Maximum allowed pattern-key error (ratio units).
    pub pattern_max_error: f64,
    /// Minimum database FOV in degrees.
    pub min_fov: f64,
    /// Maximum database FOV in degrees.
    pub max_fov: f64,
    /// Target verification-star density.
    pub verification_stars_per_fov: f64,
    /// Stored number of patterns (for Bonferroni correction).
    pub num_patterns: usize,
    /// Hash-table type selects the index function.
    pub hash_table_type: HashTableType,
    /// Equinox epoch string.
    pub epoch_equinox: String,
    /// Proper-motion epoch string.
    pub epoch_proper_motion: String,
}

impl DatabaseProperties {
    /// Midpoint of the database FOV range in degrees.
    pub fn fov_midpoint_deg(&self) -> f64 {
        (self.min_fov + self.max_fov) / 2.0
    }
}

/// Hash-table addressing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashTableType {
    /// `key_hash mod table_size` with linear probing.
    LinearProbe,
    /// Quadratic probing using the magic multiplier.
    QuadraticProbe,
}

/// A single catalog star as stored in the database star table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatalogStar {
    /// Right ascension in radians.
    pub ra: f64,
    /// Declination in radians.
    pub dec: f64,
    /// Unit vector on the celestial sphere.
    pub vector: UnitVector,
    /// Apparent magnitude (brightest-first sort key).
    pub mag: f64,
    /// Catalog identifier.
    pub id: CatalogId,
}

/// Catalog identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogId {
    /// Hipparcos catalog number.
    Hip(u32),
    /// Tycho catalog number.
    Tyc(u32),
    /// Bright Star Catalog number.
    Bsc(u32),
    /// Generic identifier.
    Other(u64),
}

/// A loaded pattern database.
#[derive(Clone, Debug)]
pub struct PatternDatabase {
    /// Properties record.
    pub properties: DatabaseProperties,
    /// Star table, brightest-first.
    pub star_table: Vec<CatalogStar>,
    /// Pattern catalog rows: four star-table indices per pattern.
    pub pattern_catalog: Vec<[usize; 4]>,
    /// Largest edge per pattern in radians.
    pub pattern_largest_edge: Vec<f64>,
    /// Low 16 bits of each pattern key hash.
    pub pattern_key_hashes: Vec<u16>,
}

impl PatternDatabase {
    /// Create an empty database for testing and scaffolding.
    pub fn empty() -> Self {
        Self {
            properties: DatabaseProperties {
                pattern_bins: 250,
                pattern_max_error: 0.001,
                min_fov: 10.0,
                max_fov: 20.0,
                verification_stars_per_fov: 150.0,
                num_patterns: 1,
                hash_table_type: HashTableType::LinearProbe,
                epoch_equinox: String::new(),
                epoch_proper_motion: String::new(),
            },
            star_table: Vec::new(),
            pattern_catalog: Vec::new(),
            pattern_largest_edge: Vec::new(),
            pattern_key_hashes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fov_midpoint_computes_average() {
        let props = DatabaseProperties {
            pattern_bins: 250,
            pattern_max_error: 0.001,
            min_fov: 8.0,
            max_fov: 16.0,
            verification_stars_per_fov: 150.0,
            num_patterns: 1,
            hash_table_type: HashTableType::LinearProbe,
            epoch_equinox: String::new(),
            epoch_proper_motion: String::new(),
        };
        assert!((props.fov_midpoint_deg() - 12.0).abs() < 1e-12);
    }
}
