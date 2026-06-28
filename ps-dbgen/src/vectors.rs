use crate::catalog::StarRecord;

/// Compute unit vector (x, y, z) for a star at (ra_rad, dec_rad).
/// x = cos(ra)*cos(dec), y = sin(ra)*cos(dec), z = sin(dec).
pub fn radec_to_unit_vector(ra: f64, dec: f64) -> [f32; 3] {
    let cos_dec = dec.cos();
    [
        (ra.cos() * cos_dec) as f32,
        (ra.sin() * cos_dec) as f32,
        dec.sin() as f32,
    ]
}

/// Compute unit vectors for all stars in the catalog, returning Vec<[f32; 3]>.
pub fn compute_star_vectors(stars: &[StarRecord]) -> Vec<[f32; 3]> {
    stars.iter().map(|s| radec_to_unit_vector(s.ra, s.dec)).collect()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_unit_vector_north_pole() {
        let v = radec_to_unit_vector(0.0, PI / 2.0);
        assert!((v[0] - 0.0).abs() < 1e-7, "x should be ~0, got {}", v[0]);
        assert!((v[1] - 0.0).abs() < 1e-7, "y should be ~0, got {}", v[1]);
        assert!((v[2] - 1.0).abs() < 1e-7, "z should be ~1, got {}", v[2]);
    }

    #[test]
    fn test_unit_vector_equator() {
        let v = radec_to_unit_vector(0.0, 0.0);
        assert!((v[0] - 1.0).abs() < 1e-7, "x should be ~1, got {}", v[0]);
        assert!((v[1] - 0.0).abs() < 1e-7, "y should be ~0, got {}", v[1]);
        assert!((v[2] - 0.0).abs() < 1e-7, "z should be ~0, got {}", v[2]);
    }

    #[test]
    fn test_compute_star_vectors_unit_norm() {
        let stars = vec![
            StarRecord { ra: 0.0, dec: 0.0, mag: 5.0, cat_id: crate::catalog::CatalogId::Hip(1) },
            StarRecord {
                ra: PI / 4.0,
                dec: PI / 6.0,
                mag: 6.0,
                cat_id: crate::catalog::CatalogId::Hip(2),
            },
            StarRecord {
                ra: PI,
                dec: -PI / 3.0,
                mag: 4.5,
                cat_id: crate::catalog::CatalogId::Hip(3),
            },
        ];
        let vectors = compute_star_vectors(&stars);
        assert_eq!(vectors.len(), 3);
        for (i, v) in vectors.iter().enumerate() {
            let mag = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64;
            assert!(
                (mag - 1.0).abs() < 1e-6,
                "star {} vector magnitude {:.10}, expected ~1.0",
                i,
                mag
            );
        }
    }

    #[test]
    fn test_unit_vector_roundtrip() {
        let ra = 1.2_f64;
        let dec = 0.5_f64;
        let v = radec_to_unit_vector(ra, dec);

        // Magnitude should be 1.0
        let mag_sq = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64;
        assert!(
            (mag_sq - 1.0).abs() < 1e-6,
            "magnitude squared {:.10}, expected ~1.0",
            mag_sq
        );

        // Components should match formula
        let exp_x = ra.cos() * dec.cos();
        let exp_y = ra.sin() * dec.cos();
        let exp_z = dec.sin();
        assert!(
            (v[0] as f64 - exp_x).abs() < 1e-6,
            "x: got {}, expected {}",
            v[0],
            exp_x
        );
        assert!(
            (v[1] as f64 - exp_y).abs() < 1e-6,
            "y: got {}, expected {}",
            v[1],
            exp_y
        );
        assert!(
            (v[2] as f64 - exp_z).abs() < 1e-6,
            "z: got {}, expected {}",
            v[2],
            exp_z
        );
    }
}
