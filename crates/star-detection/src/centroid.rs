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
    if box_width < 3 || box_height < 3 {
        return None;
    }

    let height = image.len() / width;
    if box_left + box_width > width || box_top + box_height > height {
        return None;
    }

    // Build x and y projections over the box.
    let mut proj_x = vec![0.0; box_width];
    let mut proj_y = vec![0.0; box_height];

    for y in 0..box_height {
        for x in 0..box_width {
            let value = image[(box_top + y) * width + (box_left + x)] as f64;
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

    // Sum over inset, peak value, saturated count.
    let mut inset_sum = 0.0;
    let mut peak_value = 0u8;
    let mut num_saturated = 0usize;

    for y in 0..inset_height {
        for x in 0..inset_width {
            let value = image[(inset_top + y) * width + (inset_left + x)];
            inset_sum += value as f64;
            if value > peak_value {
                peak_value = value;
            }
            if value == 255 {
                num_saturated += 1;
            }
        }
    }

    let brightness = (inset_sum - perimeter_mean * (inset_width * inset_height) as f64).max(0.0);

    Some(Star::new(
        box_left as f64 + peak_x + 0.5,
        box_top as f64 + peak_y + 0.5,
        peak_value,
        brightness,
        num_saturated,
    ))
}

/// Find the refined 1-D peak position in a projection.
///
/// Uses quadratic interpolation `p = 0.5 * (a - c) / (a - 2b + c)` around the
/// peak `b` and its neighbors `a, c`. A run of equal peak values resolves to the
/// run midpoint. A peak at the projection edge takes the edge index.
/// The returned value is the sub-pixel peak index relative to the box origin,
/// which is then converted to the centroid coordinate by adding 0.5 and the
/// box offset in the caller.
fn refine_peak(proj: &[f64]) -> Option<f64> {
    if proj.is_empty() {
        return None;
    }
    if proj.len() == 1 {
        return Some(0.0);
    }

    // Find the maximum value and the contiguous run of equal maxima.
    let max_val = proj.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut run_start = None;
    let mut run_end = 0usize;

    for (i, &v) in proj.iter().enumerate() {
        if (v - max_val).abs() < 1e-12 {
            if run_start.is_none() {
                run_start = Some(i);
            }
            run_end = i;
        }
    }

    let run_start = run_start?;
    let run_mid = (run_start + run_end) as f64 / 2.0;

    // If the run touches an edge, take the edge index.
    if run_start == 0 || run_end == proj.len() - 1 {
        return Some(run_mid);
    }

    // If the run spans more than one sample, the midpoint is the answer.
    if run_end > run_start {
        return Some(run_mid);
    }

    // Single-sample peak: interpolate around it.
    let b_idx = run_start;
    let a = proj[b_idx - 1];
    let b = proj[b_idx];
    let c = proj[b_idx + 1];

    let denom = a - 2.0 * b + c;
    let offset = if denom.abs() < 1e-12 {
        0.0
    } else {
        0.5 * (a - c) / denom
    };

    // Clamp offset to [-0.5, 0.5].
    let offset = offset.clamp(-0.5, 0.5);

    Some(b_idx as f64 + offset)
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
        let mut stars = vec![a, b, c];
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
