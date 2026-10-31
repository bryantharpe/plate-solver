//! Capture through the camera port; a failed adapter does not terminate the service.
use crate::{detect, Camera, CameraError, GrayFrame, RangeError, Settings};
use scope_link::proto::Centroid;
use std::time::Duration;

#[derive(Debug)]
pub struct Captured {
    pub id: u64,
    pub frame: GrayFrame,
    pub centroids: Vec<Centroid>,
}

pub struct CaptureLoop<C> {
    camera: C,
    settings: Settings,
    id: u64,
    restart_at: Option<tokio::time::Instant>,
    restarts: u64,
}
impl<C: Camera> CaptureLoop<C> {
    pub fn new(mut camera: C, settings: Settings) -> Result<Self, RangeError> {
        camera.apply(&settings)?;
        Ok(Self {
            camera,
            settings,
            id: 0,
            restart_at: None,
            restarts: 0,
        })
    }
    pub fn restarts(&self) -> u64 {
        self.restarts
    }
    pub fn camera(&self) -> &C {
        &self.camera
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn apply(&mut self, settings: Settings) -> Result<(), RangeError> {
        self.camera.apply(&settings)?;
        self.settings = settings;
        Ok(())
    }
    /// An error is observable by the service; its next call retries after 1 s.
    pub async fn next(&mut self) -> Result<Captured, CameraError> {
        let result = self.capture().await;
        if let Err(error) = &result {
            eprintln!("camera capture failed: {error}");
            self.restart_at = Some(tokio::time::Instant::now() + Duration::from_secs(1));
        }
        result
    }
    async fn capture(&mut self) -> Result<Captured, CameraError> {
        if let Some(at) = self.restart_at.take() {
            tokio::time::sleep_until(at).await;
            self.camera.restart()?;
            self.restarts += 1;
        }
        let frame = self.camera.next_frame().await?;
        self.id += 1;
        let centroids = detect::detect(&frame, &self.settings.detection);
        Ok(Captured {
            id: self.id,
            frame,
            centroids,
        })
    }
}
