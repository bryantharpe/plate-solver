//! Serialization of a generated pattern catalog into the on-disk `.npz` format.
//!
//! Writes `star_table`, `pattern_catalog`, `pattern_largest_edge`,
//! `pattern_key_hashes`, `star_catalog_IDs`, and `props_packed` using the same
//! layout and dtypes that `pattern-database::load` expects, including legacy
//! field-name fallbacks. The output is a standard zip archive of `.npy` files
//! with `CompressionMethod::Stored`, matching the upstream convention.

use std::io::{self, Write};
use std::path::Path;

use pattern_database::DatabaseProperties;

use crate::catalog::{CatalogEntry, CatalogId};
use crate::pattern_catalog::PatternCatalog;
use math_core::UnitVector;

/// Error serializing a pattern database.
#[derive(Debug)]
pub enum SerializeError {
    /// Failure reading, writing, or zipping the file.
    Io(io::Error),
    /// The catalog or star table is empty.
    Empty,
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializeError::Io(e) => write!(f, "io error: {e}"),
            SerializeError::Empty => write!(f, "cannot serialize an empty database"),
        }
    }
}

impl std::error::Error for SerializeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SerializeError::Io(e) => Some(e),
            SerializeError::Empty => None,
        }
    }
}

impl From<io::Error> for SerializeError {
    fn from(e: io::Error) -> Self {
        SerializeError::Io(e)
    }
}

impl From<zip::result::ZipError> for SerializeError {
    fn from(e: zip::result::ZipError) -> Self {
        SerializeError::Io(io::Error::new(io::ErrorKind::Other, e))
    }
}

/// Serialize a generated database to `path` as an `.npz` archive.
///
/// * `entries` must be the brightness-sorted star catalog used to build the
///   pattern catalog.
/// * `catalog` is the built pattern catalog from `build_pattern_catalog`.
/// * `properties` carries the database metadata record.
///
/// The pattern-catalog dtype is chosen as the smallest unsigned integer that
/// can hold the maximum star index (`u8`, `u16`, or `u32`). The catalog-ID
/// array shape depends on the source catalog flavor: `u16` for BSC, `u32`
/// for Hipparcos, and `(N, 3)` `u16` for Tycho.
pub fn serialize_to_path(
    path: &Path,
    entries: &[CatalogEntry],
    catalog: &PatternCatalog,
    properties: &DatabaseProperties,
) -> Result<(), SerializeError> {
    if entries.is_empty() {
        return Err(SerializeError::Empty);
    }
    if catalog.pattern_catalog.is_empty() {
        return Err(SerializeError::Empty);
    }

    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::new(file));
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let star_table = build_star_table(entries);
    write_npy_array(
        &mut zip,
        "star_table",
        "'<f4'",
        &[entries.len() as u64, 6],
        &star_table,
        options,
    )?;

    let max_index = catalog
        .pattern_catalog
        .iter()
        .flat_map(|row| row.iter().copied())
        .filter(|&i| i != usize::MAX)
        .max()
        .unwrap_or(0);
    let (cat_dtype, cat_data) = encode_pattern_catalog(&catalog.pattern_catalog, max_index);
    let cat_dtype_quoted = format!("'{}'", cat_dtype);
    write_npy_array(
        &mut zip,
        "pattern_catalog",
        &cat_dtype_quoted,
        &[catalog.pattern_catalog.len() as u64, 4],
        &cat_data,
        options,
    )?;

    let edge_data = encode_f16_array(&catalog.pattern_largest_edge);
    write_npy_array(
        &mut zip,
        "pattern_largest_edge",
        "'<f2'",
        &[catalog.pattern_largest_edge.len() as u64],
        &edge_data,
        options,
    )?;

    let hash_data = catalog
        .pattern_key_hashes
        .iter()
        .flat_map(|&v| v.to_le_bytes())
        .collect::<Vec<u8>>();
    write_npy_array(
        &mut zip,
        "pattern_key_hashes",
        "'<u2'",
        &[catalog.pattern_key_hashes.len() as u64],
        &hash_data,
        options,
    )?;

    let (ids_shape, ids_dtype, ids_data) = encode_catalog_ids(entries);
    let ids_dtype_quoted = format!("'{}'", ids_dtype);
    write_npy_array(
        &mut zip,
        "star_catalog_IDs",
        &ids_dtype_quoted,
        &ids_shape,
        &ids_data,
        options,
    )?;

    let props_data = encode_properties(properties);
    write_npy_array(
        &mut zip,
        "props_packed",
        &properties_dtype(),
        &[1],
        &props_data,
        options,
    )?;

    zip.finish()?;
    Ok(())
}

/// Build the `(N, 6)` `f32` star table: `[RA, Dec, x, y, z, mag]`.
fn build_star_table(entries: &[CatalogEntry]) -> Vec<u8> {
    let mut out = Vec::with_capacity(entries.len() * 6 * 4);
    for entry in entries {
        let v = UnitVector::from_radec(entry.ra, entry.dec);
        for &f in &[
            entry.ra as f32,
            entry.dec as f32,
            v.x as f32,
            v.y as f32,
            v.z as f32,
            entry.mag as f32,
        ] {
            out.extend_from_slice(&f.to_le_bytes());
        }
    }
    out
}

/// Encode the pattern catalog into the smallest unsigned dtype that holds
/// `max_index`. Empty slots (`usize::MAX`) are written as all-zeros, which the
/// loader translates back to the `usize::MAX` sentinel.
fn encode_pattern_catalog(rows: &[[usize; 4]], max_index: usize) -> (String, Vec<u8>) {
    if max_index <= u8::MAX as usize {
        let mut data = Vec::with_capacity(rows.len() * 4);
        for row in rows {
            for &idx in row {
                let v = if idx == usize::MAX { 0 } else { idx as u8 };
                data.push(v);
            }
        }
        ("|u1".to_string(), data)
    } else if max_index <= u16::MAX as usize {
        let mut data = Vec::with_capacity(rows.len() * 4 * 2);
        for row in rows {
            for &idx in row {
                let v = if idx == usize::MAX { 0 } else { idx as u16 };
                data.extend_from_slice(&v.to_le_bytes());
            }
        }
        ("<u2".to_string(), data)
    } else {
        let mut data = Vec::with_capacity(rows.len() * 4 * 4);
        for row in rows {
            for &idx in row {
                let v = if idx == usize::MAX { 0 } else { idx as u32 };
                data.extend_from_slice(&v.to_le_bytes());
            }
        }
        ("<u4".to_string(), data)
    }
}

/// Encode `pattern_largest_edge` values as little-endian `float16` bytes.
fn encode_f16_array(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&v| encode_f16(v).to_le_bytes())
        .collect()
}

/// Encode an IEEE 754 binary16 value from `f32`.
fn encode_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = (bits >> 31) & 0x1;
    let exp32 = ((bits >> 23) & 0xff) as i32;
    let mant32 = bits & 0x7fffff;

    let bits16: u32 = if exp32 == 0xff {
        // Inf or NaN.
        (sign << 15) | 0x7c00 | ((mant32 >> 13) & 0x3ff)
    } else if exp32 == 0 {
        // Subnormal f32 stays zero in f16 (too small to represent).
        sign << 15
    } else {
        let exp16 = exp32 - 127 + 15;
        let mut mant16 = (mant32 >> 13) & 0x3ff;
        let round_bit = (mant32 >> 12) & 0x1;
        if round_bit != 0 {
            // Round to nearest even: increment if there are sticky bits below the
            // round bit, or if mant16 is already odd (ties round to even).
            let sticky = mant32 & ((1 << 12) - 1);
            if sticky != 0 || (mant16 & 1) == 1 {
                mant16 += 1;
                // If mant16 overflows, carry into the exponent.
                if mant16 == 0x400 {
                    mant16 = 0;
                    if exp16 + 1 >= 31 {
                        return ((sign << 15) | 0x7c00) as u16;
                    }
                    return ((sign << 15) | (((exp16 + 1) as u32) << 10) | mant16) as u16;
                }
            }
        }
        if exp16 >= 31 {
            // Overflow to infinity.
            (sign << 15) | 0x7c00
        } else if exp16 <= 0 {
            // Underflow to zero (or subnormal, but zero is sufficient here).
            sign << 15
        } else {
            (sign << 15) | ((exp16 as u32) << 10) | mant16
        }
    };
    bits16 as u16
}

/// Encode source catalog IDs according to the catalog flavor.
///
/// Returns `(shape, dtype, data)`. The shape is 1-D for BSC/Hipparcos and
/// 2-D `(N, 3)` for Tycho-2, matching what `pattern-database::load` expects.
fn encode_catalog_ids(entries: &[CatalogEntry]) -> (Vec<u64>, String, Vec<u8>) {
    let first = &entries[0].id;
    match first {
        CatalogId::Bsc(_) => {
            let mut data = Vec::with_capacity(entries.len() * 2);
            for entry in entries {
                let CatalogId::Bsc(id) = entry.id else {
                    panic!("mixed catalog ID flavors in star table");
                };
                data.extend_from_slice(&(id as u16).to_le_bytes());
            }
            (vec![entries.len() as u64], "<u2".to_string(), data)
        }
        CatalogId::Hip(_) => {
            let mut data = Vec::with_capacity(entries.len() * 4);
            for entry in entries {
                let CatalogId::Hip(id) = entry.id else {
                    panic!("mixed catalog ID flavors in star table");
                };
                data.extend_from_slice(&id.to_le_bytes());
            }
            (vec![entries.len() as u64], "<u4".to_string(), data)
        }
        CatalogId::Tyc(_, _, _) => {
            let mut data = Vec::with_capacity(entries.len() * 3 * 2);
            for entry in entries {
                let CatalogId::Tyc(a, b, c) = entry.id else {
                    panic!("mixed catalog ID flavors in star table");
                };
                data.extend_from_slice(&(a as u16).to_le_bytes());
                data.extend_from_slice(&(b as u16).to_le_bytes());
                data.extend_from_slice(&(c as u16).to_le_bytes());
            }
            (vec![entries.len() as u64, 3], "<u2".to_string(), data)
        }
    }
}

/// Return the numpy dtype description for the packed properties record.
fn properties_dtype() -> String {
    "[('pattern_mode', '|S12'), ('hash_table_type', '|S14'), ('pattern_size', '<u2'), \
     ('pattern_bins', '<u2'), ('pattern_max_error', '<f4'), ('max_fov', '<f4'), \
     ('min_fov', '<f4'), ('star_catalog', '|S16'), ('epoch_equinox', '<u2'), \
     ('epoch_proper_motion', '<f4'), ('verification_stars_per_fov', '<u2'), \
     ('star_max_magnitude', '<f4'), ('num_patterns', '<u4')]"
        .to_string()
}

/// Return the numpy dtype description for the packed properties record using
/// legacy field names. Kept for reference; the loader applies fallbacks at
/// read time, so the canonical names are preferred.
#[allow(dead_code)]
fn properties_dtype_legacy() -> String {
    "[('pattern_mode', '|S12'), ('hash_table_type', '|S14'), ('pattern_size', '<u2'), \
     ('pattern_bins', '<u2'), ('pattern_max_error', '<f4'), ('max_fov', '<f4'), \
     ('min_fov', '<f4'), ('star_catalog', '|S16'), ('epoch_equinox', '<u2'), \
     ('epoch_proper_motion', '<f4'), ('catalog_stars_per_fov', '<u2'), \
     ('star_min_magnitude', '<f4'), ('num_patterns', '<u4')]"
        .to_string()
}

/// Encode the properties record into a single structured-array row.
fn encode_properties(props: &DatabaseProperties) -> Vec<u8> {
    let mut out = Vec::with_capacity(74);
    out.extend_from_slice(&pad_bytes(&props.pattern_mode, 12));
    out.extend_from_slice(&pad_bytes(&props.hash_table_type, 14));
    out.extend_from_slice(&props.pattern_size.to_le_bytes());
    out.extend_from_slice(&props.pattern_bins.to_le_bytes());
    out.extend_from_slice(&props.pattern_max_error.to_le_bytes());
    out.extend_from_slice(&props.max_fov.to_le_bytes());
    out.extend_from_slice(&props.min_fov.to_le_bytes());
    out.extend_from_slice(&pad_bytes(&props.star_catalog, 16));
    out.extend_from_slice(&props.epoch_equinox.to_le_bytes());
    out.extend_from_slice(&props.epoch_proper_motion.to_le_bytes());
    out.extend_from_slice(&props.verification_stars_per_fov.to_le_bytes());
    out.extend_from_slice(&props.star_max_magnitude.to_le_bytes());
    out.extend_from_slice(&props.num_patterns.to_le_bytes());
    debug_assert_eq!(out.len(), 74);
    out
}

fn pad_bytes(s: &str, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    let bytes = s.as_bytes();
    let n = bytes.len().min(len);
    out[..n].copy_from_slice(&bytes[..n]);
    out
}

/// Write one `.npy` entry into the in-progress `.npz` archive.
fn write_npy_array<W: Write + io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    dtype: &str,
    shape: &[u64],
    data: &[u8],
    options: zip::write::FileOptions,
) -> io::Result<()> {
    zip.start_file(format!("{name}.npy"), options)?;
    write_npy_header(zip, dtype, shape)?;
    zip.write_all(data)?;
    Ok(())
}

/// Write a version-1 `.npy` header for the given dtype and shape, padded to a
/// 16-byte boundary.
fn write_npy_header<W: Write>(writer: &mut W, dtype: &str, shape: &[u64]) -> io::Result<()> {
    let shape_str = shape
        .iter()
        .map(|&d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let shape_part = if shape.len() == 1 {
        format!("({},)", shape_str)
    } else {
        format!("({})", shape_str)
    };
    let header = format!(
        "{{'descr': {}, 'fortran_order': False, 'shape': {}, }}",
        dtype, shape_part
    );

    // Pad with spaces so that 6 (magic) + 2 (version) + 2 (len) + header.len()
    // is a multiple of 16. The newline is included in the header length.
    let mut header_bytes = header.into_bytes();
    let prefix = 6 + 2 + 2;
    let rem = (prefix + header_bytes.len() + 1) % 16;
    if rem != 0 {
        header_bytes.extend(std::iter::repeat_n(b' ', 16 - rem));
    }
    header_bytes.push(b'\n');

    writer.write_all(&[0x93])?;
    writer.write_all(b"NUMPY")?;
    writer.write_all(&[0x01, 0x00])?;
    writer.write_all(&(header_bytes.len() as u16).to_le_bytes())?;
    writer.write_all(&header_bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_round_trip_simple_values() {
        let cases = [(1.0f32, 0x3C00u16), (-2.0f32, 0xC000), (0.0f32, 0x0000)];
        for (f, expected) in cases {
            assert_eq!(encode_f16(f), expected, "encoding {f}");
        }
    }

    #[test]
    fn f16_overflow_to_infinity() {
        assert_eq!(encode_f16(1e9), 0x7C00);
    }

    #[test]
    fn f16_underflow_to_zero() {
        assert_eq!(encode_f16(1e-9), 0x0000);
    }
}
