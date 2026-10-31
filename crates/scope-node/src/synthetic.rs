//! Database-free seeded stars for checking the scope node without hardware.
use crate::{
    camera::{check_range, FrameFuture, Mode},
    clock::NodeClock,
    Camera, CameraInfo, RangeError, Settings,
};
use std::{sync::Arc, time::Duration};

pub struct SyntheticCamera {
    clock: Arc<dyn NodeClock>,
    settings: Settings,
    frames: u64,
}
impl SyntheticCamera {
    pub fn new(clock: Arc<dyn NodeClock>) -> Self {
        Self {
            clock,
            settings: Settings::default(),
            frames: 0,
        }
    }
    pub fn frames_taken(&self) -> u64 {
        self.frames
    }
}
impl Camera for SyntheticCamera {
    fn info(&self) -> CameraInfo {
        let mode = Mode {
            width: 640,
            height: 480,
            fps: 5.0,
        };
        CameraInfo {
            model: "simulated".into(),
            width: mode.width,
            height: mode.height,
            modes: vec![mode],
            active_mode: Some(mode),
            exposure_min_ms: 1.0,
            exposure_max_ms: 2000.0,
            gain_min: 1.0,
            gain_max: 16.0,
        }
    }
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info(), settings)?;
        self.settings = settings.clone();
        Ok(())
    }
    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            let info = self.info();
            let exposure_ms = self.settings.exposure_ms.unwrap_or(20.0);
            let gain = self.settings.gain.unwrap_or(1.0);
            let start_s = self.clock.now();
            tokio::time::sleep(Duration::from_secs_f64((exposure_ms / 1000.0).max(0.2))).await;
            let mut pixels = vec![8; (info.width * info.height) as usize];
            let mut seed = 13u64;
            let mut random = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 32) as u32
            };
            for _ in 0..40 {
                let x = 16 + random() % (info.width - 32);
                let y = 16 + random() % (info.height - 32);
                let peak = (40.0 + f64::from(random() % 80)) * exposure_ms * gain / 20.0;
                for dy in -5i32..=5 {
                    for dx in -5i32..=5 {
                        let index = (y as i32 + dy) as usize * info.width as usize
                            + (x as i32 + dx) as usize;
                        let value =
                            peak * (-f64::from(dx * dx + dy * dy) / (2.0 * 1.4 * 1.4)).exp();
                        pixels[index] = (f64::from(pixels[index]) + value).min(255.0) as u8;
                    }
                }
            }
            self.frames += 1;
            Ok(crate::GrayFrame {
                pixels,
                width: info.width,
                height: info.height,
                start_s,
                end_s: start_s + exposure_ms / 1000.0,
                exposure_ms,
                gain,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FixedClock;
    impl NodeClock for FixedClock {
        fn now(&self) -> f64 {
            1234.0
        }
    }

    #[tokio::test(start_paused = true)]
    async fn synthetic_stars_are_seeded_and_use_the_supplied_clock_and_settings() {
        let mut camera = SyntheticCamera::new(Arc::new(FixedClock));
        let first = camera.next_frame().await.unwrap();
        assert_eq!((first.start_s, first.end_s), (1234.0, 1234.02));
        assert!(!crate::detect::detect(&first, &Default::default()).is_empty());
        assert_eq!(first, camera.next_frame().await.unwrap());
        camera
            .apply(&Settings {
                exposure_ms: Some(40.0),
                gain: Some(2.0),
                ..Default::default()
            })
            .unwrap();
        let brighter = camera.next_frame().await.unwrap();
        assert_eq!((brighter.start_s, brighter.end_s), (1234.0, 1234.04));
        assert_ne!(first.pixels, brighter.pixels);
        assert_eq!(camera.frames_taken(), 3);
    }
}
