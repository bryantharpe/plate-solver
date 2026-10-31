//! Camera port, adapters and cedar star detection for the scope node.
#![forbid(unsafe_code)]

pub mod camera;
pub mod capture;
pub mod clock;
pub mod config;
pub mod detect;
pub mod rpicam;
pub mod synthetic;

pub use camera::{Camera, CameraError, CameraInfo, GrayFrame, RangeError, Settings};
