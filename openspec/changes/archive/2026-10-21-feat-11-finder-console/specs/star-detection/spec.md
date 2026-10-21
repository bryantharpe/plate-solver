## ADDED Requirements

### Requirement: Gray from color as cedar makes it

The system SHALL provide `star_detection::gray` to turn color into the 8-bit gray that
detection takes, exactly as PIL's `convert('L')` does in cedar's reference pipeline: ITU-R
601-2 luma in 16-bit fixed point, rounded, `(19595·R + 38470·G + 7471·B + 32768) >> 16`.
`pil_luma` SHALL convert one pixel and `from_rgb8` a row-major interleaved RGB buffer, one gray
value per whole pixel, ignoring a trailing partial pixel. Detection itself SHALL still take
gray only. (Ref: `crates/star-detection/src/gray.rs`; tests: `primaries_and_white_match_pil`,
`a_green_pixel_is_601_not_709`, `rgb_rows_convert_pixel_by_pixel`.)

#### Scenario: PIL's values
- **WHEN** pure red, green, blue, white and black are converted
- **THEN** they become 76, 150, 29, 255 and 0, as PIL makes them

#### Scenario: Not Rec. 709
- **WHEN** the pixel (10, 200, 30) is converted
- **THEN** it becomes 124, not the 147 that the `image` crate's Rec. 709 `to_luma8` gives

#### Scenario: A buffer
- **WHEN** `from_rgb8` is given three pixels, or one pixel and a stray byte
- **THEN** it returns one gray value for each whole pixel
