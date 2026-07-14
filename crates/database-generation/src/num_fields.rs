//! Estimate the number of lattice fields needed to tile the sky.

use std::f64::consts::PI;

/// Estimate how many fields of view `fov` (degrees) are needed to cover the
/// whole celestial sphere, accounting for overlap.
///
/// A field covers a solid angle of `2π·(1 - cos(fov/2))` steradians. The sky is
/// `4π` steradians. We add a 25% overlap factor so the estimate is conservative.
pub fn num_fields_for_sky(fov: f64) -> f64 {
    let fov_rad = fov.to_radians();
    let field_solid_angle = 2.0 * PI * (1.0 - (fov_rad / 2.0).cos());
    if field_solid_angle <= 0.0 {
        return 1.0;
    }
    let coverage = 4.0 * PI / field_solid_angle;
    coverage * 1.25
}
