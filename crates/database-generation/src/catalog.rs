//! Star catalog parsers for BSC5, HIP, and TYC.

use std::io::{self, Read};

/// Identifies the source of a catalog entry.
#[derive(Clone, Debug, PartialEq)]
pub enum CatalogId {
    /// Bright Star Catalog number.
    Bsc(u32),
    /// Hipparcos number.
    Hip(u32),
    /// Tycho-2 numbers: (TYC1, TYC2, TYC3).
    Tyc(u32, u32, u32),
}

/// A single parsed star row.
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogEntry {
    /// Right ascension in radians.
    pub ra: f64,
    /// Declination in radians.
    pub dec: f64,
    /// Visual magnitude.
    pub mag: f64,
    /// Catalog source identifier.
    pub id: CatalogId,
    /// Proper motion in RA (milliarcseconds/year), catalog form.
    /// For HIP/TYC this is `pmRA = μ_α* · cosδ`.
    pub pm_ra: Option<f64>,
    /// Proper motion in Dec (milliarcseconds/year).
    pub pm_dec: Option<f64>,
}

/// Source catalog flavor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CatalogSource {
    Bsc5,
    Hip,
    Tyc,
}

impl CatalogEntry {
    fn new(ra: f64, dec: f64, mag: f64, id: CatalogId) -> Self {
        Self {
            ra,
            dec,
            mag,
            id,
            pm_ra: None,
            pm_dec: None,
        }
    }
}

/// Parse a BSC5 binary catalog.
///
/// Header: first 28 bytes are the Yale header. `STARN` is a signed 32-bit
/// integer at offset 4 (little-endian per the BSC5 convention used here).
/// Negative entry count => J2000 equinox, otherwise B1950.
/// Each entry is 32 bytes:
///   - bytes 0-3:   catalog number (f32, but stored as integer)
///   - bytes 4-11:  RA  (f64, radians)
///   - bytes 12-19: Dec (f64, radians)
///   - bytes 20-23: magnitude (i32, hundredths)
///   - bytes 24-31: proper motion RA/Dec (i32, milliarcsec/year, each)
///
/// The reference implementation uses this compact layout for testing.
pub fn parse_bsc5<R: Read>(mut reader: R) -> io::Result<Vec<CatalogEntry>> {
    let mut header = [0u8; 28];
    reader.read_exact(&mut header)?;

    let starn = i32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    let _equinox_is_j2000 = starn < 0;
    let count = starn.unsigned_abs() as usize;

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;

        let id = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let ra = f64::from_le_bytes(buf[4..12].try_into().unwrap());
        let dec = f64::from_le_bytes(buf[12..20].try_into().unwrap());
        let mag_raw = i32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
        let mag = mag_raw as f64 / 100.0;
        let pm_ra = i32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]) as f64;
        let pm_dec = i32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]) as f64;

        let mut entry = CatalogEntry::new(ra, dec, mag, CatalogId::Bsc(id));
        // BSC5 proper motions are stored directly in mas/year.
        if pm_ra != 0.0 || pm_dec != 0.0 {
            entry.pm_ra = Some(pm_ra);
            entry.pm_dec = Some(pm_dec);
        }
        entries.push(entry);
    }

    Ok(entries)
}

/// Parse a pipe-delimited Hipparcos catalog.
///
/// Expected columns (0-based):
///   0: HIP number
///   1: RA (degrees, ICRS)
///   2: Dec (degrees, ICRS)
///   3: pmRA (milliarcseconds/year, μ_α* = μ_α·cosδ)
///   4: pmDec (milliarcseconds/year)
///   5: Vmag
///
/// Rows with empty RA/Dec/mag are skipped.
pub fn parse_hip<R: Read>(reader: R) -> io::Result<Vec<CatalogEntry>> {
    parse_pipe_delimited(reader, |cols| {
        let hip = cols[0].parse::<u32>().ok()?;
        let ra_deg: f64 = cols[1].parse().ok()?;
        let dec_deg: f64 = cols[2].parse().ok()?;
        let pm_ra: f64 = cols[3].parse().ok()?;
        let pm_dec: f64 = cols[4].parse().ok()?;
        let mag: f64 = cols[5].parse().ok()?;

        if mag.is_nan() || ra_deg.is_nan() || dec_deg.is_nan() {
            return None;
        }

        let ra = ra_deg.to_radians();
        let dec = dec_deg.to_radians();
        let mut entry = CatalogEntry::new(ra, dec, mag, CatalogId::Hip(hip));
        entry.pm_ra = Some(pm_ra);
        entry.pm_dec = Some(pm_dec);
        Some(entry)
    })
}

/// Parse a pipe-delimited Tycho-2 catalog.
///
/// Expected columns (0-based):
///   0: TYC1
///   1: TYC2
///   2: TYC3
///   3: RA (degrees, ICRS)
///   4: Dec (degrees, ICRS)
///   5: pmRA (milliarcseconds/year)
///   6: pmDec (milliarcseconds/year)
///   7: Vmag
///
/// Rows with empty RA/Dec/mag are skipped.
pub fn parse_tyc<R: Read>(reader: R) -> io::Result<Vec<CatalogEntry>> {
    parse_pipe_delimited(reader, |cols| {
        let tyc1 = cols[0].parse::<u32>().ok()?;
        let tyc2 = cols[1].parse::<u32>().ok()?;
        let tyc3 = cols[2].parse::<u32>().ok()?;
        let ra_deg: f64 = cols[3].parse().ok()?;
        let dec_deg: f64 = cols[4].parse().ok()?;
        let pm_ra: f64 = cols[5].parse().ok()?;
        let pm_dec: f64 = cols[6].parse().ok()?;
        let mag: f64 = cols[7].parse().ok()?;

        if mag.is_nan() || ra_deg.is_nan() || dec_deg.is_nan() {
            return None;
        }

        let ra = ra_deg.to_radians();
        let dec = dec_deg.to_radians();
        let mut entry = CatalogEntry::new(ra, dec, mag, CatalogId::Tyc(tyc1, tyc2, tyc3));
        entry.pm_ra = Some(pm_ra);
        entry.pm_dec = Some(pm_dec);
        Some(entry)
    })
}

fn parse_pipe_delimited<R: Read, F>(reader: R, mut parse_row: F) -> io::Result<Vec<CatalogEntry>>
where
    F: FnMut(&[String]) -> Option<CatalogEntry>,
{
    use std::io::{BufRead, BufReader};
    let mut entries = Vec::new();
    for line in BufReader::new(reader).lines() {
        let line = line?;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<String> = line.split('|').map(|s| s.trim().to_string()).collect();
        if let Some(entry) = parse_row(&cols) {
            entries.push(entry);
        }
    }
    Ok(entries)
}
