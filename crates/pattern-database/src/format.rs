//! In-memory and memory-mapped pattern database format.
//!
//! Defines the arrays carried with a loaded database: star table, pattern catalog,
//! largest-edge table, 16-bit key hashes, and catalog IDs. The layout intentionally
//! mirrors the .npz archive written by the upstream generators.

use math_core::UnitVector;
use crate::kdtree::StarKdTree;

/// Source-catalog identifier for a star.
///
/// BSC and Hipparcos use a single number; Tycho uses a triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogId {
    /// Bright Star Catalog number (uint16 in the upstream format).
    Bsc(u16),
    /// Hipparcos catalog number (uint32).
    Hip(u32),
    /// Tycho catalog number (TYC1, TYC2, TYC3).
    Tyc(u16, u16, u16),
}

/// Lightweight star identifier used inside the pattern catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StarId(pub usize);

/// A loaded pattern database.
///
/// Arrays are stored in the smallest unsigned dtype that can hold the maximum
/// star index, matching the upstream convention. The star table is always
///   with rows  ordered brightest-first.
#[derive(Debug, Clone)]
pub struct PatternDatabase {
    /// Star table:   rows .
    pub star_table: Vec<f32>,
    /// Number of stars in .
    pub num_stars: usize,
    /// Pattern catalog:  star indices in probe order.
    pub pattern_catalog: Vec<[usize; 4]>,
    /// Largest edge angle per occupied slot, in milliradians.
    pub pattern_largest_edge: Vec<f32>,
    /// Low 16 bits of the key hash per occupied slot.
    pub pattern_key_hashes: Vec<u16>,
    /// Source catalog IDs, one per star.
    pub star_catalog_ids: Vec<CatalogId>,
    /// Database properties record.
    pub properties: crate::properties::DatabaseProperties,
    /// KD-tree built over the star unit vectors at load time.
    pub star_kdtree: StarKdTree,
}

impl PatternDatabase {
    /// Return catalog star indices within `radius` radians of `boresight`, brightest-first.
    ///
    /// Uses the KD-tree with chord radius `2·sin(radius/2)`.
    pub fn nearby_stars(&self, boresight: UnitVector, radius: f64) -> Vec<usize> {
        self.star_kdtree.query_ball_point(boresight, radius)
    }

    /// Return the unit vector for a star index.
    pub fn star_vector(&self, index: StarId) -> Option<UnitVector> {
        let i = index.0;
        if i >= self.num_stars {
            return None;
        }
        let base = i * 6;
        Some(UnitVector {
            x: self.star_table[base + 2] as f64,
            y: self.star_table[base + 3] as f64,
            z: self.star_table[base + 4] as f64,
        })
    }

    /// Return  for a star index.
    pub fn star_radec_mag(&self, index: StarId) -> Option<(f64, f64, f64)> {
        let i = index.0;
        if i >= self.num_stars {
            return None;
        }
        let base = i * 6;
        Some((
            self.star_table[base] as f64,
            self.star_table[base + 1] as f64,
            self.star_table[base + 5] as f64,
        ))
    }
}
