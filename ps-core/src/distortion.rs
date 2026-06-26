//! Single-parameter radial lens distortion (doc 02 §4).
//!
//! `k` is the fractional radial displacement at the half-width radius. The
//! model relates undistorted radius `r_u` and distorted radius `r_d` (raw
//! pixels) by
//!
//! ```text
//! r_u = r_d · (1 − k'·r_d²) / (1 − k),   k' = k · (2/width)²
//! ```
//!
//! `k < 0` is barrel, `k > 0` pincushion. Coordinates are `(y, x)` with the
//! image centre at `[height/2, width/2]` (see crate conventions).
//!
//! Computation is in `f64` to honour the crate's f64-compute convention and so
//! that the distort∘undistort round-trip closes to `< 1e-6`. The reference
//! (`tetra3._undistort_centroids` / `_distort_centroids`) casts its working
//! array to `float32` internally, so reference-parity holds to ~1e-4 px (f32
//! quantisation of the goldens), not bit-for-bit.

/// Newton–Raphson tolerance for the forward distortion inversion.
const DISTORT_TOL: f64 = 1e-6;
/// Maximum Newton–Raphson iterations for the forward distortion inversion.
const DISTORT_MAXITER: usize = 30;

/// Undistort centroids in closed form: distorted pixels `[y, x]` → undistorted
/// `[y, x]`.
///
/// `size = (height, width)`. With `k = 0` this is the identity; the image
/// centre is a fixed point for any `k`.
pub fn undistort_centroids(centroids: &[[f64; 2]], size: (usize, usize), k: f64) -> Vec<[f64; 2]> {
    let height = size.0 as f64;
    let width = size.1 as f64;
    let cy0 = height / 2.0;
    let cx0 = width / 2.0;
    let kp = k * (2.0 / width).powi(2);

    centroids
        .iter()
        .map(|&[y, x]| {
            let cy = y - cy0;
            let cx = x - cx0;
            let r = (cy * cy + cx * cx).sqrt();
            let scale = (1.0 - kp * r * r) / (1.0 - k);
            [cy * scale + cy0, cx * scale + cx0]
        })
        .collect()
}

/// Apply the forward distortion `r_u → r_d` by Newton–Raphson inversion of the
/// undistortion model (`tol = 1e-6`, `maxiter = 30`): undistorted pixels
/// `[y, x]` → distorted `[y, x]`.
///
/// `distort_centroids(undistort_centroids(p))` recovers `p` for points inside
/// the invertible radius. The exact image centre (`r = 0`) is a fixed point and
/// is returned unchanged (the reference divides by `r_undist` and yields `NaN`
/// there; that 0/0 singularity is guarded here as the physically-correct
/// identity — no fixture exercises a centre point through `distort`).
pub fn distort_centroids(centroids: &[[f64; 2]], size: (usize, usize), k: f64) -> Vec<[f64; 2]> {
    let height = size.0 as f64;
    let width = size.1 as f64;
    let cy0 = height / 2.0;
    let cx0 = width / 2.0;
    let kp = k * (2.0 / width).powi(2);

    centroids
        .iter()
        .map(|&[y, x]| {
            let cy = y - cy0;
            let cx = x - cx0;
            let r_undist = (cy * cy + cx * cx).sqrt();
            if r_undist == 0.0 {
                return [cy0, cx0];
            }
            let mut r_dist = r_undist;
            for _ in 0..DISTORT_MAXITER {
                let r_undist_est = r_dist * (1.0 - kp * r_dist * r_dist) / (1.0 - k);
                let dru_drd = (1.0 - 2.0 * kp * r_dist) / (1.0 - k);
                let error = r_undist - r_undist_est;
                r_dist += error / dru_drd;
                if error.abs() < DISTORT_TOL {
                    break;
                }
            }
            let factor = r_dist / r_undist;
            [cy * factor + cy0, cx * factor + cx0]
        })
        .collect()
}
