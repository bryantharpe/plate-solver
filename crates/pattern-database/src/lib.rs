//! Read-side precomputed sky index.
//!
//! Loads the offline-built pattern database, exposes its properties, builds a KD-tree
//! over the star unit vectors, and provides the key-to-candidates lookup path with
//! the cheap rejection filters used at solve time.

pub mod format;
pub mod kdtree;
pub mod load;
pub mod lookup;
pub mod properties;

pub use format::{CatalogId, PatternDatabase, StarId};
pub use load::{load_from_path, LoadError};
pub use lookup::{Candidate, LookupQuery};
pub use properties::DatabaseProperties;
