use crate::properties::DatabaseProperties;
use math_core::UnitVector;

/// Unsigned index type sized to the catalog.
pub type CatalogIndex = u32;

/// One row of the star table: `[RA, Dec, x, y, z, mag]` in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarRow {
    pub ra: f32,
    pub dec: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub mag: f32,
}

impl StarRow {
    pub fn unit_vector(&self) -> UnitVector {
        UnitVector {
            x: self.x as f64,
            y: self.y as f64,
            z: self.z as f64,
        }
    }

    pub fn from_radec_mag(ra: f64, dec: f64, mag: f32) -> Self {
        let v = UnitVector::from_radec(ra, dec);
        Self {
            ra: ra as f32,
            dec: dec as f32,
            x: v.x as f32,
            y: v.y as f32,
            z: v.z as f32,
            mag,
        }
    }
}

/// One pattern row: four indices into the star table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PatternRow(pub [CatalogIndex; 4]);

/// In-memory representation of a loaded pattern database.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternDatabase {
    pub properties: DatabaseProperties,
    pub star_table: Vec<StarRow>,
    pub pattern_catalog: Vec<PatternRow>,
    pub pattern_largest_edge: Vec<f32>,
    pub pattern_key_hashes: Vec<u16>,
    pub star_catalog_ids: Vec<Vec<u8>>,
}

impl PatternDatabase {
    /// Number of stars in the database.
    pub fn num_stars(&self) -> usize {
        self.star_table.len()
    }

    /// Number of rows in the pattern catalog (hash-table slots).
    pub fn catalog_len(&self) -> usize {
        self.pattern_catalog.len()
    }
}
