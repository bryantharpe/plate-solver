//! Automatic exposure and gain control.
use crate::{CameraInfo, Settings};

pub struct Controller {
    exposure_min: f64,
    exposure_max: f64,
    gain_min: f64,
    gain_max: f64,
}
impl Controller {
    /// The exposure cap also respects the requested frame interval.
    pub fn new(info: &CameraInfo, frame_interval_ms: f64) -> Self {
        Self {
            exposure_min: info.exposure_min_ms,
            exposure_max: info
                .exposure_max_ms
                .min(frame_interval_ms)
                .max(info.exposure_min_ms),
            gain_min: info.gain_min,
            gain_max: info.gain_max,
        }
    }
    pub fn next(&self, exposure: f64, gain: f64, stars: usize, settings: &Settings) -> (f64, f64) {
        let mut e = settings.exposure_ms.unwrap_or(exposure);
        let mut g = settings.gain.unwrap_or(gain);
        if (25..=40).contains(&stars) {
            return (e, g);
        }
        let step = (30.0 / stars.max(1) as f64).powf(0.7).clamp(0.25, 4.0);
        if stars < 25 {
            if settings.exposure_ms.is_none() && e < self.exposure_max {
                e = (e * step).clamp(self.exposure_min, self.exposure_max);
            } else if settings.gain.is_none() {
                g = (g * step).clamp(self.gain_min, self.gain_max);
            }
        } else if settings.gain.is_none() && g > self.gain_min {
            g = (g * step).clamp(self.gain_min, self.gain_max);
        } else if settings.exposure_ms.is_none() {
            e = (e * step).clamp(self.exposure_min, self.exposure_max);
        }
        (e, g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CameraInfo, Settings};

    fn info() -> CameraInfo {
        CameraInfo {
            exposure_min_ms: 1.0,
            exposure_max_ms: 2000.0,
            gain_min: 1.0,
            gain_max: 16.0,
            ..CameraInfo::default()
        }
    }

    #[test]
    fn proportional_step_is_clamped_to_four_in_each_direction() {
        let c = Controller::new(&info(), 500.0);
        assert_eq!(c.next(20.0, 1.0, 0, &Settings::default()), (80.0, 1.0));
        assert_eq!(
            c.next(400.0, 1.0, 10000, &Settings::default()),
            (100.0, 1.0)
        );
        let expected = 100.0 * (30.0_f64 / 20.0).powf(0.7);
        assert!((c.next(100.0, 1.0, 20, &Settings::default()).0 - expected).abs() < 1e-10);
    }

    #[test]
    fn target_band_holds_and_gain_waits_for_exposure_cap() {
        let c = Controller::new(&info(), 500.0);
        for n in 25..=40 {
            assert_eq!(c.next(200.0, 3.0, n, &Settings::default()), (200.0, 3.0));
        }
        assert_eq!(c.next(100.0, 1.0, 1, &Settings::default()), (400.0, 1.0));
        let (e, g) = c.next(500.0, 1.0, 5, &Settings::default());
        assert_eq!(e, 500.0);
        assert!(g > 1.0 && g <= 4.0);
        let (e, g) = c.next(500.0, 4.0, 10000, &Settings::default());
        assert_eq!((e, g), (500.0, 1.0)); // Gain falls before exposure.
        assert_eq!(c.next(200.0, 16.0, 0, &Settings::default()).1, 16.0);
        assert_eq!(c.next(1.0, 1.0, 10000, &Settings::default()), (1.0, 1.0));
    }

    #[test]
    fn fixed_settings_are_never_moved() {
        let c = Controller::new(&info(), 500.0);
        for n in [0, 5, 30, 200] {
            let fixed = Settings {
                exposure_ms: Some(200.0),
                gain: Some(4.0),
                ..Settings::default()
            };
            assert_eq!(c.next(200.0, 4.0, n, &fixed), (200.0, 4.0));
            let exposure_fixed = Settings {
                gain: None,
                ..fixed.clone()
            };
            assert_eq!(c.next(200.0, 4.0, n, &exposure_fixed).0, 200.0);
            let gain_fixed = Settings {
                exposure_ms: None,
                ..fixed
            };
            assert_eq!(c.next(200.0, 4.0, n, &gain_fixed).1, 4.0);
        }
        let fixed = Settings {
            exposure_ms: Some(200.0),
            ..Settings::default()
        };
        assert!(c.next(200.0, 1.0, 5, &fixed).1 > 1.0);
    }
}
