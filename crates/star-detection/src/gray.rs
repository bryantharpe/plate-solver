//! Turning a color frame gray the way cedar's reference pipeline does.
//!
//! cedar-solve's example script decodes a frame with PIL and calls `Image.convert('L')`,
//! which weighs the channels by ITU-R 601-2 luma (0.299, 0.587, 0.114) in 16-bit fixed
//! point. The `image` crate's `to_luma8` uses Rec. 709 weights instead, which moves faint
//! stars enough that a color frame no longer solves to cedar's answer.

/// One RGB pixel to 8-bit gray, exactly as PIL's `convert('L')` computes it.
pub fn pil_luma(r: u8, g: u8, b: u8) -> u8 {
    let l = u32::from(r) * 19_595 + u32::from(g) * 38_470 + u32::from(b) * 7_471 + 0x8000;
    (l >> 16) as u8
}

/// Row-major interleaved RGB, three bytes a pixel, to 8-bit gray as PIL's `convert('L')`
/// makes it. A trailing partial pixel is ignored.
pub fn from_rgb8(rgb: &[u8]) -> Vec<u8> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .map(|&[r, g, b]| pil_luma(r, g, b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_and_white_match_pil() {
        // PIL: Image.new("RGB", (1, 1), c).convert("L").getpixel((0, 0)).
        assert_eq!(pil_luma(255, 0, 0), 76);
        assert_eq!(pil_luma(0, 255, 0), 150);
        assert_eq!(pil_luma(0, 0, 255), 29);
        assert_eq!(pil_luma(255, 255, 255), 255);
        assert_eq!(pil_luma(0, 0, 0), 0);
    }

    #[test]
    fn a_green_pixel_is_601_not_709() {
        // Rec. 709 (the `image` crate's `to_luma8`) makes this 147; PIL makes it 124.
        assert_eq!(pil_luma(10, 200, 30), 124);
    }

    #[test]
    fn rgb_rows_convert_pixel_by_pixel() {
        assert_eq!(
            from_rgb8(&[255, 0, 0, 0, 255, 0, 10, 200, 30]),
            [76, 150, 124]
        );
        assert_eq!(from_rgb8(&[0, 0, 255, 7]), [29]);
        assert!(from_rgb8(&[]).is_empty());
    }
}
