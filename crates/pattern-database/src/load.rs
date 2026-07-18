//! On-disk loader: npz decode and memory-mapped loading.
//!
//! An `.npz` archive is a `.npy`-per-array zip container. `npyz` parses the `.npy` headers
//! (dtype, shape, order) for us; this module supplies the element decoding that `npyz` cannot
//! do on its own — `float16` has no native Rust representation, and the `props_packed` record
//! carries a variable, version-dependent set of fields (see "Legacy fallbacks" below) that the
//! derive-based structured-array reading can't express.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use npyz::{DType, DTypeError, Deserialize, TypeRead};

use math_core::pattern::PATTERN_SIZE;

use crate::format::{CatalogId, PatternDatabase};
use crate::properties::DatabaseProperties;

/// Error loading a pattern database from disk.
#[derive(Debug)]
pub enum LoadError {
    /// Failure reading, unzipping, or otherwise touching the file.
    Io(io::Error),
    /// A required array was absent from the archive.
    MissingArray(String),
    /// The archive was present but its contents didn't match the on-disk format.
    Format(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "io error: {e}"),
            LoadError::MissingArray(name) => write!(f, "missing array '{name}' in database"),
            LoadError::Format(msg) => write!(f, "malformed pattern database: {msg}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(e) => Some(e),
            LoadError::MissingArray(_) | LoadError::Format(_) => None,
        }
    }
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        LoadError::Io(e)
    }
}

/// Load a pattern database from an `.npz` file, reading it into memory.
pub fn load_from_path(path: &Path) -> Result<PatternDatabase, LoadError> {
    let mut archive = npyz::npz::NpzArchive::open(path)?;
    decode_database(&mut archive)
}

/// Load a pattern database via memory-mapping rather than an eager, whole-file read.
///
/// Intended for narrow-FOV / too-big-for-RAM databases: the file's pages are handed to the
/// OS instead of copied up front by a single large `read()`, so the archive can be opened
/// against databases far larger than would comfortably fit in RAM. Pair with a
/// linear-probe `pattern_catalog` (see `DatabaseProperties::linear_probe`) so probe chains
/// stay contiguous within the mapped bytes; quadratic-probe tables assume the table fits in
/// RAM and gain nothing from mapping.
pub fn load_mmap(path: &Path) -> Result<PatternDatabase, LoadError> {
    let file = std::fs::File::open(path)?;
    // Safety: standard file-backed mmap caveat — the file must not be mutated by another
    // process while mapped. The database is generated offline and treated as read-only.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let mut archive = npyz::npz::NpzArchive::new(io::Cursor::new(&mmap[..]))?;
    decode_database(&mut archive)
}

fn decode_database<R: io::Read + io::Seek>(
    archive: &mut npyz::npz::NpzArchive<R>,
) -> Result<PatternDatabase, LoadError> {
    let (star_dtype, star_shape, star_items) = read_raw(archive, "star_table")?;
    let star_ty = parse_type_str(&star_dtype)?;
    if star_ty.kind != 'f' || star_ty.size != 4 {
        return Err(LoadError::Format(format!(
            "star_table: expected f4 elements, got {}",
            star_dtype.descr()
        )));
    }
    if star_shape.len() != 2 || star_shape[1] != 6 {
        return Err(LoadError::Format(format!(
            "star_table: expected shape (N, 6), got {star_shape:?}"
        )));
    }
    let num_stars = star_shape[0] as usize;
    let star_table: Vec<f32> = star_items
        .iter()
        .map(|item| decode_f32(&item.0, star_ty.little))
        .collect();

    let (cat_dtype, cat_shape, cat_items) = read_raw(archive, "pattern_catalog")?;
    let cat_ty = parse_type_str(&cat_dtype)?;
    if cat_ty.kind != 'u' {
        return Err(LoadError::Format(format!(
            "pattern_catalog: expected an unsigned int dtype, got {}",
            cat_dtype.descr()
        )));
    }
    if cat_shape.len() != 2 || cat_shape[1] as usize != PATTERN_SIZE {
        return Err(LoadError::Format(format!(
            "pattern_catalog: expected shape (N, {PATTERN_SIZE}), got {cat_shape:?}"
        )));
    }
    let catalog_length = cat_shape[0] as usize;
    let cat_flat: Vec<u64> = cat_items
        .iter()
        .map(|item| decode_uint(&item.0, cat_ty.little))
        .collect();
    let mut pattern_catalog = Vec::with_capacity(catalog_length);
    for chunk in cat_flat.chunks_exact(PATTERN_SIZE) {
        // Upstream (tetra3) pre-zeros the table and marks a slot occupied only once every
        // column has been written, so an all-zero row is the on-disk "unoccupied" sentinel.
        // lookup.rs's is_empty predicate checks row[0] == usize::MAX, so that's the sentinel
        // we must translate it to here.
        if chunk.iter().all(|&v| v == 0) {
            pattern_catalog.push([usize::MAX; PATTERN_SIZE]);
        } else {
            let row: [usize; PATTERN_SIZE] = std::array::from_fn(|i| chunk[i] as usize);
            pattern_catalog.push(row);
        }
    }

    let (edge_dtype, edge_shape, edge_items) = read_raw(archive, "pattern_largest_edge")?;
    let edge_ty = parse_type_str(&edge_dtype)?;
    if edge_ty.kind != 'f' || edge_ty.size != 2 {
        return Err(LoadError::Format(format!(
            "pattern_largest_edge: expected f2 (float16) elements, got {}",
            edge_dtype.descr()
        )));
    }
    if edge_shape != [catalog_length as u64] {
        return Err(LoadError::Format(format!(
            "pattern_largest_edge: expected shape ({catalog_length}), got {edge_shape:?}"
        )));
    }
    let pattern_largest_edge: Vec<f32> = edge_items
        .iter()
        .map(|item| decode_f16(&item.0, edge_ty.little))
        .collect();

    let (hash_dtype, hash_shape, hash_items) = read_raw(archive, "pattern_key_hashes")?;
    let hash_ty = parse_type_str(&hash_dtype)?;
    if hash_ty.kind != 'u' || hash_ty.size != 2 {
        return Err(LoadError::Format(format!(
            "pattern_key_hashes: expected u2 elements, got {}",
            hash_dtype.descr()
        )));
    }
    if hash_shape != [catalog_length as u64] {
        return Err(LoadError::Format(format!(
            "pattern_key_hashes: expected shape ({catalog_length}), got {hash_shape:?}"
        )));
    }
    let pattern_key_hashes: Vec<u16> = hash_items
        .iter()
        .map(|item| decode_uint(&item.0, hash_ty.little) as u16)
        .collect();

    let (ids_dtype, ids_shape, ids_items) = read_raw(archive, "star_catalog_IDs")?;
    let ids_ty = parse_type_str(&ids_dtype)?;
    if ids_ty.kind != 'u' {
        return Err(LoadError::Format(format!(
            "star_catalog_IDs: expected an unsigned int dtype, got {}",
            ids_dtype.descr()
        )));
    }
    let ids_flat: Vec<u64> = ids_items
        .iter()
        .map(|item| decode_uint(&item.0, ids_ty.little))
        .collect();
    let star_catalog_ids: Vec<CatalogId> = match (ids_shape.as_slice(), ids_ty.size) {
        ([n], 2) if *n as usize == num_stars => ids_flat
            .into_iter()
            .map(|v| CatalogId::Bsc(v as u16))
            .collect(),
        ([n], 4) if *n as usize == num_stars => ids_flat
            .into_iter()
            .map(|v| CatalogId::Hip(v as u32))
            .collect(),
        ([n, 3], 2) if *n as usize == num_stars => ids_flat
            .chunks_exact(3)
            .map(|c| CatalogId::Tyc(c[0] as u16, c[1] as u16, c[2] as u16))
            .collect(),
        _ => {
            return Err(LoadError::Format(format!(
                "star_catalog_IDs: unsupported shape/dtype combination {ids_shape:?} u{}",
                ids_ty.size
            )))
        }
    };

    let (props_dtype, _props_shape, props_items) = read_raw(archive, "props_packed")?;
    let properties = decode_properties(&props_dtype, &props_items, catalog_length)?;

    Ok(PatternDatabase {
        star_table,
        num_stars,
        pattern_catalog,
        pattern_largest_edge,
        pattern_key_hashes,
        star_catalog_ids,
        properties,
    })
}

/// Database properties record with legacy fallbacks applied.
///
/// Older databases used different field names and omitted some fields entirely:
/// `verification_stars_per_fov <- catalog_stars_per_fov`, `star_max_magnitude <-
/// star_min_magnitude`, missing `num_patterns <- pattern_catalog.shape[0] / 2`, and missing
/// `min_fov <- max_fov`.
fn decode_properties(
    dtype: &DType,
    items: &[RawItem],
    catalog_length: usize,
) -> Result<DatabaseProperties, LoadError> {
    let fields = match dtype {
        DType::Record(fields) => fields,
        other => {
            return Err(LoadError::Format(format!(
                "props_packed: expected a record dtype, got {}",
                other.descr()
            )))
        }
    };
    let record = &items
        .first()
        .ok_or_else(|| LoadError::Format("props_packed: array has no records".to_string()))?
        .0;

    let mut values: HashMap<&str, FieldValue> = HashMap::new();
    let mut offset = 0usize;
    for field in fields {
        let size = field.dtype.num_bytes().ok_or_else(|| {
            LoadError::Format(format!(
                "props_packed: field '{}' has a variable-size dtype",
                field.name
            ))
        })?;
        let bytes = record.get(offset..offset + size).ok_or_else(|| {
            LoadError::Format(format!(
                "props_packed: record too short for field '{}'",
                field.name
            ))
        })?;
        values.insert(field.name.as_str(), decode_field(&field.dtype, bytes)?);
        offset += size;
    }

    let missing = |name: &str| LoadError::Format(format!("props_packed: missing field '{name}'"));
    let str_field = |name: &str| -> Option<String> {
        match values.get(name) {
            Some(FieldValue::Str(s)) => Some(s.clone()),
            _ => None,
        }
    };
    let u16_field = |name: &str| -> Option<u16> {
        match values.get(name) {
            Some(FieldValue::U16(v)) => Some(*v),
            _ => None,
        }
    };
    let u32_field = |name: &str| -> Option<u32> {
        match values.get(name) {
            Some(FieldValue::U32(v)) => Some(*v),
            _ => None,
        }
    };
    let f32_field = |name: &str| -> Option<f32> {
        match values.get(name) {
            Some(FieldValue::F32(v)) => Some(*v),
            _ => None,
        }
    };

    let pattern_mode = str_field("pattern_mode").ok_or_else(|| missing("pattern_mode"))?;
    let hash_table_type = str_field("hash_table_type").ok_or_else(|| missing("hash_table_type"))?;
    let pattern_size = u16_field("pattern_size").ok_or_else(|| missing("pattern_size"))?;
    let pattern_bins = u16_field("pattern_bins").ok_or_else(|| missing("pattern_bins"))?;
    let pattern_max_error =
        f32_field("pattern_max_error").ok_or_else(|| missing("pattern_max_error"))?;
    let max_fov = f32_field("max_fov").ok_or_else(|| missing("max_fov"))?;
    let min_fov = f32_field("min_fov").unwrap_or(max_fov);
    let star_catalog = str_field("star_catalog").ok_or_else(|| missing("star_catalog"))?;
    let epoch_equinox = u16_field("epoch_equinox").ok_or_else(|| missing("epoch_equinox"))?;
    let epoch_proper_motion =
        f32_field("epoch_proper_motion").ok_or_else(|| missing("epoch_proper_motion"))?;
    let verification_stars_per_fov = u16_field("verification_stars_per_fov")
        .or_else(|| u16_field("catalog_stars_per_fov"))
        .ok_or_else(|| missing("verification_stars_per_fov"))?;
    let star_max_magnitude = f32_field("star_max_magnitude")
        .or_else(|| f32_field("star_min_magnitude"))
        .ok_or_else(|| missing("star_max_magnitude"))?;
    let num_patterns = u32_field("num_patterns").unwrap_or((catalog_length / 2) as u32);

    Ok(DatabaseProperties {
        pattern_mode,
        hash_table_type,
        pattern_size,
        pattern_bins,
        pattern_max_error,
        max_fov,
        min_fov,
        star_catalog,
        epoch_equinox,
        epoch_proper_motion,
        verification_stars_per_fov,
        star_max_magnitude,
        num_patterns,
    })
}

enum FieldValue {
    Str(String),
    U16(u16),
    U32(u32),
    F32(f32),
}

fn decode_field(dtype: &DType, bytes: &[u8]) -> Result<FieldValue, LoadError> {
    let ty = parse_type_str(dtype)?;
    match (ty.kind, ty.size) {
        ('S', _) => Ok(FieldValue::Str(decode_bytestr(bytes))),
        ('u', 2) => Ok(FieldValue::U16(decode_uint(bytes, ty.little) as u16)),
        ('u', 4) => Ok(FieldValue::U32(decode_uint(bytes, ty.little) as u32)),
        ('f', 4) => Ok(FieldValue::F32(decode_f32(bytes, ty.little))),
        _ => Err(LoadError::Format(format!(
            "unsupported properties field type '{}{}{}'",
            if ty.little { '<' } else { '>' },
            ty.kind,
            ty.size
        ))),
    }
}

/// Read every element of an on-disk array as raw bytes, deferring interpretation.
///
/// `npyz` parses the `.npy` header (dtype, shape) for us; we decode elements ourselves
/// because the format uses a `float16` array and a properties record whose field set varies
/// by database version (see [`decode_properties`]), neither of which `npyz`'s typed readers
/// support directly.
fn read_raw<R: io::Read + io::Seek>(
    archive: &mut npyz::npz::NpzArchive<R>,
    name: &str,
) -> Result<(DType, Vec<u64>, Vec<RawItem>), LoadError> {
    let npy = archive
        .by_name(name)?
        .ok_or_else(|| LoadError::MissingArray(name.to_string()))?;
    let dtype = npy.dtype();
    let shape = npy.shape().to_vec();
    let items = npy.into_vec::<RawItem>()?;
    Ok((dtype, shape, items))
}

/// One array element's raw, undecoded bytes.
struct RawItem(Vec<u8>);

struct RawItemReader {
    size: usize,
}

impl TypeRead for RawItemReader {
    type Value = RawItem;

    fn read_one<R: io::Read>(&self, mut reader: R) -> io::Result<RawItem> {
        let mut buf = vec![0u8; self.size];
        reader.read_exact(&mut buf)?;
        Ok(RawItem(buf))
    }
}

impl Deserialize for RawItem {
    type TypeReader = RawItemReader;

    fn reader(dtype: &DType) -> Result<Self::TypeReader, DTypeError> {
        let size = dtype
            .num_bytes()
            .ok_or_else(|| DTypeError::custom("variable-size dtypes are not supported"))?;
        Ok(RawItemReader { size })
    }
}

/// A parsed numpy type-string, e.g. `<f4` decomposed into endianness/kind/size.
///
/// `npyz::TypeStr`'s fields are crate-private, so its `Display` impl (the numpy type-string
/// form) is the only way to inspect a dtype from outside `npyz`.
struct ScalarType {
    little: bool,
    kind: char,
    size: usize,
}

fn parse_type_str(dtype: &DType) -> Result<ScalarType, LoadError> {
    let DType::Plain(ts) = dtype else {
        return Err(LoadError::Format(format!(
            "expected a scalar dtype, got {}",
            dtype.descr()
        )));
    };
    let s = ts.to_string();
    let mut chars = s.chars();
    let endian_ch = chars
        .next()
        .ok_or_else(|| LoadError::Format("empty type string".to_string()))?;
    let kind = chars
        .next()
        .ok_or_else(|| LoadError::Format(format!("bad type string '{s}'")))?;
    let size: usize = chars
        .as_str()
        .parse()
        .map_err(|_| LoadError::Format(format!("bad type string '{s}'")))?;
    let little = match endian_ch {
        '<' | '|' => true,
        '>' => false,
        _ => return Err(LoadError::Format(format!("bad type string '{s}'"))),
    };
    Ok(ScalarType { little, kind, size })
}

fn decode_uint(bytes: &[u8], little: bool) -> u64 {
    let mut buf = [0u8; 8];
    if little {
        buf[..bytes.len()].copy_from_slice(bytes);
        u64::from_le_bytes(buf)
    } else {
        buf[8 - bytes.len()..].copy_from_slice(bytes);
        u64::from_be_bytes(buf)
    }
}

fn decode_f32(bytes: &[u8], little: bool) -> f32 {
    let arr: [u8; 4] = bytes.try_into().expect("f32 field must be 4 bytes");
    if little {
        f32::from_le_bytes(arr)
    } else {
        f32::from_be_bytes(arr)
    }
}

/// Decode an IEEE 754 binary16 value to `f32`.
fn decode_f16(bytes: &[u8], little: bool) -> f32 {
    let arr: [u8; 2] = bytes.try_into().expect("f16 field must be 2 bytes");
    let bits = if little {
        u16::from_le_bytes(arr)
    } else {
        u16::from_be_bytes(arr)
    };

    let sign = (bits & 0x8000) as u32;
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = (bits & 0x3ff) as u32;

    let bits32 = if exponent == 0 {
        if mantissa == 0 {
            sign << 16
        } else {
            // Subnormal half-float: normalize the mantissa into a normal f32.
            let mut shift = 0u32;
            let mut m = mantissa;
            while m & 0x400 == 0 {
                m <<= 1;
                shift += 1;
            }
            m &= 0x3ff;
            let exp32 = 127 - 15 - shift;
            (sign << 16) | (exp32 << 23) | (m << 13)
        }
    } else if exponent == 0x1f {
        // Inf or NaN.
        (sign << 16) | (0xff << 23) | (mantissa << 13)
    } else {
        let exp32 = exponent as u32 + (127 - 15);
        (sign << 16) | (exp32 << 23) | (mantissa << 13)
    };
    f32::from_bits(bits32)
}

/// Decode a numpy fixed-size byte string (`S` dtype), trimming the zero padding.
fn decode_bytestr(bytes: &[u8]) -> String {
    let end = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_bit_patterns() {
        // 0x3C00 = 1.0, 0xC000 = -2.0, 0x0000 = 0.0, 0x7C00 = +inf.
        assert_eq!(decode_f16(&0x3C00u16.to_le_bytes(), true), 1.0);
        assert_eq!(decode_f16(&0xC000u16.to_le_bytes(), true), -2.0);
        assert_eq!(decode_f16(&0x0000u16.to_le_bytes(), true), 0.0);
        assert!(decode_f16(&0x7C00u16.to_le_bytes(), true).is_infinite());
        assert_eq!(decode_f16(&0x3C00u16.to_be_bytes(), false), 1.0);
    }

    #[test]
    fn bytestr_trims_zero_padding() {
        assert_eq!(decode_bytestr(b"edge_ratio\0\0"), "edge_ratio");
    }
}
