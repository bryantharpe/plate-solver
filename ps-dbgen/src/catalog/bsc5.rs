use super::{CatalogId, ParseParams, StarRecord};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

pub fn parse_bsc5<R: Read>(
    reader: &mut R,
    params: &ParseParams,
) -> Result<Vec<StarRecord>, String> {
    // read header (7 x i32)
    let star0 = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let star1 = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let starn = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let stnum = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let mprop = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let nmag = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let nbent = reader
        .read_i32::<LittleEndian>()
        .map_err(|e| e.to_string())?;
    let _ = (star0, star1); // unused

    // sanity (warn, don't abort)
    if stnum != 1 {
        eprintln!("BSC5: STNUM={} (expected 1)", stnum);
    }
    if mprop != 1 {
        eprintln!("BSC5: MPROP={} (expected 1)", mprop);
    }
    if nmag != 1 {
        eprintln!("BSC5: NMAG={} (expected 1)", nmag);
    }
    if nbent != 32 {
        eprintln!("BSC5: NBENT={} (expected 32)", nbent);
    }

    let pm_origin = if starn < 0 { 2000.0_f64 } else { 1950.0_f64 };
    let num_entries = starn.unsigned_abs() as usize;

    let mut records = Vec::with_capacity(num_entries);
    for _ in 0..num_entries {
        let id_raw = reader
            .read_f32::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let ra_raw = reader
            .read_f64::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let dec_raw = reader
            .read_f64::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let _type = reader
            .read_i16::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let mag_raw = reader
            .read_i16::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let ra_pm = reader
            .read_f32::<LittleEndian>()
            .map_err(|e| e.to_string())?;
        let dec_pm = reader
            .read_f32::<LittleEndian>()
            .map_err(|e| e.to_string())?;

        let mag = mag_raw as f64 / 100.0;
        let cos_delta = dec_raw.cos();
        let (mu_alpha, mu_delta_val) = if cos_delta > 0.05 {
            (ra_pm as f64 / cos_delta, dec_pm as f64)
        } else {
            (0.0, 0.0)
        };
        let dt = params.epoch_proper_motion - pm_origin;
        let ra = ra_raw + mu_alpha * dt;
        let dec = dec_raw + mu_delta_val * dt;

        records.push(StarRecord {
            ra,
            dec,
            mag,
            cat_id: CatalogId::Bsc(id_raw as u16),
        });
    }

    // cleanup: drop RA==0 && Dec==0
    records.retain(|r| r.ra != 0.0 || r.dec != 0.0);
    // sort by mag ascending
    records.sort_by(|a, b| {
        a.mag
            .partial_cmp(&b.mag)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(records)
}
