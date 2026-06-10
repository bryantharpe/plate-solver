//! Celestial-frame conversions: equatorial `(RA, Dec)` in radians ↔ unit
//! vectors. Reference: doc 02 §1.3 (`x = cosRA·cosDec`, `y = sinRA·cosDec`,
//! `z = sinDec`; inverse `RA = atan2(y, x) mod 2π`, `Dec = asin(z)`).

use nalgebra::Vector3;
use std::f64::consts::TAU;

/// Convert celestial `(ra, dec)` (radians) to an equatorial unit vector.
pub fn radec_to_vector(ra: f64, dec: f64) -> Vector3<f64> {
    let cos_dec = dec.cos();
    Vector3::new(ra.cos() * cos_dec, ra.sin() * cos_dec, dec.sin())
}

/// Convert an equatorial unit vector to `(ra, dec)` (radians), with
/// `ra ∈ [0, 2π)` via `atan2(y, x) mod 2π` and `dec = asin(z)`.
pub fn vector_to_radec(v: &Vector3<f64>) -> (f64, f64) {
    let ra = v.y.atan2(v.x).rem_euclid(TAU);
    let dec = v.z.asin();
    (ra, dec)
}