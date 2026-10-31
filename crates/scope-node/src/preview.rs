//! Small JPEG previews with node-clock pacing.
use crate::GrayFrame;
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType};

#[derive(Default)]
pub struct PreviewSchedule {
    last: Option<f64>,
}
impl PreviewSchedule {
    /// Mark a preview at this exposure end. Explicit requests reset the interval.
    pub fn due(&mut self, now: f64, requested: bool) -> bool {
        if requested || self.last.is_none_or(|last| now - last >= 5.0) {
            self.last = Some(now);
            true
        } else {
            false
        }
    }
}

/// Box average into the aspect-preserving size before quality-80 JPEG encoding.
pub fn encode(frame: &GrayFrame) -> image::ImageResult<Vec<u8>> {
    let longest = frame.width.max(frame.height);
    let scale = 640.0 / f64::from(longest.max(640));
    let width = (f64::from(frame.width) * scale).floor().max(1.0) as u32;
    let height = (f64::from(frame.height) * scale).floor().max(1.0) as u32;
    // ImageBuffer validates the port's size/pixel contract before indexing.
    let source = image::GrayImage::from_raw(frame.width, frame.height, frame.pixels.clone())
        .ok_or_else(|| {
            image::ImageError::Parameter(image::error::ParameterError::from_kind(
                image::error::ParameterErrorKind::DimensionMismatch,
            ))
        })?;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        let y0 = y * frame.height / height;
        let y1 = ((y + 1) * frame.height / height).max(y0 + 1);
        for x in 0..width {
            let x0 = x * frame.width / width;
            let x1 = ((x + 1) * frame.width / width).max(x0 + 1);
            let mut sum = 0_u64;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    sum += u64::from(source[(sx, sy)][0]);
                }
            }
            pixels.push((sum / u64::from((x1 - x0) * (y1 - y0))) as u8);
        }
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 80).encode(
        &pixels,
        width,
        height,
        ExtendedColorType::L8,
    )?;
    Ok(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GrayFrame;

    #[test]
    fn jpeg_dimensions_preserve_aspect_and_do_not_upscale() {
        for (w, h, expected) in [
            (1456, 1088, (640, 478)),
            (800, 1600, (320, 640)),
            (123, 45, (123, 45)),
        ] {
            let frame = GrayFrame {
                pixels: vec![80; (w * h) as usize],
                width: w,
                height: h,
                start_s: 1000.0,
                end_s: 1000.2,
                exposure_ms: 200.0,
                gain: 1.0,
            };
            let jpeg = encode(&frame).unwrap();
            assert_eq!(&jpeg[..2], &[0xff, 0xd8]);
            let decoded =
                image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg).unwrap();
            assert_eq!((decoded.width(), decoded.height()), expected);
        }
    }

    #[test]
    fn periodic_limit_is_five_seconds_and_requests_override_it() {
        let mut p = PreviewSchedule::default();
        assert!(p.due(1000.0, false));
        assert!(!p.due(1004.999, false));
        assert!(p.due(1005.0, false));
        assert!(p.due(1006.0, true));
        assert!(!p.due(1010.999, false));
        assert!(p.due(1011.0, false));
    }

    #[test]
    fn box_filter_averages_source_pixels() {
        let mut frame = GrayFrame {
            pixels: vec![0; 1280 * 2],
            width: 1280,
            height: 2,
            start_s: 0.0,
            end_s: 1.0,
            exposure_ms: 1000.0,
            gain: 1.0,
        };
        for row in frame.pixels.chunks_mut(1280) {
            for pair in row.chunks_mut(2) {
                pair[1] = 200;
            }
        }
        let jpeg = encode(&frame).unwrap();
        let gray = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg)
            .unwrap()
            .to_luma8();
        assert_eq!(gray.dimensions(), (640, 1));
        assert!(gray.pixels().all(|p| (i16::from(p[0]) - 100).abs() <= 2));
    }
}
