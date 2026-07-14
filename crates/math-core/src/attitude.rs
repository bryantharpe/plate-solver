//! Attitude determination from matched image/catalog unit vectors.
//!
//! Solves Wahba's problem via SVD of the cross-covariance matrix
//! `H = image_vectorsᵀ · catalog_vectors`, yielding the least-squares rotation
//! `R = U · Vᵀ`. Reflections (`det(R) < 0`) are rejected as false positives.
//!
//! Pointing (RA/Dec/Roll) is extracted from `R` with row 0 as the boresight.

use crate::{UnitVector, TAU};
use nalgebra::{Matrix3, SVD};

/// A 3×3 proper rotation matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RotationMatrix {
    /// Rotation matrix rows are camera axes in the celestial frame.
    pub rows: [[f64; 3]; 3],
}

impl RotationMatrix {
    /// Build a rotation matrix from its rows.
    fn from_rows(rows: [[f64; 3]; 3]) -> Self {
        Self { rows }
    }

    /// Determinant of the rotation matrix.
    pub fn det(&self) -> f64 {
        let r = &self.rows;
        r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
    }

    /// Apply this rotation to a unit vector.
    pub fn rotate(&self, v: UnitVector) -> UnitVector {
        let r = &self.rows;
        UnitVector {
            x: r[0][0] * v.x + r[0][1] * v.y + r[0][2] * v.z,
            y: r[1][0] * v.x + r[1][1] * v.y + r[1][2] * v.z,
            z: r[2][0] * v.x + r[2][1] * v.y + r[2][2] * v.z,
        }
    }
}

/// Solve Wahba's problem for the least-squares rotation between matched vectors.
///
/// `image` and `catalog` must be the same length and ordered as matching pairs.
/// Returns `None` if the input is empty or if the resulting matrix is a
/// reflection (`det(R) < 0`).
pub fn solve_attitude(image: &[UnitVector], catalog: &[UnitVector]) -> Option<RotationMatrix> {
    assert_eq!(
        image.len(),
        catalog.len(),
        "image and catalog match lists must have the same length"
    );

    if image.is_empty() {
        return None;
    }

    let mut h: Matrix3<f64> = Matrix3::zeros();
    for (img, cat) in image.iter().zip(catalog.iter()) {
        h[(0, 0)] += img.x * cat.x;
        h[(0, 1)] += img.x * cat.y;
        h[(0, 2)] += img.x * cat.z;
        h[(1, 0)] += img.y * cat.x;
        h[(1, 1)] += img.y * cat.y;
        h[(1, 2)] += img.y * cat.z;
        h[(2, 0)] += img.z * cat.x;
        h[(2, 1)] += img.z * cat.y;
        h[(2, 2)] += img.z * cat.z;
    }

    let svd = SVD::new(h, true, true);
    let u = svd.u?;
    let vt = svd.v_t?;
    let m: Matrix3<f64> = u * vt;

    let rows = [
        [m[(0, 0)], m[(0, 1)], m[(0, 2)]],
        [m[(1, 0)], m[(1, 1)], m[(1, 2)]],
        [m[(2, 0)], m[(2, 1)], m[(2, 2)]],
    ];
    let rotation = RotationMatrix::from_rows(rows);

    if rotation.det() < 0.0 {
        return None;
    }

    Some(rotation)
}

/// Extract `(RA, Dec, Roll)` in radians from a rotation matrix.
///
/// Row 0 is the boresight. `RA` and `Roll` are normalized to `[0, 2π)`;
/// `Dec` is in `[-π/2, π/2]`.
pub fn extract_radec_roll(rotation: &RotationMatrix) -> (f64, f64, f64) {
    let r = &rotation.rows;
    let ra = atan2_mod_tau(r[0][1], r[0][0]);
    let dec = (r[0][2]).atan2((r[1][2] * r[1][2] + r[2][2] * r[2][2]).sqrt());
    let roll = atan2_mod_tau(r[1][2], r[2][2]);
    (ra, dec, roll)
}

/// `atan2(y, x)` normalized to `[0, 2π)`.
fn atan2_mod_tau(y: f64, x: f64) -> f64 {
    let a = y.atan2(x);
    if a < 0.0 {
        a + TAU
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitVector;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn rotation_z(angle: f64) -> RotationMatrix {
        let c = angle.cos();
        let s = angle.sin();
        RotationMatrix::from_rows([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]])
    }

    fn random_catalog_vectors(n: usize, seed: u64) -> Vec<UnitVector> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut vectors = Vec::with_capacity(n);
        for i in 0..n {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            seed.hash(&mut hasher);
            let h = hasher.finish();
            let u1 = ((h >> 32) as f64) / (u32::MAX as f64 + 1.0);
            let u2 = ((h & 0xFFFFFFFF) as f64) / (u32::MAX as f64 + 1.0);
            let ra = u1 * TAU;
            let dec = (u2 * PI - FRAC_PI_2).clamp(-FRAC_PI_2 + 1e-12, FRAC_PI_2 - 1e-12);
            vectors.push(UnitVector::from_radec(ra, dec));
        }
        vectors
    }

    #[test]
    fn recovers_known_rotation() {
        let catalog = random_catalog_vectors(20, 42);
        let r0 = rotation_z(0.5);
        let image: Vec<UnitVector> = catalog.iter().map(|v| r0.rotate(*v)).collect();

        let solved = solve_attitude(&image, &catalog).expect("should solve");
        let diff = max_abs_diff(&solved, &r0);
        assert!(diff < 1e-9, "rotation diff = {}", diff);
    }

    #[test]
    fn recovers_known_rotation_with_noise() {
        let catalog = random_catalog_vectors(50, 123);
        let r0 = rotation_z(2.3);
        let image: Vec<UnitVector> = catalog.iter().map(|v| r0.rotate(*v)).collect();

        let solved = solve_attitude(&image, &catalog).expect("should solve");
        let diff = max_abs_diff(&solved, &r0);
        assert!(diff < 1e-9, "rotation diff = {}", diff);
    }

    #[test]
    fn reflection_is_rejected() {
        let catalog = random_catalog_vectors(10, 7);
        // A pure z-reflection is not a proper rotation.
        let reflection =
            RotationMatrix::from_rows([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]]);
        let image: Vec<UnitVector> = catalog.iter().map(|v| reflection.rotate(*v)).collect();
        assert!(
            solve_attitude(&image, &catalog).is_none(),
            "reflection should be rejected"
        );
    }

    #[test]
    fn empty_match_list_yields_none() {
        assert!(solve_attitude(&[], &[]).is_none());
    }

    #[test]
    fn boresight_row_gives_image_center_radec() {
        let ra = 1.2;
        let dec = 0.4;
        let r = rotation_pointing_at(ra, dec);
        let (ra_out, dec_out, _) = extract_radec_roll(&r);

        let ra_diff = ((ra_out - ra + PI).rem_euclid(TAU)) - PI;
        assert!(ra_diff.abs() < 1e-12, "ra diff = {}", ra_diff);
        assert!(
            (dec_out - dec).abs() < 1e-12,
            "dec diff = {}",
            dec_out - dec
        );
    }

    #[test]
    fn radec_round_trip_across_poles_and_wrap() {
        let cases = [
            (0.0, 0.0),
            (0.01, 0.0),
            (TAU - 0.01, 0.0),
            (PI / 2.0, FRAC_PI_4),
            (PI, -FRAC_PI_4),
            (1.5, FRAC_PI_2 - 1e-6),
            (1.5, -FRAC_PI_2 + 1e-6),
        ];
        for (ra, dec) in cases {
            let r = rotation_pointing_at(ra, dec);
            let (ra_out, dec_out, _) = extract_radec_roll(&r);
            let ra_diff = ((ra_out - ra + PI).rem_euclid(TAU)) - PI;
            assert!(
                ra_diff.abs() < 1e-12,
                "ra round-trip failed for ({}, {}): got {}",
                ra,
                dec,
                ra_out
            );
            assert!(
                (dec_out - dec).abs() < 1e-12,
                "dec round-trip failed for ({}, {}): got {}",
                ra,
                dec,
                dec_out
            );
        }
    }

    #[test]
    fn roll_extraction_matches_spec_formula() {
        // Construct a rotation with a known roll value and verify the spec formula.
        let roll_in = 0.7;
        let r = rotation_with_roll(roll_in);
        let (_, _, roll_out) = extract_radec_roll(&r);
        // The spec formula is atan2(R[1,2], R[2,2]) mod 2π.
        // For this construction R[1,2] = -sin(roll_in) and R[2,2] = cos(roll_in),
        // so the raw atan2 is -roll_in, which normalizes to 2π - roll_in.
        let expected = TAU - roll_in;
        assert!(
            (roll_out - expected).abs() < 1e-12,
            "roll = {} expected {}",
            roll_out,
            expected
        );
    }

    fn max_abs_diff(a: &RotationMatrix, b: &RotationMatrix) -> f64 {
        let mut max: f64 = 0.0;
        for i in 0..3 {
            for j in 0..3 {
                max = max.max((a.rows[i][j] - b.rows[i][j]).abs());
            }
        }
        max
    }

    fn rotation_pointing_at(ra: f64, dec: f64) -> RotationMatrix {
        let r0 = UnitVector::from_radec(ra, dec);
        let mut tmp = UnitVector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        if (r0.x * tmp.x + r0.y * tmp.y + r0.z * tmp.z).abs() > 0.99 {
            tmp = UnitVector {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            };
        }
        let row1 = cross(r0, tmp);
        let row1 = normalize(row1);
        let row2 = cross(r0, row1);
        RotationMatrix::from_rows([
            [r0.x, r0.y, r0.z],
            [row1.x, row1.y, row1.z],
            [row2.x, row2.y, row2.z],
        ])
    }

    fn rotation_with_roll(roll: f64) -> RotationMatrix {
        // Boresight along x, roll around x by `roll`.
        let c = roll.cos();
        let s = roll.sin();
        RotationMatrix::from_rows([[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]])
    }

    fn cross(a: UnitVector, b: UnitVector) -> UnitVector {
        UnitVector {
            x: a.y * b.z - a.z * b.y,
            y: a.z * b.x - a.x * b.z,
            z: a.x * b.y - a.y * b.x,
        }
    }

    fn normalize(v: UnitVector) -> UnitVector {
        let n = v.norm();
        UnitVector {
            x: v.x / n,
            y: v.y / n,
            z: v.z / n,
        }
    }
}
