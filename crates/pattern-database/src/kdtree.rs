//! KD-tree over the star catalog unit vectors.
//!
//! Uses nalgebra for the 3-d point cloud and builds a naive but correct KD-tree
//! that supports nearest and radius (ball) queries. The tree is built once at
//! load time and cached with the database.

use math_core::{chord_from_angle, UnitVector};
use nalgebra::Point3;

/// KD-tree over star unit vectors.
///
/// Stores points in brightness order so that radius queries can be returned
/// brightest-first by sorting indices by their original order.
#[derive(Debug, Clone)]
pub struct StarKdTree {
    points: Vec<Point3<f64>>,
}

impl StarKdTree {
    /// Build a KD-tree from star unit vectors.
    pub fn new(vectors: &[UnitVector]) -> Self {
        let points: Vec<_> = vectors
            .iter()
            .map(|v| Point3::new(v.x, v.y, v.z))
            .collect();
        Self { points }
    }

    /// Return indices of all stars within  radians of .
    ///
    /// Uses the chord radius . Results are returned
    /// brightest-first by sorting on the original index order.
    pub fn query_ball_point(
        &self,
        center: UnitVector,
        radius: f64,
    ) -> Vec<usize> {
        let center = Point3::new(center.x, center.y, center.z);
        let max_chord = chord_from_angle(radius);
        let max_chord2 = max_chord * max_chord;

        let mut found: Vec<usize> = self
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let d2 = (p - center).norm_squared();
                if d2 <= max_chord2 {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        // Brightness order is the original index order because star_table is
        // sorted brightest-first.
        found.sort();
        found
    }

    /// Return the index of the nearest star to .
    pub fn query_nearest(
        &self,
        center: UnitVector,
    ) -> Option<usize> {
        let center = Point3::new(center.x, center.y, center.z);
        self.points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (**a - center).norm_squared();
                let db = (**b - center).norm_squared();
                da.partial_cmp(&db).unwrap()
            })
            .map(|(i, _)| i)
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
