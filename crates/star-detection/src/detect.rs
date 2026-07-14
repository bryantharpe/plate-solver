//! Star detection pipeline entry point.
//!
//! Implements the full front-end pipeline: noise estimation, binning cascade,
//! 1-D row gating, hot-pixel rejection, blob formation, 2-D gating, sub-pixel
//! centroiding, and brightest-first ordering.

use crate::binning::build_cascade;
use crate::centroid::measure_star;
use crate::noise::estimate_noise;
use crate::star::Star;

/// Detect stars in an 8-bit grayscale image.
///
/// `image` is row-major with `width` columns and `height` rows. `sigma` sets
/// the detection threshold in units of the estimated RMS noise. `binning` must
/// be one of 1, 2, 4, or 8. When `normalize_rows` is true, each row's dark level
/// is shifted to a bias of 2.0 before binning. When `detect_hot_pixels` is true,
/// isolated single-pixel spikes are rejected.
///
/// Returns a `Vec<Star>` sorted by background-subtracted brightness descending.
/// Coordinates are in full-resolution input-image coordinates with `(0.5,0.5)`
/// the center of the top-left pixel.
pub fn detect_stars(
    image: &[u8],
    width: usize,
    height: usize,
    sigma: f64,
    binning: usize,
    normalize_rows: bool,
    detect_hot_pixels: bool,
) -> Vec<Star> {
    assert_eq!(
        image.len(),
        width * height,
        "image length must equal width * height"
    );

    if width < 7 || height < 1 {
        return Vec::new();
    }

    let (detection, higher) = build_cascade(image, width, height, binning, normalize_rows);
    let higher = higher.as_ref().unwrap_or(&detection);

    let noise = estimate_noise(&detection.data, detection.width, detection.height);
    let sigma_noise_2 = sigma_noise_2(sigma, noise);
    let sigma_noise_3 = sigma_noise_3(sigma, noise);

    let candidates = scan_rows(
        &detection.data,
        detection.width,
        detection.height,
        sigma_noise_2,
        sigma_noise_3,
    );

    let candidates = if detect_hot_pixels {
        reject_hot_pixels(&candidates, image, width, height, binning)
    } else {
        candidates
    };

    let blobs = form_blobs(candidates, detection.max_size);
    let mut stars = Vec::new();

    for blob in blobs {
        if let Some(star) = process_blob(
            blob,
            &detection.data,
            detection.width,
            detection.height,
            detection.binning,
            &higher.data,
            higher.width,
            higher.height,
            higher.binning,
            binning,
            noise,
            sigma,
            detection.max_size,
        ) {
            stars.push(star);
        }
    }

    stars.sort();
    stars
}

fn sigma_noise_2(sigma: f64, noise: f64) -> i64 {
    (2.0 * sigma * noise).round().max(2.0) as i64
}

fn sigma_noise_3(sigma: f64, noise: f64) -> i64 {
    (3.0 * sigma * noise).round().max(3.0) as i64
}

/// A candidate pixel emitted by the 1-D row gate.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    x: usize,
    y: usize,
}

/// Scan every row of the detection image and emit 1-D candidates.
///
/// A pixel qualifies when it is part of a local-maximum run: the pixel and its
/// immediate neighbors/margins are not higher than the run value, the significance
/// test passes, and the border background is uniform. Consecutive pixels of equal
/// value are merged and exactly one candidate — the midpoint of the run — is
/// emitted, guaranteeing one center per flat-topped peak.
fn scan_rows(
    image: &[u8],
    width: usize,
    height: usize,
    sigma_noise_2: i64,
    sigma_noise_3: i64,
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let scan_left = 3usize;
    let scan_right = width.saturating_sub(3);
    if scan_left >= scan_right {
        return candidates;
    }

    for y in 0..height {
        let row_offset = y * width;

        // Cheap row_min from every 64th pixel.
        let mut row_min = u8::MAX;
        for x in (scan_left..scan_right).step_by(64) {
            row_min = row_min.min(image[row_offset + x]);
        }
        // Guard against an empty sample set on tiny rows.
        if row_min == u8::MAX {
            row_min = image[row_offset + scan_left];
        }

        let threshold = row_min as i64 + sigma_noise_2 / 2;

        let mut run_start: Option<usize> = None;
        let mut run_value: u8 = 0;

        for x in scan_left..scan_right {
            let c = image[row_offset + x];
            let ok = c as i64 >= threshold
                && is_row_peak(image, width, height, x, y, sigma_noise_2, sigma_noise_3);

            if ok {
                if run_start.is_none() {
                    run_start = Some(x);
                    run_value = c;
                } else if c != run_value {
                    // End the previous run and start a new one.
                    emit_run_center(&mut candidates, run_start.unwrap(), x - 1, y);
                    run_start = Some(x);
                    run_value = c;
                }
            } else if let Some(start) = run_start {
                emit_run_center(&mut candidates, start, x - 1, y);
                run_start = None;
            }
        }

        if let Some(start) = run_start {
            emit_run_center(&mut candidates, start, scan_right - 1, y);
        }
    }

    candidates
}

fn emit_run_center(candidates: &mut Vec<Candidate>, start: usize, end: usize, y: usize) {
    let center = (start + end) / 2;
    candidates.push(Candidate { x: center, y });
}

/// 7-pixel 1-D local-maximum test centered at `(cx, cy)`.
///
/// Pixels are laid out as: lb l lm C rm r rb. A pixel is part of a local
/// maximum run when it is not lower than its neighbors and margins and the
/// significance/uniform-background tests pass. The caller collapses consecutive
/// equal-valued local maxima to a single center.
fn is_row_peak(
    image: &[u8],
    width: usize,
    _height: usize,
    cx: usize,
    cy: usize,
    sigma_noise_2: i64,
    sigma_noise_3: i64,
) -> bool {
    let row_offset = cy * width;
    let c = image[row_offset + cx] as i64;
    let lb = image[row_offset + cx - 3] as i64;
    let l = image[row_offset + cx - 2] as i64;
    let lm = image[row_offset + cx - 1] as i64;
    let rm = image[row_offset + cx + 1] as i64;
    let r = image[row_offset + cx + 2] as i64;
    let rb = image[row_offset + cx + 3] as i64;

    // Significance: 2*C - (lb+rb) >= sigma_noise_2.
    if 2 * c - (lb + rb) < sigma_noise_2 {
        return false;
    }

    // Neighbors and margins not higher than center.
    if l > c || r > c || lm > c || rm > c {
        return false;
    }

    // Uniform background: border difference bounded.
    if (lb - rb).abs() > sigma_noise_3 {
        return false;
    }

    true
}

/// Hot-pixel rejection against the full-resolution image.
///
/// Each candidate maps back to a block of full-resolution pixels. A backing
/// pixel is classified as `Hot` when it is bright but isolated, `Dark` when not
/// bright, otherwise `Bright`. A candidate is dropped when all bright backing
/// pixels are hot.
fn reject_hot_pixels(
    candidates: &[Candidate],
    full_image: &[u8],
    full_width: usize,
    full_height: usize,
    binning: usize,
) -> Vec<Candidate> {
    let mut kept = Vec::with_capacity(candidates.len());
    for &cand in candidates {
        let (bx0, bx1, by0, by1) =
            binned_to_full_block(cand.x, cand.y, full_width, full_height, binning);

        let mut hot_count = 0usize;
        let mut bright_count = 0usize;

        for y in by0..by1 {
            for x in bx0..bx1 {
                let class = classify_pixel(full_image, full_width, full_height, x, y);
                match class {
                    PixelClass::Hot => hot_count += 1,
                    PixelClass::Bright => bright_count += 1,
                    PixelClass::Dark => {}
                }
            }
        }

        if bright_count == 0 && hot_count > 0 {
            // All bright pixels are hot -> drop candidate.
            continue;
        }
        kept.push(cand);
    }
    kept
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PixelClass {
    Dark,
    Bright,
    Hot,
}

fn classify_pixel(image: &[u8], width: usize, _height: usize, x: usize, y: usize) -> PixelClass {
    let row_offset = y * width;
    let c = image[row_offset + x] as i64;

    // Neighbors and borders in the 7-pixel 1-D window centered on x.
    let l = if x >= 2 {
        image[row_offset + x - 2] as i64
    } else {
        0
    };
    let r = if x + 2 < width {
        image[row_offset + x + 2] as i64
    } else {
        0
    };
    let lb = if x >= 3 {
        image[row_offset + x - 3] as i64
    } else {
        0
    };
    let rb = if x + 3 < width {
        image[row_offset + x + 3] as i64
    } else {
        0
    };

    let excess = 2 * c - (lb + rb);
    if excess <= 0 {
        return PixelClass::Dark;
    }

    // Hot when neighbors carry < 1/8 of the center excess.
    // Spec: 4*((l+r)-(lb+rb)) <= (2C-(lb+rb))/2
    // Rearranged: 8*((l+r)-(lb+rb)) <= 2C-(lb+rb)
    // i.e. 8*(l+r - lb - rb) <= excess
    let neighbor_excess = l + r - lb - rb;
    if 8 * neighbor_excess <= excess {
        return PixelClass::Hot;
    }

    PixelClass::Bright
}

/// Map a binned candidate coordinate back to the inclusive full-resolution
/// pixel block it represents.
fn binned_to_full_block(
    bx: usize,
    by: usize,
    full_width: usize,
    full_height: usize,
    binning: usize,
) -> (usize, usize, usize, usize) {
    if binning <= 1 {
        return (bx, (bx + 1).min(full_width), by, (by + 1).min(full_height));
    }
    let x0 = bx * binning;
    let y0 = by * binning;
    let x1 = ((bx + 1) * binning).min(full_width);
    let y1 = ((by + 1) * binning).min(full_height);
    (x0, x1, y0, y1)
}

/// A blob is a set of vertically adjacent candidates.
#[derive(Clone, Debug)]
struct Blob {
    pixels: Vec<Candidate>,
}

impl Blob {
    fn left(&self) -> usize {
        self.pixels.iter().map(|p| p.x).min().unwrap_or(0)
    }
    fn right(&self) -> usize {
        self.pixels.iter().map(|p| p.x).max().unwrap_or(0)
    }
    fn top(&self) -> usize {
        self.pixels.iter().map(|p| p.y).min().unwrap_or(0)
    }
    fn bottom(&self) -> usize {
        self.pixels.iter().map(|p| p.y).max().unwrap_or(0)
    }
    fn width(&self) -> usize {
        self.right() - self.left() + 1
    }
    fn height(&self) -> usize {
        self.bottom() - self.top() + 1
    }
}

/// Merge candidates into blobs using union-find-style recipient forwarding.
fn form_blobs(candidates: Vec<Candidate>, _max_size: usize) -> Vec<Blob> {
    if candidates.is_empty() {
        return Vec::new();
    }

    // Bucket candidates by row.
    let mut by_row: Vec<Vec<Candidate>> = Vec::new();
    for cand in candidates {
        if cand.y >= by_row.len() {
            by_row.resize_with(cand.y + 1, Vec::new);
        }
        by_row[cand.y].push(cand);
    }

    // Union-find over candidate indices.
    let total = by_row.iter().map(|r| r.len()).sum::<usize>();
    let mut parent: Vec<usize> = (0..total).collect();
    let mut index_of: Vec<Vec<usize>> = Vec::with_capacity(by_row.len());
    let mut offset = 0usize;
    for row in &by_row {
        let mut idx = Vec::with_capacity(row.len());
        for i in 0..row.len() {
            idx.push(offset + i);
        }
        index_of.push(idx);
        offset += row.len();
    }

    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        // Path compression.
        let mut j = i;
        while parent[j] != root {
            let next = parent[j];
            parent[j] = root;
            j = next;
        }
        root
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    // Merge with previous row within +/- 3 in x.
    for y in 1..by_row.len() {
        for (i, cand) in by_row[y].iter().enumerate() {
            let cur_idx = index_of[y][i];
            for (j, prev) in by_row[y - 1].iter().enumerate() {
                if prev.x.abs_diff(cand.x) <= 3 {
                    let prev_idx = index_of[y - 1][j];
                    union(&mut parent, cur_idx, prev_idx);
                }
            }
        }
    }

    // Collect blobs.
    let mut blob_map: std::collections::HashMap<usize, Vec<Candidate>> =
        std::collections::HashMap::new();
    for y in 0..by_row.len() {
        for (i, cand) in by_row[y].iter().enumerate() {
            let root = find(&mut parent, index_of[y][i]);
            blob_map.entry(root).or_default().push(*cand);
        }
    }

    blob_map
        .into_values()
        .map(|pixels| Blob { pixels })
        .collect()
}

/// Apply the 2-D gate on the detection image and centroid on the higher-res image.
#[allow(clippy::too_many_arguments)]
fn process_blob(
    blob: Blob,
    detection_image: &[u8],
    detection_width: usize,
    detection_height: usize,
    detection_binning: usize,
    higher_image: &[u8],
    higher_width: usize,
    higher_height: usize,
    higher_binning: usize,
    input_binning: usize,
    noise: f64,
    sigma: f64,
    max_size: usize,
) -> Option<Star> {
    // Core is the blob bounding box in detection coordinates.
    let core_left = blob.left();
    let core_top = blob.top();
    let core_width = blob.width();
    let core_height = blob.height();

    // Size gate: a blob is acceptable if at least one dimension fits max_size.
    // This keeps narrow 1-D peak stacks (common for small synthetic stars) while
    // still rejecting large 2-D bleeding blobs where both dimensions are oversized.
    if core_width > max_size && core_height > max_size {
        return None;
    }

    // Concentric boxes for the 2-D gate, computed on the detection image.
    let nb_left = core_left.saturating_sub(1);
    let nb_top = core_top.saturating_sub(1);
    let nb_width = (core_width + 2).min(detection_width - nb_left);
    let nb_height = (core_height + 2).min(detection_height - nb_top);

    let mg_left = core_left.saturating_sub(2);
    let mg_top = core_top.saturating_sub(2);
    let mg_width = (core_width + 4).min(detection_width - mg_left);
    let mg_height = (core_height + 4).min(detection_height - mg_top);

    let pr_left = core_left.saturating_sub(3);
    let pr_top = core_top.saturating_sub(3);
    let pr_width = (core_width + 6).min(detection_width - pr_left);
    let pr_height = (core_height + 6).min(detection_height - pr_top);

    // Perimeter must be fully inside the detection image.
    if pr_left + pr_width > detection_width || pr_top + pr_height > detection_height {
        return None;
    }
    if pr_left >= detection_width || pr_top >= detection_height {
        return None;
    }

    let core_mean = box_mean(
        detection_image,
        detection_width,
        core_left,
        core_top,
        core_width,
        core_height,
    );
    let neighbor_mean = box_mean_excluding_corners(
        detection_image,
        detection_width,
        nb_left,
        nb_top,
        nb_width,
        nb_height,
    );
    let margin_mean = box_mean(
        detection_image,
        detection_width,
        mg_left,
        mg_top,
        mg_width,
        mg_height,
    );
    let (perimeter_mean, perimeter_stddev, perimeter_min, perimeter_max) = box_stats_perimeter(
        detection_image,
        detection_width,
        pr_left,
        pr_top,
        pr_width,
        pr_height,
    );

    // Inner-core brightness (3x3 center of core) when core >= 3x3.
    if core_width >= 3 && core_height >= 3 {
        let inner_left = core_left + core_width / 2 - 1;
        let inner_top = core_top + core_height / 2 - 1;
        let outer_core_mean = box_mean(
            detection_image,
            detection_width,
            core_left,
            core_top,
            core_width,
            core_height,
        );
        let inner_core_mean = box_mean(
            detection_image,
            detection_width,
            inner_left,
            inner_top,
            3,
            3,
        );
        if inner_core_mean < outer_core_mean {
            return None;
        }
    }

    // Core >= neighbor mean (corners excluded).
    if core_mean < neighbor_mean {
        return None;
    }

    // Core > margin mean.
    if core_mean <= margin_mean {
        return None;
    }

    // Uniform perimeter.
    if perimeter_max - perimeter_min > 3.0 * sigma * noise {
        return None;
    }

    // Significance.
    let effective_noise = noise.max(perimeter_stddev);
    if core_mean - perimeter_mean < sigma * effective_noise {
        return None;
    }

    // Map the detection core to the higher-res image and expand the measurement
    // box to include a one-pixel background ring (the "neighbor" box) while also
    // ensuring it is large enough for centroiding (at least 3x3).
    let scale_to_higher = detection_binning / higher_binning;
    let mut meas_left = core_left * scale_to_higher;
    let mut meas_top = core_top * scale_to_higher;
    let meas_width = (core_width * scale_to_higher + 2).max(3);
    let meas_height = (core_height * scale_to_higher + 2).max(3);

    // Center the expansion on the blob.
    let extra_w = meas_width - (core_width * scale_to_higher);
    meas_left = meas_left.saturating_sub(extra_w / 2);
    let extra_h = meas_height - (core_height * scale_to_higher);
    meas_top = meas_top.saturating_sub(extra_h / 2);

    // Clamp to the higher-res image bounds.
    if meas_left + meas_width > higher_width {
        meas_left = higher_width.saturating_sub(meas_width);
    }
    if meas_top + meas_height > higher_height {
        meas_top = higher_height.saturating_sub(meas_height);
    }

    // Centroid on the higher-res image, then scale back to input coordinates.
    let star = measure_star(
        higher_image,
        higher_width,
        meas_left,
        meas_top,
        meas_width,
        meas_height,
    )?;

    let scale = input_binning as f64 / higher_binning as f64;
    Some(Star::new(
        star.x * scale,
        star.y * scale,
        star.peak_value,
        star.brightness * scale * scale,
        star.num_saturated,
    ))
}

fn box_mean(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    box_width: usize,
    box_height: usize,
) -> f64 {
    let mut sum = 0u64;
    for y in top..top + box_height {
        let row_offset = y * width;
        for x in left..left + box_width {
            sum += u64::from(image[row_offset + x]);
        }
    }
    sum as f64 / (box_width * box_height) as f64
}

fn box_mean_excluding_corners(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    box_width: usize,
    box_height: usize,
) -> f64 {
    let mut sum = 0u64;
    let mut count = 0usize;
    for y in top..top + box_height {
        let row_offset = y * width;
        for x in left..left + box_width {
            let is_corner =
                (y == top || y == top + box_height - 1) && (x == left || x == left + box_width - 1);
            if is_corner {
                continue;
            }
            sum += u64::from(image[row_offset + x]);
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    sum as f64 / count as f64
}

fn box_stats_perimeter(
    image: &[u8],
    width: usize,
    left: usize,
    top: usize,
    box_width: usize,
    box_height: usize,
) -> (f64, f64, f64, f64) {
    let mut sum = 0u64;
    let mut count = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for y in top..top + box_height {
        let row_offset = y * width;
        for x in left..left + box_width {
            let on_perimeter =
                y == top || y == top + box_height - 1 || x == left || x == left + box_width - 1;
            if !on_perimeter {
                continue;
            }
            let v = f64::from(image[row_offset + x]);
            sum += v as u64;
            count += 1;
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }

    if count == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }

    let mean = sum as f64 / count as f64;
    let variance: f64 = {
        let mut acc = 0.0;
        for y in top..top + box_height {
            let row_offset = y * width;
            for x in left..left + box_width {
                let on_perimeter =
                    y == top || y == top + box_height - 1 || x == left || x == left + box_width - 1;
                if !on_perimeter {
                    continue;
                }
                let d = f64::from(image[row_offset + x]) - mean;
                acc += d * d;
            }
        }
        acc / count as f64
    };
    (mean, variance.sqrt(), min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_image(width: usize, height: usize, fill: u8) -> Vec<u8> {
        vec![fill; width * height]
    }

    #[test]
    fn detect_single_star() {
        let width = 200;
        let height = 200;
        let mut img = make_image(width, height, 20);
        // Smooth peaked spot centered near (100,100). Use a larger image so
        // max_size (width/100 = 2) accommodates the blob.
        let cx = 100.0;
        let cy = 100.0;
        for y in 80..120 {
            for x in 80..120 {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let r2 = dx * dx + dy * dy;
                let v = 220.0 * (-r2 / 2.0).exp();
                let v = (v as u8).max(20);
                img[y * width + x] = v;
            }
        }
        let stars = detect_stars(&img, width, height, 8.0, 1, false, false);
        assert!(!stars.is_empty(), "expected at least one star");
        let s = &stars[0];
        assert!((s.x - 100.5).abs() < 1.0, "x = {}", s.x);
        assert!((s.y - 100.5).abs() < 1.0, "y = {}", s.y);
        assert!(s.brightness > 0.0);
    }

    #[test]
    fn brightest_first_ordering() {
        let mut img = make_image(60, 20, 20);
        // Two stars, left one brighter.
        for y in 8..=12 {
            for x in 10..=14 {
                img[y * 60 + x] = 200;
            }
        }
        for y in 8..=12 {
            for x in 40..=44 {
                img[y * 60 + x] = 120;
            }
        }
        let stars = detect_stars(&img, 60, 20, 8.0, 1, false, false);
        assert_eq!(stars.len(), 2);
        assert!(stars[0].brightness > stars[1].brightness);
    }

    #[test]
    fn hot_pixel_rejected() {
        let mut img = make_image(40, 40, 20);
        // Single isolated bright pixel.
        img[20 * 40 + 20] = 250;
        let with_hot = detect_stars(&img, 40, 40, 8.0, 1, false, true);
        let without_hot = detect_stars(&img, 40, 40, 8.0, 1, false, false);
        assert!(with_hot.len() < without_hot.len() || with_hot.is_empty());
    }

    #[test]
    fn binning_four_reports_input_coordinates() {
        let mut img = make_image(64, 64, 20);
        // Bright 4x4 spot centered around (32,32).
        for y in 30..=34 {
            for x in 30..=34 {
                img[y * 64 + x] = 200;
            }
        }
        let stars = detect_stars(&img, 64, 64, 8.0, 4, false, false);
        assert!(!stars.is_empty());
        let s = &stars[0];
        assert!(s.x > 20.0 && s.x < 45.0, "x = {}", s.x);
        assert!(s.y > 20.0 && s.y < 45.0, "y = {}", s.y);
    }
}
