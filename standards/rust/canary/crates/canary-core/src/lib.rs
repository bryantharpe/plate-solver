#![forbid(unsafe_code)]
//! Numeric core for the verification canary.
//!
//! Nothing here is astronomically load-bearing — the point is to give the
//! Tier-2 gates something real to chew on: exact-value unit tests, `proptest`
//! invariants, doctests, a `criterion` bench, and a differential test against an
//! independently-computed golden fixture. It mirrors the plate-solver domain
//! (unit vectors, the `acos(â·b̂)` angle convention) on purpose.
//!
//! `#![forbid(unsafe_code)]` makes the unsafe census for this crate exactly zero,
//! enforced by the compiler rather than by an external tool.

/// Dot product of two 3-vectors.
#[must_use]
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Return `v` scaled to unit length. A zero-length vector maps to the zero vector.
///
/// # Examples
/// ```
/// use canary_core::{normalize, dot};
/// let u = normalize([0.0, 3.0, 4.0]);
/// assert!((dot(u, u).sqrt() - 1.0).abs() < 1e-12);
/// ```
#[must_use]
pub fn normalize(v: [f64; 3]) -> [f64; 3] {
    let n = dot(v, v).sqrt();
    if n > 0.0 {
        [v[0] / n, v[1] / n, v[2] / n]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Angular separation in radians between the directions of `a` and `b`.
///
/// Computed as `acos(â · b̂)` with the dot product clamped to `[-1, 1]` so that
/// floating-point error near parallel/antiparallel inputs can never feed an
/// out-of-domain value to `acos` (which would yield `NaN`). The result lies in
/// `[0, π]`.
///
/// # Examples
/// ```
/// use canary_core::angular_separation;
/// let right_angle = angular_separation([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
/// assert!((right_angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
/// ```
#[must_use]
pub fn angular_separation(a: [f64; 3], b: [f64; 3]) -> f64 {
    let cos = dot(normalize(a), normalize(b)).clamp(-1.0, 1.0);
    cos.acos()
}

#[cfg(test)]
mod tests {
    use super::{angular_separation, dot, normalize};
    use std::f64::consts::{FRAC_PI_2, PI};

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn normalize_unit_axis() {
        let u = normalize([3.0, 0.0, 0.0]);
        assert!(close(u[0], 1.0, 1e-15));
        assert!(close(u[1], 0.0, 1e-15));
        assert!(close(u[2], 0.0, 1e-15));
    }

    #[test]
    fn normalize_zero_stays_zero() {
        let u = normalize([0.0, 0.0, 0.0]);
        assert!(close(dot(u, u), 0.0, 1e-15));
    }

    #[test]
    fn normalize_yields_unit_length() {
        let u = normalize([1.0, 2.0, 3.0]);
        assert!(close(dot(u, u).sqrt(), 1.0, 1e-12));
    }

    #[test]
    fn sep_right_angle() {
        assert!(close(
            angular_separation([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            FRAC_PI_2,
            1e-12
        ));
    }

    #[test]
    fn sep_identical_is_zero() {
        // Kills any mutation of the upper clamp bound: without clamp(_, 1.0),
        // acos of a dot slightly > 1.0 would not be exactly 0 here.
        assert!(close(
            angular_separation([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
            0.0,
            1e-12
        ));
    }

    #[test]
    fn sep_antipodal_is_pi() {
        // Kills any mutation of the lower clamp bound (-1.0): the dot product is
        // exactly -1.0 here, so raising the lower bound changes the result.
        assert!(close(
            angular_separation([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            PI,
            1e-12
        ));
    }

    #[test]
    fn sep_is_symmetric() {
        let a = [0.2, 0.5, 0.84];
        let b = [0.9, 0.1, 0.4];
        assert!(close(
            angular_separation(a, b),
            angular_separation(b, a),
            1e-12
        ));
    }
}

#[cfg(test)]
mod props {
    use super::{angular_separation, dot, normalize};
    use proptest::prelude::*;
    use std::f64::consts::PI;

    fn comp() -> impl Strategy<Value = f64> {
        -1000.0f64..1000.0
    }
    fn vec3() -> impl Strategy<Value = [f64; 3]> {
        [comp(), comp(), comp()]
    }

    proptest! {
        #[test]
        fn separation_is_finite_and_in_range(a in vec3(), b in vec3()) {
            let s = angular_separation(a, b);
            prop_assert!(s.is_finite());
            prop_assert!((0.0..=PI + 1e-9).contains(&s));
        }

        #[test]
        fn separation_is_symmetric(a in vec3(), b in vec3()) {
            let s1 = angular_separation(a, b);
            let s2 = angular_separation(b, a);
            prop_assert!((s1 - s2).abs() < 1e-9);
        }

        #[test]
        fn normalize_of_nonzero_is_unit(v in vec3()) {
            prop_assume!(dot(v, v).sqrt() > 1e-6);
            let u = normalize(v);
            prop_assert!((dot(u, u).sqrt() - 1.0).abs() < 1e-9);
        }
    }
}
