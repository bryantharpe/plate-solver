#![forbid(unsafe_code)]
//! Plate solver math core.
//!
//! Shared numerical primitives used by every other capability: conversion between
//! celestial `(RA, Dec)` coordinates and equatorial unit vectors, and angular
//! distance via the `2·arcsin(d/2)` chord convention.

use std::f64::consts::TAU;

/// A unit vector in equatorial coordinates.
///
/// Components are `(x, y, z)` with `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`,
/// and `z = sin(Dec)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVector {
    /// X component: `cos(RA) * cos(Dec)`.
    pub x: f64,
    /// Y component: `sin(RA) * cos(Dec)`.
    pub y: f64,
    /// Z component: `sin(Dec)`.
    pub z: f64,
}

impl UnitVector {
    /// Create a unit vector from right ascension and declination (radians).
    #[must_use]
    pub fn from_radec(ra: f64, dec: f64) -> Self {
        let cos_dec = dec.cos();
        Self {
            x: ra.cos() * cos_dec,
            y: ra.sin() * cos_dec,
            z: dec.sin(),
        }
    }

    /// Recover `(RA, Dec)` in radians from this unit vector.
    ///
    /// `RA` is returned in `[0, 2π)`; `Dec` is in `[-π/2, π/2]`.
    #[must_use]
    pub fn to_radec(self) -> (f64, f64) {
        let ra = self.y.atan2(self.x).rem_euclid(TAU);
        let dec = self.z.asin();
        (ra, dec)
    }

    /// Euclidean (chord) distance to another unit vector.
    #[must_use]
    pub fn chord_distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Central angle between this vector and another via `2·arcsin(d/2)`.
    #[must_use]
    pub fn angular_distance(self, other: Self) -> f64 {
        angle_from_chord(self.chord_distance(other))
    }
}

/// Compute the central angle (radians) from a chord distance `d`.
///
/// Uses `2·arcsin(d/2)` for small-angle conditioning.
#[must_use]
pub fn angle_from_chord(d: f64) -> f64 {
    2.0 * (0.5 * d).asin()
}

/// Compute the chord distance from a central angle (radians).
///
/// Inverse of [`angle_from_chord`]: `d = 2·sin(angle/2)`.
#[must_use]
pub fn chord_from_angle(angle: f64) -> f64 {
    2.0 * (0.5 * angle).sin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn assert_close(a: f64, b: f64, eps: f64) {
        assert!(
            (a - b).abs() <= eps,
            "expected {b:.3e}, got {a:.3e} (eps {eps:.3e})"
        );
    }

    #[test]
    fn forward_conversion_produces_unit_vector() {
        let ra = 1.23;
        let dec = 0.45;
        let v = UnitVector::from_radec(ra, dec);
        let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        assert_close(norm, 1.0, 1e-12);
        assert_close(v.x, ra.cos() * dec.cos(), 1e-12);
        assert_close(v.y, ra.sin() * dec.cos(), 1e-12);
        assert_close(v.z, dec.sin(), 1e-12);
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
            let v = UnitVector::from_radec(ra, dec);
            let (ra_back, dec_back) = v.to_radec();
            assert_close(ra_back.rem_euclid(TAU), ra.rem_euclid(TAU), 1e-12);
            assert_close(dec_back, dec, 1e-12);
        }
    }

    #[test]
    fn angle_chord_inversion() {
        let angles = [0.0, 1e-6, 0.01, 0.5, 1.0, FRAC_PI_2, PI - 0.01, PI];
        for angle in angles {
            let d = chord_from_angle(angle);
            let recovered = angle_from_chord(d);
            assert_close(recovered, angle, 1e-12);
        }
    }

    #[test]
    fn small_angle_conditioning() {
        // 0.1 arcsecond in radians.
        let angle = 0.1_f64.to_radians() / 3600.0;
        let u = UnitVector::from_radec(0.0, 0.0);
        let v = UnitVector::from_radec(angle, 0.0);
        let via_arcsin = u.angular_distance(v);
        assert_close(via_arcsin, angle, 1e-12);
    }
}
