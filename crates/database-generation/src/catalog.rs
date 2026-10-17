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
/// Header: first 28 bytes are the Yale header. `STARN` is the third signed
/// 32-bit integer (offset 8, little-endian). Negative entry count => J2000
/// equinox, otherwise B1950. Each entry is 32 bytes:
///   - bytes 0-3:   catalog number (f32, but stored as integer)
///   - bytes 4-11:  RA  (f64, radians)
///   - bytes 12-19: Dec (f64, radians)
///   - bytes 20-21: spectral type (i16, ignored)
///   - bytes 22-23: magnitude (i16, hundredths)
///   - bytes 24-27: proper motion RA (f32, radians/year)
///   - bytes 28-31: proper motion Dec (f32, radians/year)
///
/// This matches the layout used by the upstream `tetra3` reference.
pub fn parse_bsc5<R: Read>(mut reader: R) -> io::Result<Vec<CatalogEntry>> {
    let mut header = [0u8; 28];
    reader.read_exact(&mut header)?;

    let starn = i32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    let _equinox_is_j2000 = starn < 0;
    let count = starn.unsigned_abs() as usize;

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;

        let id = f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as u32;
        let ra = f64::from_le_bytes(buf[4..12].try_into().unwrap());
        let dec = f64::from_le_bytes(buf[12..20].try_into().unwrap());
        let mag_raw = i16::from_le_bytes([buf[22], buf[23]]);
        let mag = f64::from(mag_raw) / 100.0;
        let pm_ra = f32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        let pm_dec = f32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);

        let mut entry = CatalogEntry::new(ra, dec, mag, CatalogId::Bsc(id));
        // BSC5 proper motions are stored in radians/year.
        if pm_ra != 0.0 || pm_dec != 0.0 {
            entry.pm_ra = Some(f64::from(pm_ra));
            entry.pm_dec = Some(f64::from(pm_dec));
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
        if cols.first().map(String::as_str) == Some("H") {
            return parse_cds_row(cols, |c| Some(CatalogId::Hip(c.get(1)?.parse().ok()?)));
        }
        let hip = cols.first()?.parse::<u32>().ok()?;
        let ra_deg: f64 = cols.get(1)?.parse().ok()?;
        let dec_deg: f64 = cols.get(2)?.parse().ok()?;
        let pm_ra: f64 = cols.get(3)?.parse().ok()?;
        let pm_dec: f64 = cols.get(4)?.parse().ok()?;
        let mag: f64 = cols.get(5)?.parse().ok()?;

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
        if cols.first().map(String::as_str) == Some("T") {
            return parse_cds_row(cols, |c| {
                let mut it = c.get(1)?.split_whitespace().map(|v| v.parse::<u32>().ok());
                Some(CatalogId::Tyc(it.next()??, it.next()??, it.next()??))
            });
        }
        let tyc1 = cols.first()?.parse::<u32>().ok()?;
        let tyc2 = cols.get(1)?.parse::<u32>().ok()?;
        let tyc3 = cols.get(2)?.parse::<u32>().ok()?;
        let ra_deg: f64 = cols.get(3)?.parse().ok()?;
        let dec_deg: f64 = cols.get(4)?.parse().ok()?;
        let pm_ra: f64 = cols.get(5)?.parse().ok()?;
        let pm_dec: f64 = cols.get(6)?.parse().ok()?;
        let mag: f64 = cols.get(7)?.parse().ok()?;

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

/// Parse one row of the real CDS `hip_main.dat` / `tyc_main.dat` (catalog I/239), the
/// files tetra3/cedar read: identifier in field 1, Vmag in 5, RAdeg in 8, DEdeg in 9,
/// pmRA (mas/yr, mu_alpha*cos(delta)) in 12, pmDE in 13. Rows without
/// magnitude or position are skipped; a missing proper motion is left unset.
fn parse_cds_row(
    cols: &[String],
    id: impl Fn(&[String]) -> Option<CatalogId>,
) -> Option<CatalogEntry> {
    let mag: f64 = cols.get(5)?.parse().ok()?;
    let ra_deg: f64 = cols.get(8)?.parse().ok()?;
    let dec_deg: f64 = cols.get(9)?.parse().ok()?;
    if !(mag.is_finite() && ra_deg.is_finite() && dec_deg.is_finite()) {
        return None;
    }
    let mut entry = CatalogEntry::new(ra_deg.to_radians(), dec_deg.to_radians(), mag, id(cols)?);
    entry.pm_ra = cols.get(12).and_then(|v| v.parse().ok());
    entry.pm_dec = cols.get(13).and_then(|v| v.parse().ok());
    if entry.pm_ra.is_none() || entry.pm_dec.is_none() {
        entry.pm_ra = None;
        entry.pm_dec = None;
    }
    Some(entry)
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
