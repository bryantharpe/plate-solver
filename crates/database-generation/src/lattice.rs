//! Fibonacci sphere lattice for uniform field-center distribution.

use math_core::UnitVector;
use std::f64::consts::PI;

/// Golden ratio.
const PHI: f64 = 1.618_033_988_749_895;

/// Golden-angle increment in radians: `2π · (1 - 1/φ)`.
const GOLDEN_ANGLE: f64 = 2.0 * PI * (1.0 - 1.0 / PHI);

/// Generate the `2n + 1` points of a Fibonacci lattice on the unit sphere.
///
/// Points are returned as [`UnitVector`]s in no particular order. The reference
/// implementation yields `(x, y, z)` triples for `i` in `[-n, n]` with
/// `z = i / (n + 0.5)` and `theta = golden_angle * i`.
pub fn fibonacci_sphere_lattice(n: usize) -> Vec<UnitVector> {
    if n == 0 {
        return Vec::new();
    }
    let mut points = Vec::with_capacity(2 * n + 1);
    let denom = n as f64 + 0.5;
    for i in -(n as isize)..=(n as isize) {
        let z = i as f64 / denom;
        let radius = (1.0 - z * z).sqrt();
        let theta = GOLDEN_ANGLE * i as f64;
        points.push(UnitVector {
            x: theta.cos() * radius,
            y: theta.sin() * radius,
            z,
        });
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_n_yields_no_points() {
        assert!(fibonacci_sphere_lattice(0).is_empty());
    }

    #[test]
    fn lattice_has_expected_count() {
        assert_eq!(fibonacci_sphere_lattice(1).len(), 3);
        assert_eq!(fibonacci_sphere_lattice(10).len(), 21);
    }

    #[test]
    fn lattice_points_are_unit_length() {
        for v in fibonacci_sphere_lattice(50) {
            assert!((v.norm() - 1.0).abs() < 1e-12, "norm = {}", v.norm());
        }
    }
}
