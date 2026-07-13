#![forbid(unsafe_code)]
//! Shared numerical foundation for plate-solver.
//!
//! Provides celestial coordinate / unit-vector conversions and angular-distance
//! primitives using the `2·arcsin(d/2)` chord convention for small-angle
//! conditioning.

use std::f64::consts::TAU;

/// Convert equatorial coordinates `(RA, Dec)` in radians to a unit vector.
///
/// Uses the convention `x = cos(RA)·cos(Dec)`, `y = sin(RA)·cos(Dec)`,
/// `z = sin(Dec)`.
///
/// # Examples
/// ```
/// use math_core::radec_to_unit;
/// use std::f64::consts::FRAC_PI_2;
///
/// let v = radec_to_unit(FRAC_PI_2, 0.0);
/// assert!((v[0] - 0.0).abs() < 1e-12);
/// assert!((v[1] - 1.0).abs() < 1e-12);
/// assert!((v[2] - 0.0).abs() < 1e-12);
/// ```
#[must_use]
pub fn radec_to_unit(ra: f64, dec: f64) -> [f64; 3] {
    let cos_dec = dec.cos();
    [ra.cos() * cos_dec, ra.sin() * cos_dec, dec.sin()]
}

/// Convert a unit vector back to equatorial coordinates `(RA, Dec)` in radians.
///
/// Returns `RA = atan2(y, x) mod 2π` in `[0, 2π)` and `Dec = arcsin(z)` in
/// `[-π/2, π/2]`. The input is assumed to be a unit vector; non-unit inputs are
/// normalized via `z` only for the declination (the standard `atan2` form).
///
/// # Examples
/// ```
/// use math_core::{radec_to_unit, unit_to_radec};
/// use std::f64::consts::{FRAC_PI_2, PI};
///
/// let (ra, dec) = unit_to_radec(radec_to_unit(PI, FRAC_PI_2));
/// assert!((ra - PI).abs() < 1e-12);
/// assert!((dec - FRAC_PI_2).abs() < 1e-12);
/// ```
#[must_use]
pub fn unit_to_radec(v: [f64; 3]) -> (f64, f64) {
    let [x, y, z] = v;
    let ra = y.atan2(x).rem_euclid(TAU);
    let dec = z.asin();
    (ra, dec)
}

/// Euclidean (chord) distance between two 3-vectors.
#[must_use]
pub fn chord_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Convert a central angle in radians to the corresponding chord distance.
///
/// Inverse of [`angle_from_chord`]: `d = 2·sin(angle/2)`.
///
/// # Examples
/// ```
/// use math_core::{angle_from_chord, chord_from_angle};
/// use std::f64::consts::FRAC_PI_2;
///
/// let d = chord_from_angle(FRAC_PI_2);
/// let angle = angle_from_chord(d);
/// assert!((angle - FRAC_PI_2).abs() < 1e-12);
/// ```
#[must_use]
pub fn chord_from_angle(angle: f64) -> f64 {
    2.0 * (angle / 2.0).sin()
}

/// Convert a chord distance to the central angle it subtends.
///
/// Uses the small-angle-stable form `angle = 2·arcsin(d/2)`. The chord distance
/// is clamped to `[0, 2]` before calling `asin` so that floating-point noise
/// can never yield `NaN`.
///
/// # Examples
/// ```
/// use math_core::{chord_from_angle, angle_from_chord};
/// use std::f64::consts::PI;
///
/// let d = chord_from_angle(PI);
/// assert!((angle_from_chord(d) - PI).abs() < 1e-12);
/// ```
#[must_use]
pub fn angle_from_chord(d: f64) -> f64 {
    let d = d.clamp(0.0, 2.0);
    2.0 * (d / 2.0).asin()
}

/// Angular distance in radians between two unit vectors.
///
/// Computes the chord distance and converts it with `2·arcsin(d/2)`. This is
/// numerically stable for sub-arcsecond separations, unlike `acos(u·v)` near 1.
///
/// # Examples
/// ```
/// use math_core::angular_distance;
/// use std::f64::consts::FRAC_PI_2;
///
/// let angle = angular_distance([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
/// assert!((angle - FRAC_PI_2).abs() < 1e-12);
/// ```
#[must_use]
pub fn angular_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    angle_from_chord(chord_distance(a, b))
}

#[cfg(test)]
mod tests {
    use super::{angular_distance, chord_distance, chord_from_angle, radec_to_unit, unit_to_radec};
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn forward_conversion_is_unit_vector() {
        let ra = 1.23;
        let dec = 0.45;
        let v = radec_to_unit(ra, dec);
        let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        assert!(close(norm, 1.0, 1e-12), "norm was {norm}");

        let expected_x = ra.cos() * dec.cos();
        let expected_y = ra.sin() * dec.cos();
        let expected_z = dec.sin();
        assert!(close(v[0], expected_x, 1e-12));
        assert!(close(v[1], expected_y, 1e-12));
        assert!(close(v[2], expected_z, 1e-12));
    }

    #[test]
    fn round_trip_is_identity() {
        let cases = [
            (0.0, 0.0),
            (PI, FRAC_PI_2),
            (3.0 * FRAC_PI_2, -0.3),
            (TAU - 0.1, 0.75),
            (0.123_456_789, -0.987_654_321),
        ];
        for (ra, dec) in cases {
            let v = radec_to_unit(ra, dec);
            let (ra_back, dec_back) = unit_to_radec(v);
            let ra_diff = (ra_back - ra.rem_euclid(TAU)).abs();
            assert!(close(ra_diff, 0.0, 1e-12), "RA diff {ra_diff}");
            assert!(close(dec_back, dec, 1e-12), "Dec diff {}", dec_back - dec);
        }
    }

    #[test]
    fn angle_chord_inversion() {
        let angles = [0.0, 0.001, 0.1, FRAC_PI_2, PI - 0.01, PI];
        for &angle in &angles {
            let d = chord_from_angle(angle);
            let recovered = angle_from_chord(d);
            assert!(close(recovered, angle, 1e-12), "angle {angle} -> {recovered}");
        }
    }

    #[test]
    fn small_angle_conditioning() {
        // 0.1 arcsecond in radians.
        let arcsec = 0.1_f64.to_radians() / 3600.0;
        let a = [1.0, 0.0, 0.0];
        let b = [arcsec.cos(), arcsec.sin(), 0.0];
        let got = angular_distance(a, b);
        // `acos(dot)` loses precision here; our arcsin form should not.
        assert!(close(got, arcsec, 1e-12), "got {got}, expected {arcsec}");
    }

    #[test]
    fn angular_distance_right_angle() {
        assert!(close(
            angular_distance([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            FRAC_PI_2,
            1e-12
        ));
    }

    #[test]
    fn angular_distance_antipodal() {
        assert!(close(
            angular_distance([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
            PI,
            1e-12
        ));
    }
}
