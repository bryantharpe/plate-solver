//! On-disk pattern database format and loader.
//!
//! Defines the file layout for the precomputed sky index, deserializes it
//! (with legacy fallbacks), builds a cached KD-tree over catalog star unit
//! vectors, and provides a writer for round-trip serialization.

pub mod format;
pub mod io;
pub mod kdtree;
pub mod properties;

pub use format::{CatalogIndex, PatternDatabase, PatternRow, StarRow};
pub use io::{load, write};
pub use properties::DatabaseProperties;
