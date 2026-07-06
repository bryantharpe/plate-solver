//! End-to-end star detection: `get_stars_from_image`.
//!
//! Wires together binning, 1-D scanning, hot-pixel rejection, blob formation,
//! and 2-D gating to produce a brightest-first list of star centroids.

use crate::binning::{bin_and_histogram_2x2, Binned2x2Result};
use crate::blob::{form_blobs_from_candidates, gate_star_2d};
use crate::gate::{all_bright_are_hot, scan_image_for_candidates, CandidateFrom1D};
use crate::noise::estimate_noise_from_image;
use crate::{GrayImage, GrayImageView, as_view};
use crate::StarDescription;
use crate::NOISE_FLOOR;
use std::cmp;

pub fn get_stars_from_image(
    image: &GrayImageView<'_>,
    noise_estimate: f64,
    sigma: f64,
    normalize_rows: bool,
    binning: u32,
    detect_hot_pixels: bool,
    return_binned_image: bool,
) -> Result<
    (Vec<StarDescription>, /*hot_pixel_count*/ i32, Option<GrayImage>, [u32; 256]),
    Box<dyn std::error::Error>,
> {
    match binning {
        1 => {
            if return_binned_image {
                return Err("cannot 'return_binned_image' when binning==1".into());
            }
        }
        2 | 4 | 8 => {}
        _ => {
            return Err(format!(
                "Invalid binning argument {}, must be 1, 2, 4, or 8",
                binning
            )
            .into());
        }
    }

    let noise_estimate = f64::max(noise_estimate, NOISE_FLOOR);

    let mut stars: Vec<StarDescription> = Vec::new();
    let mut hot_pixel_count: i32 = 0;

    let max_size = image.width() / 100;

    if binning == 1 {
        let candidates_1d = scan_image_for_candidates(image, noise_estimate, sigma);
        let sigma_noise_2 = cmp::max((2.0 * sigma * noise_estimate + 0.5) as i16, 2);
        let mut filtered_candidates: Vec<CandidateFrom1D> = Vec::new();
        let mut max_y = 0usize;

        for cand in candidates_1d {
            if !detect_hot_pixels {
                max_y = max_y.max(cand.y as usize);
                filtered_candidates.push(cand);
            } else if all_bright_are_hot(image, cand.x, cand.y, binning, sigma_noise_2) {
                hot_pixel_count += 1;
            } else {
                max_y = max_y.max(cand.y as usize);
                filtered_candidates.push(cand);
            }
        }

        for blob in form_blobs_from_candidates(filtered_candidates, max_y) {
            if let Some(star) = gate_star_2d(
                &blob,
                image,
                /*higher_res_image=*/ image,
                binning,
                noise_estimate,
                sigma,
                max_size,
                max_size,
            ) {
                stars.push(star);
            }
        }

        // Sort by brightness estimate, brightest first.
        stars.sort_by(|a, b| {
            b.brightness
                .partial_cmp(&a.brightness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        return Ok((stars, hot_pixel_count, None, [0u32; 256]));
    }

    // We are binning by 2x, 4x, or 8x.
    let Binned2x2Result {
        binned: binned_2x,
        histogram: histogram_2x,
    } = bin_and_histogram_2x2(image, normalize_rows);

    // Determine which binned images to use for detection and centroiding.
    let binned_4x;
    let binned_8x;

    let (detect_image_ref, higher_res_image_ref) = if binning == 2 {
        (&binned_2x, None)
    } else {
        let Binned2x2Result {
            binned: b4x,
            histogram: _,
        } = bin_and_histogram_2x2(&as_view(&binned_2x), /*normalize_rows=*/ false);
        binned_4x = b4x;
        if binning == 4 {
            (&binned_4x, Some(&binned_2x))
        } else {
            // binning == 8
            let Binned2x2Result {
                binned: b8x,
                histogram: _,
            } = bin_and_histogram_2x2(&as_view(&binned_4x), /*normalize_rows=*/ false);
            binned_8x = b8x;
            (&binned_8x, Some(&binned_4x))
        }
    };

    let detect_image_view = as_view(detect_image_ref);
    let higher_res_image_view: GrayImageView<'_> = match higher_res_image_ref {
        Some(img) => as_view(img),
        None => image.clone(), // binning==2: higher_res_image is the original full-res input
    };

    let noise_estimate_binned = f64::max(estimate_noise_from_image(&detect_image_view), NOISE_FLOOR);

    let sigma_noise_2 = cmp::max((2.0 * sigma * noise_estimate_binned + 0.5) as i16, 2);

    let candidates_1d = scan_image_for_candidates(&detect_image_view, noise_estimate_binned, sigma);
    let mut filtered_candidates: Vec<CandidateFrom1D> = Vec::new();
    let mut max_y = 0usize;

    for cand in candidates_1d {
        if !detect_hot_pixels {
            max_y = max_y.max(cand.y as usize);
            filtered_candidates.push(cand);
        } else if all_bright_are_hot(image, cand.x, cand.y, binning, sigma_noise_2) {
            hot_pixel_count += 1;
        } else {
            max_y = max_y.max(cand.y as usize);
            filtered_candidates.push(cand);
        }
    }

    for blob in form_blobs_from_candidates(filtered_candidates, max_y) {
        if let Some(star) = gate_star_2d(
            &blob,
            &detect_image_view,
            &higher_res_image_view,
            binning,
            noise_estimate_binned,
            sigma,
            max_size / binning + 1,
            max_size / binning + 1,
        ) {
            stars.push(star);
        }
    }

    // Sort by brightness estimate, brightest first.
    stars.sort_by(|a, b| {
        b.brightness
            .partial_cmp(&a.brightness)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok((stars, hot_pixel_count, Some(binned_2x), histogram_2x))
}
