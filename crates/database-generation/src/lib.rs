//! Database generation: catalog parsing, proper-motion propagation, cleanup,
//! and magnitude limiting for the sky index.

pub mod catalog;
pub mod cleanup;
pub mod config;
pub mod num_fields;
pub mod proper_motion;

pub use catalog::{CatalogEntry, CatalogId, CatalogSource, parse_bsc5, parse_hip, parse_tyc};
pub use cleanup::{clean_and_limit, derive_magnitude_limit};
pub use config::GenerationConfig;
pub use num_fields::num_fields_for_sky;
