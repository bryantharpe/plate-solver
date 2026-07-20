//! KD-tree over the star catalog unit vectors.
//!
//! Builds a real KD-tree over the star unit vectors at load time and caches it
//! with the loaded database. Radius queries use the chord metric
//! `2·sin(radius/2)` and return indices in brightness order (i.e. the original
//! index order, because `star_table` is sorted brightest-first).

use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use math_core::{chord_from_angle, UnitVector};

/// KD-tree over star unit vectors.
///
/// Stores points in brightness order so that radius queries can be returned
/// brightest-first by sorting indices by their original order.
#[derive(Debug, Clone)]
pub struct StarKdTree {
    tree: KdTree<f64, usize, [f64; 3]>,
}

impl StarKdTree {
    /// Build a KD-tree from star unit vectors.
    pub fn new(vectors: &[UnitVector]) -> Self {
        // Use the crate's default bucket capacity (16). A bucket capacity of N
        // would leave the tree as a single leaf and degenerate to a linear scan.
        let mut tree = KdTree::new(3);
        for (i, v) in vectors.iter().enumerate() {
            // `add` only fails on dimension mismatch or zero capacity; neither
            // can happen here, so the unwrap is safe.
            tree.add([v.x, v.y, v.z], i).expect("point dimension is 3");
        }
        Self { tree }
    }

    /// Return indices of all stars within `radius` radians of `center`.
    ///
    /// Uses the chord radius `2·sin(radius/2)`. Results are returned
    /// brightest-first by sorting on the original index order.
    pub fn query_ball_point(&self, center: UnitVector, radius: f64) -> Vec<usize> {
        let max_chord = chord_from_angle(radius);
        let max_chord2 = max_chord * max_chord;

        let found = self
            .tree
            .within_unsorted(
                &[center.x, center.y, center.z],
                max_chord2,
                &squared_euclidean,
            )
            .expect("center has dimension 3");

        let mut indices: Vec<usize> = found.into_iter().map(|(_, &idx)| idx).collect();
        // Brightness order is the original index order because star_table is
        // sorted brightest-first.
        indices.sort();
        indices
    }

    /// Return the index of the nearest star to `center`.
    pub fn query_nearest(&self, center: UnitVector) -> Option<usize> {
        self.tree
            .nearest(&[center.x, center.y, center.z], 1, &squared_euclidean)
            .expect("center has dimension 3")
            .first()
            .map(|(_, &idx)| idx)
    }

    /// Return the number of points stored in the tree.
    pub fn size(&self) -> usize {
        self.tree.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_core::UnitVector;

    #[test]
    fn radius_query_finds_nearby_stars() {
        let vectors = vec![
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(1.0_f64.to_radians(), 0.0),
            UnitVector::from_radec(10.0_f64.to_radians(), 0.0),
        ];
        let tree = StarKdTree::new(&vectors);
        let nearby = tree.query_ball_point(UnitVector::from_radec(0.0, 0.0), 2.0_f64.to_radians());
        assert_eq!(nearby, vec![0, 1]);
    }

    #[test]
    fn radius_query_returns_brightest_first() {
        // Indices are brightness order; query should preserve it.
        let vectors = vec![
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(0.5_f64.to_radians(), 0.0),
            UnitVector::from_radec(0.3_f64.to_radians(), 0.0),
        ];
        let tree = StarKdTree::new(&vectors);
        let nearby = tree.query_ball_point(UnitVector::from_radec(0.0, 0.0), 1.0_f64.to_radians());
        assert_eq!(nearby, vec![0, 1, 2]);
    }

    #[test]
    fn nearest_query_returns_closest() {
        let vectors = vec![
            UnitVector::from_radec(0.0, 0.0),
            UnitVector::from_radec(1.0_f64.to_radians(), 0.0),
            UnitVector::from_radec(10.0_f64.to_radians(), 0.0),
        ];
        let tree = StarKdTree::new(&vectors);
        let nearest = tree.query_nearest(UnitVector::from_radec(0.05_f64.to_radians(), 0.0));
        assert_eq!(nearest, Some(0));
    }
}
