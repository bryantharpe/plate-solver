pub mod bsc5;
pub mod hip;
pub mod tyc;

/// A star record with position (radians), magnitude, and source ID.
#[derive(Debug, Clone)]
pub struct StarRecord {
    pub ra: f64,  // radians
    pub dec: f64, // radians
    pub mag: f64,
    pub cat_id: CatalogId,
}

/// Catalog-specific identifier type.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogId {
    Bsc(u16),
    Hip(u32),
    Tyc([u16; 3]),
}

/// Parameters for catalog parsing.
#[derive(Debug, Clone)]
pub struct ParseParams {
    /// Epoch for proper-motion propagation (e.g. 2000.0)
    pub epoch_proper_motion: f64,
}

impl Default for ParseParams {
    fn default() -> Self {
        Self {
            epoch_proper_motion: 2000.0,
        }
    }
}
