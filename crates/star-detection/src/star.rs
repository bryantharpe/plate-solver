//! The `Star` output type and ordering conventions.

/// A detected star with sub-pixel centroid and brightness information.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Sub-pixel x coordinate in full-resolution image coordinates.
    /// `(0.5, 0.5)` is the center of the top-left pixel.
    pub x: f64,
    /// Sub-pixel y coordinate in full-resolution image coordinates.
    pub y: f64,
    /// Maximum pixel value in the measurement inset (bounding box shrunk by 1 px).
    pub peak_value: u8,
    /// Perimeter-background-subtracted sum over the inset, clamped to >= 0.
    pub brightness: f64,
    /// Number of saturated pixels (value == 255) in the inset.
    pub num_saturated: usize,
}

impl Star {
    /// Create a new star from raw fields.
    pub fn new(x: f64, y: f64, peak_value: u8, brightness: f64, num_saturated: usize) -> Self {
        Self {
            x,
            y,
            peak_value,
            brightness: brightness.max(0.0),
            num_saturated,
        }
    }
}

impl Eq for Star {}

impl PartialOrd for Star {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Star {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by brightness descending; tie-break by coordinates for determinism.
        other
            .brightness
            .partial_cmp(&self.brightness)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                self.y
                    .partial_cmp(&other.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        self.x
                            .partial_cmp(&other.x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            })
    }
}
