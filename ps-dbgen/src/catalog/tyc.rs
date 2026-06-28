use std::io::{BufRead, BufReader, Read};
use super::{CatalogId, ParseParams, StarRecord};

pub fn parse_tyc<R: Read>(reader: R, params: &ParseParams) -> Result<Vec<StarRecord>, String> {
    let pm_origin = 1991.25_f64;
    let propagate = (params.epoch_proper_motion - pm_origin).abs() > 1e-9;
    let buf = BufReader::new(reader);
    let mut records = Vec::new();

    for line in buf.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() { continue; }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() < 14 { continue; }

        let mag_s  = fields[5].trim();
        let ra_s   = fields[8].trim();
        let dec_s  = fields[9].trim();
        if mag_s.is_empty() || ra_s.is_empty() || dec_s.is_empty() { continue; }

        let pm_ra_s  = fields[12].trim();
        let pm_dec_s = fields[13].trim();
        if propagate && (pm_ra_s.is_empty() || pm_dec_s.is_empty()) { continue; }

        let mag: f64   = mag_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let alpha: f64 = ra_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let delta: f64 = dec_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;

        // TYC ID: "TYC1 TYC2 TYC3" space-separated
        let id_parts: Vec<&str> = fields[1].trim().split_whitespace().collect();
        if id_parts.len() < 3 { continue; }
        let tyc1: u16 = id_parts[0].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let tyc2: u16 = id_parts[1].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let tyc3: u16 = id_parts[2].parse().map_err(|e: std::num::ParseIntError| e.to_string())?;

        let (mu_alpha_cos_delta, mu_delta_raw) = if propagate {
            let pm_ra:  f64 = pm_ra_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
            let pm_dec: f64 = pm_dec_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
            (pm_ra / 1000.0 / 3600.0, pm_dec / 1000.0 / 3600.0)
        } else {
            (0.0, 0.0)
        };

        let cos_delta = delta.to_radians().cos();
        let (mu_alpha, mu_delta_val) = if cos_delta > 0.05 {
            (mu_alpha_cos_delta / cos_delta, mu_delta_raw)
        } else {
            (0.0, 0.0)
        };
        let dt = params.epoch_proper_motion - pm_origin;
        let ra  = (alpha + mu_alpha    * dt).to_radians();
        let dec = (delta + mu_delta_val * dt).to_radians();

        records.push(StarRecord { ra, dec, mag, cat_id: CatalogId::Tyc([tyc1, tyc2, tyc3]) });
    }

    records.retain(|r| r.ra != 0.0 || r.dec != 0.0);
    records.sort_by(|a, b| a.mag.partial_cmp(&b.mag).unwrap_or(std::cmp::Ordering::Equal));
    Ok(records)
}
