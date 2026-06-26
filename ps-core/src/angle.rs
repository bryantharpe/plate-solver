//! Angular distance helpers. The central angle between two unit vectors at
//! chord (Euclidean) distance `d` is `2·asin(d/2)`; the inverse chord distance
//! for a central angle is `2·sin(angle/2)`. This `2·asin(d/2)` form is used
//! everywhere in preference to `arccos(u·v)` for small-angle conditioning.
//! Reference: doc 02 §2 (`_angle_from_distance` / `_distance_from_angle`).

/// Central angle (radians) for a chord distance `d` between unit vectors.
pub fn angle_from_distance(d: f64) -> f64 {
    2.0 * (0.5 * d).asin()
}

/// Chord (Euclidean) distance for a central `angle` (radians) between unit vectors.
pub fn distance_from_angle(angle: f64) -> f64 {
    2.0 * (angle / 2.0).sin()
}
