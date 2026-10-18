//! Sub-pixel centroid and brightness measurement from a measurement box.

use crate::star::Star;

/// Compute the sub-pixel centroid and brightness of a star from a measurement box.
///
/// `image` is an 8-bit grayscale image stored row-major with `width` columns.
/// `box_left`, `box_top`, `box_width`, `box_height` define the measurement box
/// in pixel indices (integer coordinates of the top-left corner).
///
/// The 3 leftmost and 3 rightmost columns of the image are excluded from scanning,
/// but this function operates on the provided box directly; callers should ensure
/// the box lies within the scannable region.
///
/// Returns `None` if the box is too small to form an inset (needs at least 3x3)
/// or if the box extends outside the image.
pub fn measure_star(
    image: &[u8],
    width: usize,
    box_left: usize,
    box_top: usize,
    box_width: usize,
    box_height: usize,
) -> Option<Star> {
    measure_star_scaled(image, width, box_left, box_top, box_width, box_height, 1.0)
}

/// [`measure_star`], with the peak position multiplied by `scale` before the
/// 0.5 pixel-center offset is added.
///
/// cedar-detect does this when it centroids a binning-4 or binning-8 star on
/// the one-less-binned image: peak index `i` there is reported at
/// `scale * i + 0.5` in input coordinates.
pub(crate) fn measure_star_scaled(
    image: &[u8],
    width: usize,
    box_left: usize,
    box_top: usize,
    box_width: usize,
    box_height: usize,
    scale: f64,
) -> Option<Star> {
    if box_width < 3 || box_height < 3 {
        return None;
    }

    let height = image.len() / width;
    if box_left + box_width > width || box_top + box_height > height {
        return None;
    }

    // Build x and y projections over the box.
    let mut proj_x = vec![0u32; box_width];
    let mut proj_y = vec![0u32; box_height];

    for y in 0..box_height {
        for x in 0..box_width {
            let value = u32::from(image[(box_top + y) * width + (box_left + x)]);
            proj_x[x] += value;
            proj_y[y] += value;
        }
    }

    let peak_x = refine_peak(&proj_x)?;
    let peak_y = refine_peak(&proj_y)?;

    // Inset is the bounding box shrunk by 1 px on all sides.
    let inset_left = box_left + 1;
    let inset_top = box_top + 1;
    let inset_width = box_width - 2;
    let inset_height = box_height - 2;

    // Compute perimeter mean from the 1-px ring around the inset.
    let mut perimeter_sum = 0.0;
    let mut perimeter_count = 0usize;

    // Top and bottom edges of perimeter (full box width).
    for x in 0..box_width {
        perimeter_sum += image[box_top * width + (box_left + x)] as f64;
        perimeter_sum += image[(box_top + box_height - 1) * width + (box_left + x)] as f64;
        perimeter_count += 2;
    }
    // Left and right edges of perimeter, excluding corners already counted.
    for y in 1..(box_height - 1) {
        perimeter_sum += image[(box_top + y) * width + box_left] as f64;
        perimeter_sum += image[(box_top + y) * width + (box_left + box_width - 1)] as f64;
        perimeter_count += 2;
    }

    let perimeter_mean = if perimeter_count > 0 {
        perimeter_sum / perimeter_count as f64
    } else {
        0.0
    };

    // Background-subtracted sum over inset, peak value, saturated count. The
    // background is subtracted pixel by pixel, as cedar-detect does, so the
    // float sum rounds the same way.
    let mut brightness = 0.0;
    let mut peak_value = 0u8;
    let mut num_saturated = 0usize;

    for y in 0..inset_height {
        for x in 0..inset_width {
            let value = image[(inset_top + y) * width + (inset_left + x)];
            brightness += value as f64 - perimeter_mean;
            if value > peak_value {
                peak_value = value;
            }
            if value == 255 {
                num_saturated += 1;
            }
        }
    }

    Some(Star::new(
        (box_left as f64 + peak_x) * scale + 0.5,
        (box_top as f64 + peak_y) * scale + 0.5,
        peak_value,
        brightness,
        num_saturated,
    ))
}

/// Find the refined 1-D peak position in a projection, as cedar-detect does.
///
/// The peak `b` is the first maximum. A run of equal values starting at it
/// resolves to the run midpoint, and a peak at either end of the projection
/// takes that index. Otherwise the peak is refined by quadratic interpolation
/// `p = 0.5 * (a - c) / (a - 2b + c)` with its neighbors `a, c`; `b` is
/// strictly greater than both, so `p` lies in `[-0.5, 0.5]`.
/// The returned value is the sub-pixel peak index relative to the box origin,
/// which is then converted to the centroid coordinate by adding 0.5 and the
/// box offset in the caller.
fn refine_peak(proj: &[u32]) -> Option<f64> {
    if proj.is_empty() {
        return None;
    }

    let mut peak_val = 0u32;
    let mut peak_ind = 0usize;
    let mut in_run = false;
    let mut run_length = 0usize;
    for (ind, &val) in proj.iter().enumerate() {
        if val > peak_val || ind == 0 {
            peak_val = val;
            peak_ind = ind;
            run_length = 1;
            in_run = true;
        } else if val == peak_val {
            if in_run {
                run_length += 1;
            }
        } else {
            in_run = false;
        }
    }

    if run_length > 1 {
        return Some(peak_ind as f64 + (run_length - 1) as f64 / 2.0);
    }
    if peak_ind == 0 || peak_ind == proj.len() - 1 {
        return Some(peak_ind as f64);
    }

    let a = proj[peak_ind - 1] as f64;
    let b = proj[peak_ind] as f64;
    let c = proj[peak_ind + 1] as f64;
    Some(peak_ind as f64 + 0.5 * (a - c) / (a - 2.0 * b + c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_image(width: usize, height: usize, fill: u8) -> Vec<u8> {
        vec![fill; width * height]
    }

    #[test]
    fn pixel_center_convention_top_left() {
        // A single bright pixel at (0,0) in a 3x3 box.
        let mut img = make_image(10, 10, 10);
        img[0] = 200;
        img[1] = 50;
        img[10] = 50;
        let star = measure_star(&img, 10, 0, 0, 3, 3).unwrap();
        assert!((star.x - 0.5).abs() < 1e-9, "x = {}", star.x);
        assert!((star.y - 0.5).abs() < 1e-9, "y = {}", star.y);
    }

    #[test]
    fn quadratic_refinement_right_bias() {
        // Projection peak with right neighbor higher than left.
        let mut img = make_image(20, 10, 10);
        // Build a 5x3 box with values: x=5:50, x=6:100, x=7:80
        for y in 0..3 {
            img[y * 20 + 5] = 50;
            img[y * 20 + 6] = 100;
            img[y * 20 + 7] = 80;
        }
        let star = measure_star(&img, 20, 5, 0, 3, 3).unwrap();
        let a = 50.0;
        let b = 100.0;
        let c = 80.0;
        let expected_offset = 0.5 * (a - c) / (a - 2.0 * b + c);
        let expected_x = 6.0 + 0.5 + expected_offset;
        assert!((star.x - expected_x).abs() < 1e-9, "x = {}", star.x);
    }

    #[test]
    fn run_of_equal_values_resolves_to_midpoint() {
        let mut img = make_image(20, 10, 10);
        // Flat top from x=5..=7, y=0..2
        for y in 0..3 {
            for x in 5..=7 {
                img[y * 20 + x] = 100;
            }
        }
        let star = measure_star(&img, 20, 5, 0, 3, 3).unwrap();
        // Run midpoint is x=6, so centroid x = 6.5
        assert!((star.x - 6.5).abs() < 1e-9, "x = {}", star.x);
    }

    #[test]
    fn peak_at_edge_takes_edge_index() {
        let mut img = make_image(20, 10, 10);
        // Bright at left edge of box, x=0 in box (absolute x=3)
        for y in 0..3 {
            img[y * 20 + 3] = 200;
        }
        let star = measure_star(&img, 20, 3, 0, 3, 3).unwrap();
        // Edge index is 0, centroid x = box_left + 0 + 0.5 = 3.5
        assert!((star.x - 3.5).abs() < 1e-9, "x = {}", star.x);
    }

    #[test]
    fn brightness_is_background_subtracted_and_clamped() {
        let mut img = make_image(10, 10, 50);
        // 5x5 box, inset 3x3 at value 150, perimeter at 50.
        for y in 1..=3 {
            for x in 1..=3 {
                img[y * 10 + x] = 150;
            }
        }
        let star = measure_star(&img, 10, 0, 0, 5, 5).unwrap();
        // Perimeter mean = 50, inset sum = 9 * 150 = 1350, expected brightness = 1350 - 9*50 = 900
        assert!(
            (star.brightness - 900.0).abs() < 1e-9,
            "brightness = {}",
            star.brightness
        );
    }

    #[test]
    fn brightness_clamped_to_zero() {
        let mut img = make_image(10, 10, 200);
        // 5x5 box, inset 3x3 at value 100, perimeter at 200.
        for y in 1..=3 {
            for x in 1..=3 {
                img[y * 10 + x] = 100;
            }
        }
        let star = measure_star(&img, 10, 0, 0, 5, 5).unwrap();
        assert_eq!(star.brightness, 0.0);
    }

    #[test]
    fn peak_value_and_num_saturated() {
        let mut img = make_image(10, 10, 10);
        for y in 1..=3 {
            for x in 1..=3 {
                img[y * 10 + x] = 255;
            }
        }
        let star = measure_star(&img, 10, 0, 0, 5, 5).unwrap();
        assert_eq!(star.peak_value, 255);
        assert_eq!(star.num_saturated, 9);
    }

    #[test]
    fn stars_sort_by_brightness_descending() {
        let a = Star::new(1.0, 1.0, 100, 50.0, 0);
        let b = Star::new(2.0, 2.0, 200, 100.0, 0);
        let c = Star::new(3.0, 3.0, 150, 75.0, 0);
        let mut stars = [a, b, c];
        stars.sort();
        assert_eq!(stars[0].brightness, 100.0);
        assert_eq!(stars[1].brightness, 75.0);
        assert_eq!(stars[2].brightness, 50.0);
    }

    #[test]
    fn box_too_small_returns_none() {
        let img = make_image(10, 10, 10);
        assert!(measure_star(&img, 10, 0, 0, 2, 3).is_none());
        assert!(measure_star(&img, 10, 0, 0, 3, 2).is_none());
    }

    #[test]
    fn box_out_of_bounds_returns_none() {
        let img = make_image(10, 10, 10);
        assert!(measure_star(&img, 10, 8, 0, 3, 3).is_none());
        assert!(measure_star(&img, 10, 0, 8, 3, 3).is_none());
    }
}
