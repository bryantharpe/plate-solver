//! Pattern database for the plate solver.
//!
//! # On-disk native format (little-endian)
//!
//! The native format is a single binary file with the following layout:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │ MAGIC: b"PSDB" (4 bytes)                                 │
//! │ VERSION: u32 little-endian (currently 1)                  │
//! │ PROPS_LEN: u32 little-endian (byte length of JSON blob)   │
//! │ PROPS_JSON: UTF-8 JSON (PROPS_LEN bytes)                  │
//! │ -- padding to 8-byte alignment --                         │
//! ├──────────────────────────────────────────────────────────┤
//! │ SECTION: star_table                                       │
//! │   COUNT: u64 le (number of rows, i.e. number of stars)   │
//! │   DATA:  COUNT × 6 × f32 le                              │
//! │          [ra_rad, dec_rad, x, y, z, magnitude]           │
//! ├──────────────────────────────────────────────────────────┤
//! │ SECTION: pattern_catalog                                  │
//! │   COUNT: u64 le (hash table slots)                        │
//! │   ELEM_SIZE: u8 (bytes per star-index element: 1, 2, or 4)│
//! │   DATA:  COUNT × 4 × ELEM_SIZE le                        │
//! │          u8/u16/u32 depending on star table size          │
//! ├──────────────────────────────────────────────────────────┤
//! │ SECTION: largest_edge                                     │
//! │   COUNT: u64 le                                           │
//! │   DATA:  COUNT × f16 le (largest edge in milliradians)   │
//! ├──────────────────────────────────────────────────────────┤
//! │ SECTION: key_hashes                                       │
//! │   COUNT: u64 le                                           │
//! │   DATA:  COUNT × u16 le (low 16 bits of pattern key hash) │
//! ├──────────────────────────────────────────────────────────┤
//! │ SECTION: star_catalog_IDs                                 │
//! │   PRESENT: u8 (0 = absent, 1 = present)                  │
//! │   if PRESENT:                                             │
//! │     ELEM_SIZE: u8 (2 or 4)                               │
//! │     COUNT: u64 le                                         │
//! │     DATA:  COUNT × ELEM_SIZE le (u16 or u32 IDs)         │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! The JSON properties blob contains a serialized [`DatabaseProperties`] struct.
//! All multi-byte values are little-endian. The file uses no compression.
//!
//! # Array semantics
//!
//! - `star_table`: Nx6 f32. Columns: [ra_rad, dec_rad, x, y, z, magnitude].
//!   Rows are sorted brightest-first (ascending magnitude).
//! - `pattern_catalog`: hash table. Each row is 4 star-table indices (0-based).
//!   An empty slot has all indices equal to `u8::MAX`/`u16::MAX`/`u32::MAX`.
//! - `largest_edge`: f16 per hash-table slot. Largest pattern edge in milliradians
//!   (`L * 1000` where L is in radians). Zero for empty slots.
//! - `key_hashes`: u16 per slot. Low 16 bits of the pattern key hash (pre-filter).
//!   Zero for empty slots.
//! - `star_catalog_IDs`: optional u16 or u32 per star giving the source catalog ID.

use half::f16;

pub mod importer;
pub mod layout;
pub mod loader;

/// Properties stored in the database header.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseProperties {
    /// Pattern matching mode. Always `"edge_ratio"`.
    pub pattern_mode: String,
    /// Hash table probe strategy. `"quadratic_probe"` or `"linear_probe"`.
    pub hash_table_type: String,
    /// Stars per pattern. Always 4.
    pub pattern_size: u16,
    /// Quantisation bins per edge-ratio dimension.
    pub pattern_bins: u16,
    /// Maximum allowed pattern key error (dimensionless ratio).
    pub pattern_max_error: f32,
    /// Maximum field of view in degrees.
    pub max_fov: f32,
    /// Minimum field of view in degrees.
    pub min_fov: f32,
    /// Source star catalog name (e.g. `"hip_main"`, `"tyc_main"`, `"bsc5"`).
    pub star_catalog: String,
    /// Equinox year (e.g. 2000).
    pub epoch_equinox: u16,
    /// Year proper motions were propagated to.
    pub epoch_proper_motion: f32,
    /// Lattice-field oversampling factor used during generation.
    pub lattice_field_oversampling: u16,
    /// Patterns generated per lattice field.
    pub patterns_per_lattice_field: u16,
    /// Stars retained per FOV during solve verification.
    pub verification_stars_per_fov: u16,
    /// Faintest star magnitude included.
    pub star_max_magnitude: f32,
    /// Whether patterns are sorted by centroid distance.
    pub presort_patterns: bool,
    /// Number of valid (non-empty) patterns in the catalog.
    pub num_patterns: u32,
}

/// Element type used for star-table indices in the pattern catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexElemSize {
    U8 = 1,
    U16 = 2,
    U32 = 4,
}

/// In-memory representation of the pattern database.
pub struct Database {
    pub properties: DatabaseProperties,
    /// Nx6 f32: [ra_rad, dec_rad, x, y, z, magnitude], brightest-first.
    pub star_table: Vec<[f32; 6]>,
    /// Hash table. Each row holds 4 star-table indices. Length = num hash slots.
    pub pattern_catalog_u8: Option<Vec<[u8; 4]>>,
    pub pattern_catalog_u16: Option<Vec<[u16; 4]>>,
    pub pattern_catalog_u32: Option<Vec<[u32; 4]>>,
    /// f16 per slot: largest pattern edge in milliradians.
    pub largest_edge: Vec<f16>,
    /// u16 per slot: low 16 bits of pattern key hash.
    pub key_hashes: Vec<u16>,
    /// Optional per-star catalog IDs.
    pub star_catalog_ids_u16: Option<Vec<u16>>,
    pub star_catalog_ids_u32: Option<Vec<u32>>,
    /// KD-tree index built from the star unit vectors at load time.
    /// Populated by `build_kd_tree()`.
    #[cfg(feature = "kd-tree")]
    pub star_kd_tree: Option<kiddo::KdTree<f32, 3>>,
}

impl Database {
    /// Create a minimal empty database (used in tests and as a builder starting point).
    pub fn empty(properties: DatabaseProperties) -> Self {
        Database {
            properties,
            star_table: Vec::new(),
            pattern_catalog_u8: None,
            pattern_catalog_u16: None,
            pattern_catalog_u32: None,
            largest_edge: Vec::new(),
            key_hashes: Vec::new(),
            star_catalog_ids_u16: None,
            star_catalog_ids_u32: None,
            #[cfg(feature = "kd-tree")]
            star_kd_tree: None,
        }
    }

    /// Return the number of hash-table slots.
    pub fn num_slots(&self) -> usize {
        self.largest_edge.len()
    }

    /// Return the number of stars.
    pub fn num_stars(&self) -> usize {
        self.star_table.len()
    }
}

impl DatabaseProperties {
    /// Resolve legacy-fallback aliases: some older databases use
    /// `anchor_stars_per_fov` / `pattern_stars_per_fov` for what is now
    /// `lattice_field_oversampling`. The JSON loader calls this after deserialisation.
    pub fn apply_legacy_fallbacks(
        pattern_mode: Option<String>,
        hash_table_type: Option<String>,
        pattern_size: Option<u16>,
        pattern_bins: Option<u16>,
        pattern_max_error: Option<f32>,
        max_fov: Option<f32>,
        min_fov: Option<f32>,
        star_catalog: Option<String>,
        epoch_equinox: Option<u16>,
        epoch_proper_motion: Option<f32>,
        lattice_field_oversampling: Option<u16>,
        patterns_per_lattice_field: Option<u16>,
        verification_stars_per_fov: Option<u16>,
        star_max_magnitude: Option<f32>,
        presort_patterns: Option<bool>,
        num_patterns: Option<u32>,
    ) -> Self {
        DatabaseProperties {
            pattern_mode: pattern_mode.unwrap_or_else(|| "edge_ratio".into()),
            hash_table_type: hash_table_type.unwrap_or_else(|| "quadratic_probe".into()),
            pattern_size: pattern_size.unwrap_or(4),
            pattern_bins: pattern_bins.unwrap_or(250),
            pattern_max_error: pattern_max_error.unwrap_or(0.001),
            max_fov: max_fov.unwrap_or(30.0),
            min_fov: min_fov.unwrap_or(10.0),
            star_catalog: star_catalog.unwrap_or_else(|| "hip_main".into()),
            epoch_equinox: epoch_equinox.unwrap_or(2000),
            epoch_proper_motion: epoch_proper_motion.unwrap_or(2015.5),
            lattice_field_oversampling: lattice_field_oversampling.unwrap_or(100),
            patterns_per_lattice_field: patterns_per_lattice_field.unwrap_or(50),
            verification_stars_per_fov: verification_stars_per_fov.unwrap_or(30),
            star_max_magnitude: star_max_magnitude.unwrap_or(7.0),
            presort_patterns: presort_patterns.unwrap_or(true),
            num_patterns: num_patterns.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_props() -> DatabaseProperties {
        DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        )
    }

    #[test]
    fn scaffold_empty_database() {
        let props = default_props();
        let db = Database::empty(props);
        assert_eq!(db.num_stars(), 0);
        assert_eq!(db.num_slots(), 0);
        #[cfg(feature = "kd-tree")]
        assert!(db.star_kd_tree.is_none());
    }

    #[test]
    fn properties_defaults() {
        let props = default_props();
        assert_eq!(props.pattern_mode, "edge_ratio");
        assert_eq!(props.hash_table_type, "quadratic_probe");
        assert_eq!(props.pattern_size, 4);
        assert_eq!(props.pattern_bins, 250);
        assert_eq!(props.epoch_equinox, 2000);
    }

    #[test]
    fn properties_serde_roundtrip() {
        let props = default_props();
        let json = serde_json::to_string(&props).unwrap();
        let props2: DatabaseProperties = serde_json::from_str(&json).unwrap();
        assert_eq!(props.pattern_bins, props2.pattern_bins);
        assert_eq!(props.max_fov, props2.max_fov);
        assert_eq!(props.num_patterns, props2.num_patterns);
    }
}
