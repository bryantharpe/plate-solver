//! Hardware-neutral camera port. Adapters validate before changing any state.
use crate::detect::DetectSettings;
use std::{fmt, future::Future, pin::Pin};

pub use scope_link::proto::{CameraInfo, Mode};

/// Starting controls; an absent exposure or gain requests automatic control.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub exposure_ms: Option<f64>,
    pub gain: Option<f64>,
    pub detection: DetectSettings,
}

/// Unpadded 8-bit gray pixels with actual exposure timing and controls.
#[derive(Clone, Debug, PartialEq)]
pub struct GrayFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub start_s: f64,
    pub end_s: f64,
    pub exposure_ms: f64,
    pub gain: f64,
}

/// Object-safe async capture, so services can own any adapter behind the port.
pub type FrameFuture<'a> =
    Pin<Box<dyn Future<Output = Result<GrayFrame, CameraError>> + Send + 'a>>;

pub trait Camera: Send {
    fn info(&self) -> CameraInfo;
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError>;
    fn next_frame(&mut self) -> FrameFuture<'_>;
    /// Discard failed capture state; the next capture reopens the adapter.
    fn restart(&mut self) -> Result<(), CameraError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeError {
    pub setting: &'static str,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub unit: &'static str,
}

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}{} is outside the camera's range {}–{}{}",
            self.setting, self.value, self.unit, self.min, self.max, self.unit
        )
    }
}
impl std::error::Error for RangeError {}

/// Shared refusal rules: never clamp a fixed setting, including NaN/infinity.
pub fn check_range(info: &CameraInfo, settings: &Settings) -> Result<(), RangeError> {
    for (setting, value, min, max, unit) in [
        (
            "exposure_ms",
            settings.exposure_ms,
            info.exposure_min_ms,
            info.exposure_max_ms,
            " ms",
        ),
        ("gain", settings.gain, info.gain_min, info.gain_max, ""),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || value < min || value > max || value <= 0.0 {
                return Err(RangeError {
                    setting,
                    value,
                    min,
                    max,
                    unit,
                });
            }
        }
    }
    Ok(())
}

/// Adapter failures include subprocess exit status and captured stderr.
#[derive(Debug)]
pub struct CameraError(pub String);
impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CameraError {}
impl From<std::io::Error> for CameraError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{clock::SystemClock, synthetic::SyntheticCamera};
    use std::sync::Arc;

    #[tokio::test]
    async fn simulated_camera_refuses_exposure_before_taking_any_frame() {
        let mut camera = SyntheticCamera::new(Arc::new(SystemClock));
        let info = camera.info();
        let error = camera
            .apply(&Settings {
                exposure_ms: Some(info.exposure_max_ms + 1.0),
                ..Settings::default()
            })
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("exposure"));
        assert!(message.contains(&info.exposure_min_ms.to_string()));
        assert!(message.contains(&info.exposure_max_ms.to_string()));
        assert_eq!(camera.frames_taken(), 0);
    }

    #[tokio::test]
    async fn both_controls_refuse_nonfinite_and_out_of_range_values_atomically() {
        let mut camera = SyntheticCamera::new(Arc::new(SystemClock));
        let info = camera.info();
        for value in [
            info.gain_min / 2.0,
            info.gain_max + 1.0,
            f64::NAN,
            f64::INFINITY,
        ] {
            let error = camera
                .apply(&Settings {
                    exposure_ms: Some(info.exposure_max_ms),
                    gain: Some(value),
                    ..Settings::default()
                })
                .unwrap_err();
            assert_eq!(error.setting, "gain");
            assert_eq!((error.min, error.max), (info.gain_min, info.gain_max));
            assert_eq!(camera.frames_taken(), 0);
        }
        camera
            .apply(&Settings {
                exposure_ms: Some(info.exposure_min_ms),
                gain: Some(info.gain_max),
                ..Settings::default()
            })
            .unwrap();
        let frame = camera.next_frame().await.unwrap();
        assert_eq!(frame.exposure_ms, info.exposure_min_ms);
        assert_eq!(frame.gain, info.gain_max);
        assert_eq!(frame.pixels.len(), (frame.width * frame.height) as usize);
    }
}
