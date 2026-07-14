use math_core::{angular_distance, UnitVector};
use nalgebra::{Point3, Vector3};

/// Cached KD-tree over catalog star unit vectors.
pub struct StarKdTree {
    points: Vec<Point3<f64>>,
    indices: Vec<usize>,
}

impl StarKdTree {
    /// Build a KD-tree from the star table unit vectors.
    pub fn from_unit_vectors(vectors: &[UnitVector]) -> Self {
        let mut indexed: Vec<_> = vectors.iter().enumerate().collect();
        Self::build_recursive(&mut indexed);
        let points: Vec<_> = indexed
            .iter()
            .map(|(_, v)| Point3::new(v.x, v.y, v.z))
            .collect();
        let indices: Vec<_> = indexed.iter().map(|(i, _)| *i).collect();
        Self { points, indices }
    }

    fn build_recursive(items: &mut [(usize, UnitVector)]) {
        if items.len() <= 1 {
            return;
        }
        let depth = 0usize;
        Self::partition_by_depth(items, depth);
        let mid = items.len() / 2;
        Self::build_recursive(&mut items[..mid]);
        Self::build_recursive(&mut items[mid + 1..]);
    }

    fn partition_by_depth(items: &mut [(usize, UnitVector)], depth: usize) {
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
        found.sort_by_key(|i| ordered_float(self.points[*i].coords.magnitude()));
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

fn ordered_float(v: f64) -> std::cmp::Ordering {
    if v.is_nan() {
        std::cmp::Ordering::Greater
    } else {
        v.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}
