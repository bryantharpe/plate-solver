#![forbid(unsafe_code)]
//! Plate solver math core.
//!
//! Provides the shared numerical primitives used by the plate solver:
//! celestial coordinate / unit-vector conversion and angular distance via the
//! `2·arcsin(d/2)` chord convention.

use std::f64::consts::TAU;

/// A 3-dimensional unit vector in equatorial coordinates.
///
/// Components are `(x, y, z)` with `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`,
/// and `z = sin(Dec)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVector {
    /// X component (cos RA cos Dec).
    pub x: f64,
    /// Y component (sin RA cos Dec).
    pub y: f64,
    /// Z component (sin Dec).
    pub z: f64,
}

impl UnitVector {
    /// Create a unit vector from right-ascension and declination (radians).
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
    pub fn distance(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Central angle between this vector and another via `2·arcsin(d/2)`.
    #[must_use]
    pub fn angular_distance(self, other: Self) -> f64 {
        angle_from_distance(self.distance(other))
    }
}

/// Compute the central angle (radians) from a chord distance `d`.
///
/// Uses `2·arcsin(d/2)` for small-angle conditioning.
#[must_use]
pub fn angle_from_distance(d: f64) -> f64 {
    2.0 * (0.5 * d).asin()
}

/// Compute the chord distance from a central angle (radians).
///
/// Inverse of [`angle_from_distance`]: `d = 2·sin(angle/2)`.
#[must_use]
pub fn distance_from_angle(angle: f64) -> f64 {
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
    fn forward_conversion_is_unit() {
        let v = UnitVector::from_radec(1.2, 0.4);
        let norm = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        assert_close(norm, 1.0, 1e-12);
        assert_close(v.x, 1.2_f64.cos() * 0.4_f64.cos(), 1e-12);
        assert_close(v.y, 1.2_f64.sin() * 0.4_f64.cos(), 1e-12);
        assert_close(v.z, 0.4_f64.sin(), 1e-12);
    }

    #[test]
    fn round_trip_is_identity() {
        let cases = [
            (0.0, 0.0),
            (PI, FRAC_PI_2 - 0.1),
            (3.0, -0.5),
            (6.0, 0.78),
        ];
        for (ra, dec) in cases {
            let v = UnitVector::from_radec(ra, dec);
            let (ra_out, dec_out) = v.to_radec();
            assert_close(ra_out.rem_euclid(TAU), ra.rem_euclid(TAU), 1e-12);
            assert_close(dec_out, dec, 1e-12);
        }
    }

    #[test]
    fn angle_chord_inversion() {
        let angles = [0.001, 0.1, 1.0, PI - 0.01];
        for &angle in &angles {
            let d = distance_from_angle(angle);
            let recovered = angle_from_distance(d);
            assert_close(recovered, angle, 1e-12);
        }
    }

    #[test]
    fn small_angle_conditioning() {
        // 0.1 arcsecond in radians.
        let angle = 0.1_f64.to_radians() / 3600.0;
        let d = distance_from_angle(angle);
        let via_arcsin = angle_from_distance(d);
        let dot = 1.0 - 0.5 * d * d; // cos(angle) for tiny angles
        let via_arccos = dot.acos();
        assert_close(via_arcsin, angle, 1e-12);
        // arccos loses precision here; the assertion is that arcsin stays good.
        assert!(via_arccos.is_finite());
        assert!((via_arcsin - angle).abs() < (via_arccos - angle).abs());
    }
}
