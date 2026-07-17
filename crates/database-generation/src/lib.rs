//! Database generation: catalog parsing, proper-motion propagation, cleanup,
//! magnitude limiting, pattern enumeration, and hash-table construction for the
//! sky index.

pub mod catalog;
pub mod cleanup;
pub mod config;
pub mod fov_ladder;
pub mod lattice;
pub mod num_fields;
pub mod pattern_catalog;
pub mod patterns;
pub mod proper_motion;
pub mod thinning;

pub use catalog::{parse_bsc5, parse_hip, parse_tyc, CatalogEntry, CatalogId, CatalogSource};
pub use cleanup::{clean_and_limit, derive_magnitude_limit};
pub use config::GenerationConfig;
pub use fov_ladder::fov_ladder;
pub use lattice::fibonacci_sphere_lattice;
pub use num_fields::num_fields_for_sky;
pub use pattern_catalog::{build_pattern_catalog, PatternCatalog};
pub use patterns::{enumerate_patterns, Pattern};
pub use thinning::{separation_for_density, thin_by_density};
