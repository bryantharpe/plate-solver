//! Histogram utilities for noise and background estimation.

/// Statistics computed from a 256-bin pixel histogram.
#[derive(Debug)]
pub struct HistogramStats {
    pub mean: f64,
    pub median: usize,
    pub stddev: f64,
}

/// Compute mean, median, and standard deviation from a 256-bin histogram.
///
/// Two-pass algorithm: first pass accumulates count and first moment,
/// second pass accumulates second moment and finds the median bin.
pub fn stats_for_histogram(histogram: &[u32; 256]) -> HistogramStats {
    let mut count: u32 = 0;
    let mut first_moment: u64 = 0;
    for (h, &bin_count) in histogram.iter().enumerate() {
        count += bin_count;
        first_moment += (bin_count as u64) * (h as u64);
    }
    if count == 0 {
        return HistogramStats {
            mean: 0.0,
            median: 0,
            stddev: 0.0,
        };
    }
    let mean = first_moment as f64 / count as f64;

    let mut second_moment: f64 = 0.0;
    let mut sub_count: u32 = 0;
    let mut median = 0;
    for (h, &bin_count) in histogram.iter().enumerate() {
        second_moment += bin_count as f64 * (h as f64 - mean) * (h as f64 - mean);
        if sub_count < count / 2 {
            sub_count += bin_count;
            if sub_count >= count / 2 {
                median = h;
            }
        }
    }
    let stddev = (second_moment / count as f64).sqrt();
    HistogramStats { mean, median, stddev }
}

/// Trim a histogram to keep only the first `count_to_keep` pixels
/// (lowest bin values). Bins beyond the cutoff become 0.
fn trim_histogram(histogram: &mut [u32; 256], count_to_keep: u32) {
    let mut count: u32 = 0;
    for bin in histogram.iter_mut() {
        let bin_count = *bin;
        if count + bin_count > count_to_keep {
            let excess = count + bin_count - count_to_keep;
            *bin -= excess;
        }
        count += *bin;
    }
}

/// Remove star-contributed bins from the histogram.
///
/// Copies the histogram, trims the brightest 10%, computes stats on the
/// trimmed copy, then zeros out bins at or above `mean + sigma * stddev`
/// in the **original** histogram.
pub fn remove_stars_from_histogram(histogram: &mut [u32; 256], sigma: f64) {
    let pixel_count: u32 = histogram.iter().sum();
    let mut copied_histogram = *histogram;
    trim_histogram(&mut copied_histogram, pixel_count * 9 / 10);
    let stats = stats_for_histogram(&copied_histogram);
    let star_cutoff = (stats.mean + sigma * stats.stddev.max(1.0)) as usize;
    for (h, bin) in histogram.iter_mut().enumerate() {
        if h >= star_cutoff {
            *bin = 0;
        }
    }
}

/// Estimate the dark level from a pixel histogram.
///
/// Returns the mean of the bottom 1% of pixel values.
pub fn estimate_dark_level(pixel_histogram: &[u32; 256], npoints: usize) -> f32 {
    let one_percent = (npoints / 100) as u32;
    if one_percent == 0 {
        for (h, &count) in pixel_histogram.iter().enumerate() {
            if count > 0 {
                return h as f32;
            }
        }
    }

    let mut accum: u64 = 0;
    let mut accum_remaining = one_percent;

    for (h, &bin_count) in pixel_histogram.iter().enumerate() {
        if bin_count == 0 {
            continue;
        }
        if bin_count < accum_remaining {
            accum += (h as u64) * (bin_count as u64);
            accum_remaining -= bin_count;
            continue;
        }
        accum += (h as u64) * (accum_remaining as u64);
        break;
    }

    accum as f32 / one_percent as f32
}

/// Return the histogram bin number N such that the cumulative count at or
/// below N exceeds `fraction` of the total histogram entries.
pub fn get_level_for_fraction(histogram: &[u32; 256], fraction: f64) -> usize {
    assert!(fraction >= 0.0);
    assert!(fraction <= 1.0);
    let total_count: u32 = histogram.iter().sum();
    let goal = (fraction * total_count as f64) as u32;
    let mut count: u32 = 0;
    for (h, &bin_count) in histogram.iter().enumerate() {
        count += bin_count;
        if count >= goal {
            return h;
        }
    }
    unreachable!();
}

/// Return the average of the N highest histogram entry values.
pub fn average_top_values(histogram: &[u32; 256], num_top_values: usize) -> u8 {
    let mut accum_count: usize = 0;
    let mut accum_val: u64 = 0;
    for bin in (1..256).rev() {
        let remain = num_top_values - accum_count;
        if remain == 0 {
            break;
        }
        let count = histogram[bin].min(remain as u32);
        accum_val += (bin as u64) * (count as u64);
        accum_count += count as usize;
    }
    if accum_count == 0 {
        0
    } else {
        std::cmp::max((accum_val / accum_count as u64) as u8, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_empty() {
        let hist = [0_u32; 256];
        let stats = stats_for_histogram(&hist);
        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.median, 0);
        assert_eq!(stats.stddev, 0.0);
    }

    #[test]
    fn test_stats_two_bins() {
        let mut hist = [0_u32; 256];
        hist[10] = 2;
        hist[20] = 2;
        let stats = stats_for_histogram(&hist);
        assert_eq!(stats.mean, 15.0);
        assert_eq!(stats.median, 10);
        assert_eq!(stats.stddev, 5.0);
    }

    #[test]
    fn test_estimate_dark_level_small() {
        let mut hist = [0_u32; 256];
        hist[2] = 1;
        hist[3] = 9;
        // npoints=10 => one_percent=0, falls through to lowest non-zero bin
        assert_eq!(estimate_dark_level(&hist, 10), 2.0);
    }

    #[test]
    fn test_estimate_dark_level() {
        let mut hist = [0_u32; 256];
        hist[2] = 5;
        hist[4] = 15;
        hist[10] = 980;
        // npoints=1000 => one_percent=10, first 5 from bin 2 and next 5 from bin 4
        assert_eq!(estimate_dark_level(&hist, 1000), 3.0);
    }

    #[test]
    fn test_average_top_values() {
        let mut hist = [0_u32; 256];
        hist[100] = 3;
        hist[200] = 2;
        // Average of top 2 values = (200+200)/2 = 200
        assert_eq!(average_top_values(&hist, 2), 200);
        // Average of top 5 values = (200*2 + 100*3)/5 = 140
        assert_eq!(average_top_values(&hist, 5), 140);
    }

    #[test]
    fn test_average_top_values_empty() {
        let hist = [0_u32; 256];
        assert_eq!(average_top_values(&hist, 10), 0);
    }

    #[test]
    fn test_get_level_for_fraction() {
        let mut hist = [0_u32; 256];
        hist[50] = 100;
        hist[100] = 100;
        // total=200, fraction=0.5 => goal=100
        assert_eq!(get_level_for_fraction(&hist, 0.5), 50);
        // fraction=0.51 => goal=102, need bin 100
        assert_eq!(get_level_for_fraction(&hist, 0.51), 100);
    }

    #[test]
    fn test_trim_histogram() {
        let mut hist = [0_u32; 256];
        hist[10] = 50;
        hist[20] = 50;
        trim_histogram(&mut hist, 60);
        assert_eq!(hist[10], 50);
        assert_eq!(hist[20], 10); // trimmed by 40 excess
    }

    #[test]
    fn test_remove_stars_from_histogram() {
        let mut hist = [0_u32; 256];
        // Fill most pixels at low values, a few bright ones
        for _i in 0..10 {
            hist[10] += 10;
        }
        hist[250] = 5;
        remove_stars_from_histogram(&mut hist, 5.0);
        // Bright bins should be zeroed
        assert_eq!(hist[250], 0);
    }
}
