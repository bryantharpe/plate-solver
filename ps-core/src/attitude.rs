//! Wahba/SVD attitude estimation and RA/Dec/Roll extraction (doc 02 §5 / §5.1,
//! `_find_rotation_matrix`).
//!
//! Given paired image and catalog unit vectors, recovers the 3×3 rotation matrix
//! via SVD of the cross-covariance (H = imageᵀ·catalog). The reference uses
//! numpy's `U, S, Vt = svd(H)` and returns `U @ Vt` with no sign-flip correction.
//!
//! A reflection guard (`det(R) < 0`) rejects false-positive candidates. RA/Dec/Roll
//! are extracted from the recovered rotation using the standard camera-to-equatorial
//! decomposition (row 0 = boresight in celestial frame).

use nalgebra::{Matrix3, Vector3};

/// Recover the rotation matrix `R` that maps catalog (celestial) vectors to image
/// (camera-frame) vectors using the SVD solution to Wahba's problem.
///
/// Builds the 3×3 cross-covariance `H = Σ img_i · cat_iᵀ`, computes the SVD
/// `H = U·S·Vᵀ`, and returns `R = U·Vᵀ`. This matches the reference's
/// `np.dot(U, V)` where numpy's third SVD output is already `Vᵀ`. No
/// determinant sign-flip is applied — if `det(R) < 0` the result is a
/// reflection and should be rejected via [`is_reflection`].
pub fn find_rotation_matrix(
    image_vectors: &[Vector3<f64>],
    catalog_vectors: &[Vector3<f64>],
) -> Matrix3<f64> {
    let mut h = Matrix3::zeros();
    for (img, cat) in image_vectors.iter().zip(catalog_vectors.iter()) {
        h += img * cat.transpose();
    }
    let svd = h.svd(true, true);
    let u = svd.u.expect("svd: U matrix");
    let v_t = svd.v_t.expect("svd: Vᵀ matrix");
    u * v_t
}

/// Reflection guard: a proper rotation has `det = +1`; cedar rejects candidates
/// with `det(R) < 0` as false positives (no sign-flip correction).
pub fn is_reflection(r: &Matrix3<f64>) -> bool {
    r.determinant() < 0.0
}

/// Extract RA, Dec, and Roll (radians) from a rotation matrix.
///
/// Row 0 of `R` is the boresight direction expressed in the celestial frame.
/// The formulas are:
///
/// - `RA  = atan2(R[0,1], R[0,0]) mod 2π`
/// - `Dec = atan2(R[0,2], ‖R[1:3, 2]‖)`
/// - `Roll = atan2(R[1,2], R[2,2]) mod 2π`
///
/// Returns `(ra, dec, roll)` in radians. The reference emits these in degrees
/// (`% 360`); this function returns radians with `ra` and `roll` in `[0, 2π)`.
pub fn extract_radec_roll(r: &Matrix3<f64>) -> (f64, f64, f64) {
    use std::f64::consts::TAU;

    let ra = r[(0, 1)].atan2(r[(0, 0)]).rem_euclid(TAU);
    let dec = r[(0, 2)].atan2((r[(1, 2)].powi(2) + r[(2, 2)].powi(2)).sqrt());
    let roll = r[(1, 2)].atan2(r[(2, 2)]).rem_euclid(TAU);
    (ra, dec, roll)
}