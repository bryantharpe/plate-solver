//! Serialization of a generated pattern catalog to the `.npz` on-disk format.
//!
//! Writes the six arrays and packed properties expected by `pattern-database`:
//! `star_table`, `pattern_catalog`, `pattern_largest_edge`, `pattern_key_hashes`,
//! `star_catalog_IDs`, and `props_packed`. The pattern catalog dtype is the smallest
//! unsigned integer that can hold the maximum star index (u8/u16/u32).

use std::io;
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use half::f16;
use npyz::{
    npz::NpzWriter, AutoSerialize, Deserialize, DType, Field, Serialize, TypeRead, TypeWrite,
    WriterBuilder,
};

use crate::catalog::{CatalogEntry, CatalogId};
use crate::pattern_catalog::PatternCatalog;
use pattern_database::{DatabaseProperties, PropsPacked};

/// Error type for serialization failures.
#[derive(Debug)]
pub struct SerializeError {
    inner: io::Error,
}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for SerializeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl From<io::Error> for SerializeError {
    fn from(inner: io::Error) -> Self {
        Self { inner }
    }
}

/// Write a generated database to `path` in the `.npz` format.
///
/// `entries` must be the cleaned, brightness-sorted star catalog used to build the
/// pattern catalog. `properties` carries the database metadata record.
pub fn write_database(
    path: &Path,
    catalog: &PatternCatalog,
    entries: &[CatalogEntry],
    properties: &DatabaseProperties,
) -> Result<(), SerializeError> {
    let file = std::fs::File::create(path)?;
    let mut npz = NpzWriter::new(std::io::BufWriter::new(file));

    write_star_table(&mut npz, entries)?;
    write_pattern_catalog(&mut npz, catalog)?;
    write_pattern_largest_edge(&mut npz, catalog)?;
    write_pattern_key_hashes(&mut npz, catalog)?;
    write_star_catalog_ids(&mut npz, entries)?;
    write_properties(&mut npz, properties)?;

    Ok(())
}

fn write_star_table<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    entries: &[CatalogEntry],
) -> io::Result<()> {
    let mut star_table: Vec<f32> = Vec::with_capacity(entries.len() * 6);
    for entry in entries {
        let v = math_core::UnitVector::from_radec(entry.ra, entry.dec);
        star_table.push(entry.ra as f32);
        star_table.push(entry.dec as f32);
        star_table.push(v.x as f32);
        star_table.push(v.y as f32);
        star_table.push(v.z as f32);
        star_table.push(entry.mag as f32);
    }

    npz.array::<f32>("star_table", zip::write::FileOptions::default())?
        .default_dtype()
        .shape(&[entries.len() as u64, 6])
        .begin_nd()?
        .extend(star_table)?;
    Ok(())
}

fn write_pattern_catalog<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    catalog: &PatternCatalog,
) -> io::Result<()> {
    let max_index = catalog
        .pattern_catalog
        .iter()
        .flat_map(|row| row.iter().copied())
        .max()
        .unwrap_or(0);

    if max_index <= u8::MAX as usize {
        write_pattern_catalog_typed::<u8, W>(npz, catalog, u8::MAX)
    } else if max_index <= u16::MAX as usize {
        write_pattern_catalog_typed::<u16, W>(npz, catalog, u16::MAX)
    } else {
        write_pattern_catalog_typed::<u32, W>(npz, catalog, u32::MAX)
    }
}

fn write_pattern_catalog_typed<T, W>(
    npz: &mut NpzWriter<W>,
    catalog: &PatternCatalog,
    empty_sentinel: T,
) -> io::Result<()>
where
    T: npyz::AutoSerialize + TryFrom<usize> + Copy,
    <T as TryFrom<usize>>::Error: std::fmt::Debug,
    W: io::Write + io::Seek,
{
    let mut flat: Vec<T> = Vec::with_capacity(catalog.table_size * 4);
    for row in &catalog.pattern_catalog {
        for &idx in row {
            let value = if idx == usize::MAX {
                empty_sentinel
            } else {
                T::try_from(idx).expect("index fits in selected dtype")
            };
            flat.push(value);
        }
    }

    npz.array::<T>("pattern_catalog", zip::write::FileOptions::default())?
        .default_dtype()
        .shape(&[catalog.table_size as u64, 4])
        .begin_nd()?
        .extend(flat)?;
    Ok(())
}

fn write_pattern_largest_edge<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    catalog: &PatternCatalog,
) -> io::Result<()> {
    let edges: Vec<F16> = catalog
        .pattern_largest_edge
        .iter()
        .map(|&e| F16(f16::from_f32(e).to_bits()))
        .collect();

    npz.array::<F16>("pattern_largest_edge", zip::write::FileOptions::default())?
        .default_dtype()
        .shape(&[catalog.table_size as u64])
        .begin_nd()?
        .extend(edges)?;
    Ok(())
}

fn write_pattern_key_hashes<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    catalog: &PatternCatalog,
) -> io::Result<()> {
    npz.array::<u16>("pattern_key_hashes", zip::write::FileOptions::default())?
        .default_dtype()
        .shape(&[catalog.table_size as u64])
        .begin_nd()?
        .extend(catalog.pattern_key_hashes.clone())?;
    Ok(())
}

fn write_star_catalog_ids<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    entries: &[CatalogEntry],
) -> io::Result<()> {
    // Detect whether any entry is Tycho. If so, write a (N,3) u32 array.
    let any_tyc = entries.iter().any(|e| matches!(e.id, CatalogId::Tyc(_, _, _)));

    if any_tyc {
        let mut ids: Vec<u32> = Vec::with_capacity(entries.len() * 3);
        for entry in entries {
            let (a, b, c) = match entry.id {
                CatalogId::Tyc(t1, t2, t3) => (t1, t2, t3),
                CatalogId::Hip(h) => (h, 0, 0),
                CatalogId::Bsc(b) => (b, 0, 0),
            };
            ids.push(a);
            ids.push(b);
            ids.push(c);
        }
        npz.array::<u32>("star_catalog_IDs", zip::write::FileOptions::default())?
            .default_dtype()
            .shape(&[entries.len() as u64, 3])
            .begin_nd()?
            .extend(ids)?;
    } else {
        let mut ids: Vec<u32> = Vec::with_capacity(entries.len());
        for entry in entries {
            ids.push(match entry.id {
                CatalogId::Hip(h) => h,
                CatalogId::Bsc(b) => b,
                CatalogId::Tyc(t1, _, _) => t1,
            });
        }
        npz.array::<u32>("star_catalog_IDs", zip::write::FileOptions::default())?
            .default_dtype()
            .shape(&[entries.len() as u64])
            .begin_nd()?
            .extend(ids)?;
    }
    Ok(())
}

fn write_properties<W: io::Write + io::Seek>(
    npz: &mut NpzWriter<W>,
    properties: &DatabaseProperties,
) -> io::Result<()> {
    let props = PropsPacked {
        pattern_mode: pad_bytes(properties.pattern_mode.as_bytes(), MAX_STRING_LEN),
        hash_table_type: pad_bytes(properties.hash_table_type.as_bytes(), MAX_STRING_LEN),
        pattern_size: properties.pattern_size,
        pattern_bins: properties.pattern_bins,
        pattern_max_error: properties.pattern_max_error,
        max_fov: properties.max_fov,
        min_fov: properties.min_fov,
        star_catalog: pad_bytes(properties.star_catalog.as_bytes(), MAX_STRING_LEN),
        epoch_equinox: properties.epoch_equinox,
        epoch_proper_motion: properties.epoch_proper_motion,
        verification_stars_per_fov: properties.verification_stars_per_fov,
        star_max_magnitude: properties.star_max_magnitude,
        num_patterns: properties.num_patterns,
    };

    npz.array::<PropsPacked>("props_packed", zip::write::FileOptions::default())?
        .dtype(props_packed_dtype())
        .shape(&[1])
        .begin_nd()?
        .push(&props)?;
    Ok(())
}

const MAX_STRING_LEN: usize = 64;

fn pad_bytes(src: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(src);
    out.resize(len, 0);
    out
}

fn props_packed_dtype() -> DType {
    let string_field = |name: &str| Field {
        name: name.to_string(),
        dtype: DType::new_scalar(format!("|S{MAX_STRING_LEN}").parse().unwrap()),
    };
    DType::Record(vec![
        string_field("pattern_mode"),
        string_field("hash_table_type"),
        plain_field("pattern_size", "<u2"),
        plain_field("pattern_bins", "<u2"),
        plain_field("pattern_max_error", "<f4"),
        plain_field("max_fov", "<f4"),
        plain_field("min_fov", "<f4"),
        string_field("star_catalog"),
        plain_field("epoch_equinox", "<u2"),
        plain_field("epoch_proper_motion", "<f4"),
        plain_field("verification_stars_per_fov", "<u2"),
        plain_field("star_max_magnitude", "<f4"),
        plain_field("num_patterns", "<u4"),
    ])
}

fn plain_field(name: &str, dtype: &str) -> Field {
    Field {
        name: name.to_string(),
        dtype: DType::new_scalar(dtype.parse().unwrap()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct F16(u16);

struct F16Reader;
impl TypeRead for F16Reader {
    type Value = F16;
    fn read_one<R: io::Read>(&self, mut reader: R) -> io::Result<F16> {
        let bits = reader.read_u16::<LittleEndian>()?;
        Ok(F16(bits))
    }
}

impl Deserialize for F16 {
    type TypeReader = F16Reader;
    fn reader(_dtype: &DType) -> Result<Self::TypeReader, npyz::DTypeError> {
        Ok(F16Reader)
    }
}

struct F16Writer;
impl TypeWrite for F16Writer {
    type Value = F16;
    fn write_one<W: io::Write>(&self, mut writer: W, value: &F16) -> io::Result<()> {
        writer.write_u16::<LittleEndian>(value.0)
    }
}

impl Serialize for F16 {
    type TypeWriter = F16Writer;
    fn writer(_dtype: &DType) -> Result<Self::TypeWriter, npyz::DTypeError> {
        Ok(F16Writer)
    }
}

impl AutoSerialize for F16 {
    fn default_dtype() -> DType {
        DType::new_scalar("<f2".parse().unwrap())
    }
}
