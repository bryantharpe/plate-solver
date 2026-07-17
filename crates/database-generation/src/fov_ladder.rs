//! Multiscale FOV ladder for pattern generation.
//!
//! Patterns must be sized to the camera FOV. For a FOV range, build patterns at
//! one or more geometrically-spaced scales, then pool and dedup patterns from all
//! scales.

/// Compute the FOV scales (in degrees) at which to generate patterns.
///
/// * `fov_ratio = max_fov / min_fov`
/// * `fov_divisions = ceil(log_step(fov_ratio)) + 1`
///
/// If `fov_ratio < sqrt(multiscale_step)`, returns a single scale `[max_fov]`.
/// Otherwise returns `2^linspace(log2(min_fov), log2(max_fov), fov_divisions)`.
///
/// The reference uses natural logs with `logk(x, k) = ln(x)/ln(k)`; the result is
/// identical to `log2` because the base cancels in the linspace.
pub fn fov_ladder(min_fov: f64, max_fov: f64, multiscale_step: f64) -> Vec<f64> {
    assert!(min_fov > 0.0 && max_fov >= min_fov && multiscale_step > 1.0);

    let fov_ratio = max_fov / min_fov;
    if fov_ratio < multiscale_step.sqrt() {
        return vec![max_fov];
    }

    let log_step = fov_ratio.ln() / multiscale_step.ln();
    let fov_divisions = log_step.ceil() as usize + 1;

    let log2_min = min_fov.log2();
    let log2_max = max_fov.log2();

    if fov_divisions <= 1 {
        return vec![max_fov];
    }

    let step = (log2_max - log2_min) / (fov_divisions - 1) as f64;
    (0..fov_divisions)
        .map(|i| {
            let exponent = log2_min + i as f64 * step;
            2.0_f64.powf(exponent)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_range_uses_single_scale() {
        // max/min = 1.2, sqrt(1.5) ~ 1.225, so single scale.
        let scales = fov_ladder(10.0, 12.0, 1.5);
        assert_eq!(scales.len(), 1);
        assert!((scales[0] - 12.0).abs() < 1e-12);
    }

    #[test]
    fn wide_range_uses_geometric_ladder() {
        let scales = fov_ladder(10.0, 30.0, 1.5);
        // fov_ratio = 3, log_1.5(3) ~ 2.71, divisions = 4.
        assert_eq!(scales.len(), 4);
        assert!((scales[0] - 10.0).abs() < 1e-9);
        assert!((scales[scales.len() - 1] - 30.0).abs() < 1e-9);
        // Geometric: each step multiplies by a constant factor.
        let factor = (scales[1] / scales[0]).ln();
        for i in 2..scales.len() {
            let f = (scales[i] / scales[i - 1]).ln();
            assert!((f - factor).abs() < 1e-9, "ladder is not geometric");
        }
    }

    #[test]
    fn exact_ratio_at_threshold_still_single() {
        // fov_ratio just below sqrt(multiscale_step) gives a single scale.
        let multiscale_step: f64 = 1.5;
        let max_fov = multiscale_step.sqrt() * 10.0 - 1e-9;
        let scales = fov_ladder(10.0, max_fov, multiscale_step);
        assert_eq!(scales.len(), 1);
    }
}
