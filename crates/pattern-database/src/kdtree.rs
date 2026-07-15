use math_core::{angular_distance, UnitVector};
use nalgebra::Point3;

/// Cached KD-tree over catalog star unit vectors.
pub struct StarKdTree {
    points: Vec<Point3<f64>>,
    indices: Vec<usize>,
    magnitudes: Vec<f32>,
}

impl StarKdTree {
    /// Build a KD-tree from the star table unit vectors.
    pub fn from_star_rows(star_table: &[crate::format::StarRow]) -> Self {
        let mut indexed: Vec<_> = star_table.iter().enumerate().collect();
        Self::build_recursive(&mut indexed);
        let points: Vec<_> = indexed
            .iter()
            .map(|(_, row)| Point3::new(row.x as f64, row.y as f64, row.z as f64))
            .collect();
        let indices: Vec<_> = indexed.iter().map(|(i, _)| *i).collect();
        let magnitudes: Vec<_> = indexed.iter().map(|(_, row)| row.mag).collect();
        Self {
            points,
            indices,
            magnitudes,
        }
    }

    fn build_recursive(items: &mut [(usize, &crate::format::StarRow)]) {
        if items.len() <= 1 {
            return;
        }
        let depth = 0usize;
        Self::partition_by_depth(items, depth);
        let mid = items.len() / 2;
        Self::build_recursive(&mut items[..mid]);
        Self::build_recursive(&mut items[mid + 1..]);
    }

    fn partition_by_depth(items: &mut [(usize, &crate::format::StarRow)], depth: usize) {
        let axis = depth % 3;
        items.sort_by(|a, b| {
            let av = match axis {
                0 => a.1.x,
                1 => a.1.y,
                _ => a.1.z,
            };
            let bv = match axis {
                0 => b.1.x,
                1 => b.1.y,
                _ => b.1.z,
            };
            av.partial_cmp(&bv).expect("finite coordinate")
        });
    }

    /// Return star indices within `radius` radians of `boresight`, brightest-first.
    ///
    /// Uses chord radius `2·sin(radius/2)` for the ball query.
    pub fn query_radius(&self, boresight: UnitVector, radius: f64) -> Vec<usize> {
        let chord_radius = 2.0 * (radius / 2.0).sin();
        let mut found = Vec::new();
        self.query_recursive(
            Point3::new(boresight.x, boresight.y, boresight.z),
            chord_radius,
            0,
            self.points.len(),
            0,
            &mut found,
        );
        found.sort_by(|a, b| {
            // star_table is brightest-first (ascending magnitude), so smaller mag = higher rank.
            let ma = self.magnitudes[*a];
            let mb = self.magnitudes[*b];
            ma.partial_cmp(&mb)
                .expect("finite magnitude")
                .then_with(|| self.indices[*a].cmp(&self.indices[*b]))
        });
        found
            .into_iter()
            .map(|tree_idx| self.indices[tree_idx])
            .collect()
    }

    fn query_recursive(
        &self,
        target: Point3<f64>,
        radius: f64,
        start: usize,
        end: usize,
        depth: usize,
        found: &mut Vec<usize>,
    ) {
        if start >= end {
            return;
        }
        let mid = (start + end) / 2;
        let p = self.points[mid];
        let d = (p - target).norm();
        if d <= radius {
            found.push(mid);
        }
        let axis = depth % 3;
        let diff = target.coords[axis] - p.coords[axis];
        if diff < 0.0 {
            self.query_recursive(target, radius, start, mid, depth + 1, found);
            if diff.abs() <= radius {
                self.query_recursive(target, radius, mid + 1, end, depth + 1, found);
            }
        } else {
            self.query_recursive(target, radius, mid + 1, end, depth + 1, found);
            if diff.abs() <= radius {
                self.query_recursive(target, radius, start, mid, depth + 1, found);
            }
        }
    }
}

/// Compute the central angle between two unit vectors.
///
/// Convenience wrapper re-exported for tests.
pub fn angle_between(a: UnitVector, b: UnitVector) -> f64 {
    angular_distance(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::StarRow;
    use std::f64::consts::PI;

    #[test]
    fn radius_query_returns_expected_star_brightest_first() {
        let star_table = vec![
            StarRow::from_radec_mag(0.0, 0.0, 2.0),
            StarRow::from_radec_mag(PI / 4.0, 0.0, 1.0),
            StarRow::from_radec_mag(PI / 2.0, 0.0, 3.0),
        ];
        let tree = StarKdTree::from_star_rows(&star_table);

        // Query around the first star with a generous radius.
        let found = tree.query_radius(star_table[0].unit_vector(), 0.1);
        assert_eq!(found, vec![0]);

        // Query around the second star; it is the brightest (mag 1.0).
        let found = tree.query_radius(star_table[1].unit_vector(), 0.1);
        assert_eq!(found, vec![1]);

        // Large radius should return all stars, ordered brightest-first.
        let found = tree.query_radius(star_table[0].unit_vector(), PI);
        assert_eq!(found, vec![1, 0, 2]);
    }
}
