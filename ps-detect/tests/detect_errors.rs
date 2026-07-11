//! Error-return tests for `get_stars_from_image`: invalid arguments must come
//! back as `Err`, never as a panic, since these values are handler-reachable
//! (a gRPC/HTTP client picks `binning` and `return_binned_image`).

use ps_detect::{as_view, get_stars_from_image, GrayImage};

#[test]
fn get_stars_invalid_binning_returns_err() {
    let image = GrayImage::from_raw(10, 10, vec![50u8; 100]).unwrap();
    let result = get_stars_from_image(&as_view(&image), 1.0, 4.0, false, 3, false, false);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid binning"));
}

#[test]
fn get_stars_binning_1_return_binned_image_returns_err() {
    let image = GrayImage::from_raw(10, 10, vec![50u8; 100]).unwrap();
    let result = get_stars_from_image(&as_view(&image), 1.0, 4.0, false, 1, false, true);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("return_binned_image"));
}

#[test]
fn get_stars_valid_params_return_ok() {
    let image = GrayImage::from_raw(10, 10, vec![50u8; 100]).unwrap();
    let result = get_stars_from_image(&as_view(&image), 1.0, 4.0, false, 1, false, false);
    assert!(result.is_ok());
    let (stars, _, binned, _) = result.unwrap();
    assert!(stars.is_empty()); // flat image → no stars
    assert!(binned.is_none()); // binning==1 → no binned image
}

#[test]
fn get_stars_flat_image_no_panic() {
    // Flat-noise image (all pixels equal) should not panic.
    let image = GrayImage::from_raw(20, 20, vec![128u8; 400]).unwrap();
    let result = get_stars_from_image(&as_view(&image), 1.0, 1.0, false, 1, false, false);
    assert!(result.is_ok());
    let (stars, _, _, _) = result.unwrap();
    assert!(stars.is_empty());
}
