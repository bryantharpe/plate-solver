//! Core geometric and numerical primitives for the plate-solver rewrite.
//!
//! This crate implements the shared math that every other capability depends on:
//! celestial unit-vector conversion, the `2·arcsin(d/2)` angular-distance convention,
//! and related helpers. All computation is performed in `f64`.

use std::f64::consts::TAU;

/// Convert right ascension and declination (radians) to an equatorial unit vector.
///
/// Uses the convention `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`.
/// The returned vector has unit length (within floating-point round-off).
///
/// # Examples
///
/// ```
/// use ps_core::radec_to_unit_vector;
/// use std::f64::consts::PI;
///
/// let v = radec_to_unit_vector(0.0, 0.0);
/// assert!((v[0] - 1.0).abs() < 1e-12);
/// assert!(v[1].abs() < 1e-12);
/// assert!(v[2].abs() < 1e-12);
///
/// let v = radec_to_unit_vector(PI / 2.0, 0.0);
/// assert!(v[0].abs() < 1e-12);
/// assert!((v[1] - 1.0).abs() < 1e-12);
/// assert!(v[2].abs() < 1e-12);
/// ```
pub fn radec_to_unit_vector(ra: f64, dec: f64) -> [f64; 3] {
    let cos_dec = dec.cos();
    [
        ra.cos() * cos_dec, // x
        ra.sin() * cos_dec, // y
        dec.sin(),          // z
    ]
}

/// Convert an equatorial unit vector back to right ascension and declination (radians).
///
/// Returns `(RA, Dec)` where `RA = atan2(y, x) mod 2π` and `Dec = arcsin(z)`.
/// For a zero vector the result is `(0.0, 0.0)`.
///
/// # Examples
///
/// ```
/// use ps_core::{radec_to_unit_vector, unit_vector_to_radec};
/// use std::f64::consts::PI;
///
/// let (ra, dec) = unit_vector_to_radec([1.0, 0.0, 0.0]);
/// assert!((ra).abs() < 1e-12);
/// assert!(dec.abs() < 1e-12);
///
/// let (ra, dec) = unit_vector_to_radec([0.0, 1.0, 0.0]);
/// assert!((ra - PI / 2.0).abs() < 1e-12);
/// assert!(dec.abs() < 1e-12);
/// ```
pub fn unit_vector_to_radec(v: [f64; 3]) -> (f64, f64) {
    let [x, y, z] = v;
    let ra = y.atan2(x).rem_euclid(TAU);
    let dec = z.clamp(-1.0, 1.0).asin();
    (ra, dec)
}

/// Compute the central angle between two unit vectors from their chord distance.
///
/// Uses the numerically stable form `angle = 2·arcsin(d/2)` in preference to
/// `arccos(u·v)`, which loses precision for small angles.
///
/// # Examples
///
/// ```
/// use ps_core::angle_from_distance;
/// use std::f64::consts::PI;
///
/// let angle = angle_from_distance(2.0_f64.sqrt()); // 90° apart
/// assert!((angle - PI / 2.0).abs() < 1e-12);
/// ```
pub fn angle_from_distance(d: f64) -> f64 {
    2.0 * (d / 2.0).clamp(-1.0, 1.0).asin()
}

/// Compute the chord distance between two unit vectors from their central angle.
///
/// This is the inverse of [`angle_from_distance`]: `d = 2·sin(angle/2)`.
///
/// # Examples
///
/// ```
/// use ps_core::{angle_from_distance, distance_from_angle};
/// use std::f64::consts::PI;
///
/// let d = distance_from_angle(PI / 2.0);
/// assert!((angle_from_distance(d) - PI / 2.0).abs() < 1e-12);
/// ```
pub fn distance_from_angle(angle: f64) -> f64 {
    2.0 * (angle / 2.0).sin()
}

/// Compute the central angle between two unit vectors.
///
/// Convenience wrapper that computes the chord distance internally and then
/// applies the `2·arcsin(d/2)` form.
///
/// # Examples
///
/// ```
/// use ps_core::angle_between_unit_vectors;
/// use std::f64::consts::PI;
///
/// let a = [1.0, 0.0, 0.0];
/// let b = [0.0, 1.0, 0.0];
/// assert!((angle_between_unit_vectors(a, b) - PI / 2.0).abs() < 1e-12);
/// ```
pub fn angle_between_unit_vectors(u: [f64; 3], v: [f64; 3]) -> f64 {
    let dx = u[0] - v[0];
    let dy = u[1] - v[1];
    let dz = u[2] - v[2];
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    angle_from_distance(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    #[test]
    fn forward_conversion_produces_unit_vector() {
        let ra = 1.23;
        let dec = 0.45;
        let v = radec_to_unit_vector(ra, dec);
        let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        assert!((norm - 1.0).abs() < 1e-12);

        let expected = [ra.cos() * dec.cos(), ra.sin() * dec.cos(), dec.sin()];
        for i in 0..3 {
            assert!((v[i] - expected[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn round_trip_is_identity() {
        let cases = [
            (0.0, 0.0),
            (PI / 3.0, FRAC_PI_2 * 0.5),
            (PI, -0.3),
            (3.0 * PI / 2.0, 0.78),
            (TAU - 0.1, -FRAC_PI_2 + 0.01),
        ];
        for (ra, dec) in cases {
            let v = radec_to_unit_vector(ra, dec);
            let (ra_back, dec_back) = unit_vector_to_radec(v);
            let ra_diff = (ra_back - ra).rem_euclid(TAU);
            let ra_diff = if ra_diff > PI { TAU - ra_diff } else { ra_diff };
            assert!(ra_diff < 1e-12, "RA round-trip failed for ({ra}, {dec})");
            assert!(
                (dec_back - dec).abs() < 1e-12,
                "Dec round-trip failed for ({ra}, {dec})"
            );
        }
    }

    #[test]
    fn angle_chord_inversion() {
        let angles = [0.0, 1e-6, 0.01, 0.5, 1.0, PI / 2.0, PI - 0.01, PI];
        for angle in angles {
            let d = distance_from_angle(angle);
            let recovered = angle_from_distance(d);
            assert!((recovered - angle).abs() < 1e-12);
        }
    }

    #[test]
    fn small_angle_conditioning() {
        // Two unit vectors separated by a sub-arcsecond angle.
        let angle = 1.0_f64.to_radians() / 3600.0; // 1 arcsec
        let u = [1.0, 0.0, 0.0];
        let v = [angle.cos(), angle.sin(), 0.0];

        let via_arcsin = angle_between_unit_vectors(u, v);
        let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        let via_arccos = dot.clamp(-1.0, 1.0).acos();

        assert!((via_arcsin - angle).abs() < 1e-12);
        // arccos loses precision near 1; it should deviate measurably for 1 arcsec.
        assert!((via_arccos - angle).abs() > 1e-15);
    }

    #[test]
    fn right_angle_between_axes() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        assert!((angle_between_unit_vectors(a, b) - FRAC_PI_2).abs() < 1e-12);

        let c = [0.0, 0.0, 1.0];
        assert!((angle_between_unit_vectors(a, c) - FRAC_PI_2).abs() < 1e-12);
    }
}
