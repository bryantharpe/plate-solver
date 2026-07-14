//! Proper-motion propagation from the catalog epoch to the target epoch.

use crate::catalog::CatalogEntry;

/// Origin epoch for Hipparcos/Tycho proper motions (J1991.25).
pub const HIP_TYC_PM_ORIGIN: f64 = 1991.25;

/// Propagate every entry in place from `pm_origin` to `target_epoch`.
///
/// For each star:
///   Δt = target_epoch - pm_origin
///   μ_α = pmRA / cosδ   (recovering the true RA rate from the catalog's
///         cosδ-weighted value)
///   RA  += μ_α · Δt
///   Dec += μ_δ · Δt
///
/// Propagation is skipped when `cosδ ≤ 0.05` (|Dec| ≳ 87°) to avoid blow-up,
/// and when either proper motion is missing.
///
/// Proper motions are in **milliarcseconds per year**; angles are in **radians**,
/// so the result is converted with `1 mas = π / (180·3600·1000)` radians.
pub fn propagate(entries: &mut [CatalogEntry], pm_origin: f64, target_epoch: f64) {
    let dt = target_epoch - pm_origin;
    let mas_to_rad = std::f64::consts::PI / (180.0 * 3600.0 * 1000.0);

    for entry in entries {
        let (Some(pm_ra), Some(pm_dec)) = (entry.pm_ra, entry.pm_dec) else {
            continue;
        };

        let cos_dec = entry.dec.cos();
        if cos_dec <= 0.05 {
            continue;
        }

        let mu_alpha = pm_ra / cos_dec;
        entry.ra += mu_alpha * dt * mas_to_rad;
        entry.dec += pm_dec * dt * mas_to_rad;
    }
}
