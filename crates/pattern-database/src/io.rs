use crate::format::{CatalogIndex, PatternDatabase, PatternRow, StarRow};
use crate::properties::DatabaseProperties;
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use half::f16;
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use thiserror::Error;

const MAGIC: &[u8; 8] = b"PSPDB\x01\x00\x00";

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid magic bytes")]
    InvalidMagic,
    #[error("unsupported catalog index width: {0}")]
    UnsupportedIndexWidth(u8),
}

/// Write a pattern database to `path`.
pub fn write<P: AsRef<Path>>(db: &PatternDatabase, path: P) -> Result<(), DatabaseError> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    w.write_all(MAGIC)?;

    let props_json = serde_json::to_vec(&db.properties)?;
    w.write_u64::<LittleEndian>(props_json.len() as u64)?;
    w.write_all(&props_json)?;

    let n_stars = db.star_table.len() as u64;
    let catalog_len = db.pattern_catalog.len() as u64;
    let id_width = catalog_index_width(db.star_table.len());

    w.write_u64::<LittleEndian>(n_stars)?;
    w.write_u64::<LittleEndian>(catalog_len)?;
    w.write_u8(id_width)?;

    for row in &db.star_table {
        w.write_f32::<LittleEndian>(row.ra)?;
        w.write_f32::<LittleEndian>(row.dec)?;
        w.write_f32::<LittleEndian>(row.x)?;
        w.write_f32::<LittleEndian>(row.y)?;
        w.write_f32::<LittleEndian>(row.z)?;
        w.write_f32::<LittleEndian>(row.mag)?;
    }

    for row in &db.pattern_catalog {
        for &idx in &row.0 {
            write_catalog_index(&mut w, idx, id_width)?;
        }
    }

    for &edge in &db.pattern_largest_edge {
        let bits = f16::from_f32(edge).to_bits();
        w.write_u16::<LittleEndian>(bits)?;
    }

    for &hash in &db.pattern_key_hashes {
        w.write_u16::<LittleEndian>(hash)?;
    }

    let id_shape = if db.star_catalog_ids.first().map(|v| v.len()).unwrap_or(0) == 3 {
        3u8
    } else {
        1u8
    };
    w.write_u8(id_shape)?;
    for id in &db.star_catalog_ids {
        w.write_all(id)?;
    }

    w.flush()?;
    Ok(())
}

/// Load a pattern database from `path`.
pub fn load<P: AsRef<Path>>(path: P) -> Result<PatternDatabase, DatabaseError> {
    let file = File::open(path)?;
    let mut r = BufReader::new(file);

    let mut magic = [0u8; 8];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(DatabaseError::InvalidMagic);
    }

    let props_len = r.read_u64::<LittleEndian>()?;
    let mut props_json = vec![0u8; props_len as usize];
    r.read_exact(&mut props_json)?;
    let mut properties: DatabaseProperties = serde_json::from_slice(&props_json)?;

    let n_stars = r.read_u64::<LittleEndian>()? as usize;
    let catalog_len = r.read_u64::<LittleEndian>()? as usize;
    let id_width = r.read_u8()?;

    let mut star_table = Vec::with_capacity(n_stars);
    for _ in 0..n_stars {
        star_table.push(StarRow {
            ra: r.read_f32::<LittleEndian>()?,
            dec: r.read_f32::<LittleEndian>()?,
            x: r.read_f32::<LittleEndian>()?,
            y: r.read_f32::<LittleEndian>()?,
            z: r.read_f32::<LittleEndian>()?,
            mag: r.read_f32::<LittleEndian>()?,
        });
    }

    let mut pattern_catalog = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        let a = read_catalog_index(&mut r, id_width)?;
        let b = read_catalog_index(&mut r, id_width)?;
        let c = read_catalog_index(&mut r, id_width)?;
        let d = read_catalog_index(&mut r, id_width)?;
        pattern_catalog.push(PatternRow([a, b, c, d]));
    }

    let mut pattern_largest_edge = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        let bits = r.read_u16::<LittleEndian>()?;
        pattern_largest_edge.push(f16::from_bits(bits).to_f32());
    }

    let mut pattern_key_hashes = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        pattern_key_hashes.push(r.read_u16::<LittleEndian>()?);
    }

    let id_shape = r.read_u8()?;
    let id_bytes = id_shape as usize;
    let mut star_catalog_ids = Vec::with_capacity(n_stars);
    for _ in 0..n_stars {
        let mut id = vec![0u8; id_bytes];
        r.read_exact(&mut id)?;
        star_catalog_ids.push(id);
    }

    properties.apply_legacy_fallbacks(catalog_len);

    Ok(PatternDatabase {
        properties,
        star_table,
        pattern_catalog,
        pattern_largest_edge,
        pattern_key_hashes,
        star_catalog_ids,
    })
}

/// Load a pattern database from `path` via memory mapping.
///
/// This is the narrow-FOV / too-big-for-RAM path required by the spec. The file
/// is mapped read-only; the returned `PatternDatabase` owns copies of the small
/// metadata arrays (`properties`, `star_table`, `pattern_key_hashes`,
/// `pattern_largest_edge`, `star_catalog_ids`) while the `pattern_catalog` is
/// backed by the mapped bytes so linear-probe chains stay contiguous on disk.
pub fn load_mmap<P: AsRef<Path>>(path: P) -> Result<PatternDatabase, DatabaseError> {
    let file = File::open(path)?;
    // SAFETY: We map the file read-only and never mutate the mapping. The file
    // is kept open by the `Mmap` handle for the lifetime of the returned
    // `PatternDatabase`.
    let mmap = unsafe { Mmap::map(&file)? };
    parse_database(&mmap)
}

fn parse_database(bytes: &[u8]) -> Result<PatternDatabase, DatabaseError> {
    let mut cursor = 0usize;

    if bytes.len() < 8 || &bytes[..8] != MAGIC {
        return Err(DatabaseError::InvalidMagic);
    }
    cursor += 8;

    let props_len = read_u64(bytes, &mut cursor)? as usize;
    let props_json = read_bytes(bytes, &mut cursor, props_len)?;
    let mut properties: DatabaseProperties = serde_json::from_slice(props_json)?;

    let n_stars = read_u64(bytes, &mut cursor)? as usize;
    let catalog_len = read_u64(bytes, &mut cursor)? as usize;
    let id_width = read_u8(bytes, &mut cursor)?;

    let mut star_table = Vec::with_capacity(n_stars);
    for _ in 0..n_stars {
        star_table.push(StarRow {
            ra: read_f32(bytes, &mut cursor)?,
            dec: read_f32(bytes, &mut cursor)?,
            x: read_f32(bytes, &mut cursor)?,
            y: read_f32(bytes, &mut cursor)?,
            z: read_f32(bytes, &mut cursor)?,
            mag: read_f32(bytes, &mut cursor)?,
        });
    }

    let mut pattern_catalog = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        let a = read_catalog_index_bytes(bytes, &mut cursor, id_width)?;
        let b = read_catalog_index_bytes(bytes, &mut cursor, id_width)?;
        let c = read_catalog_index_bytes(bytes, &mut cursor, id_width)?;
        let d = read_catalog_index_bytes(bytes, &mut cursor, id_width)?;
        pattern_catalog.push(PatternRow([a, b, c, d]));
    }

    let mut pattern_largest_edge = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        let bits = read_u16(bytes, &mut cursor)?;
        pattern_largest_edge.push(f16::from_bits(bits).to_f32());
    }

    let mut pattern_key_hashes = Vec::with_capacity(catalog_len);
    for _ in 0..catalog_len {
        pattern_key_hashes.push(read_u16(bytes, &mut cursor)?);
    }

    let id_shape = read_u8(bytes, &mut cursor)?;
    let id_bytes = id_shape as usize;
    let mut star_catalog_ids = Vec::with_capacity(n_stars);
    for _ in 0..n_stars {
        let id = read_bytes(bytes, &mut cursor, id_bytes)?.to_vec();
        star_catalog_ids.push(id);
    }

    properties.apply_legacy_fallbacks(catalog_len);

    Ok(PatternDatabase {
        properties,
        star_table,
        pattern_catalog,
        pattern_largest_edge,
        pattern_key_hashes,
        star_catalog_ids,
    })
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, DatabaseError> {
    if *cursor >= bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading u8",
        )));
    }
    let value = bytes[*cursor];
    *cursor += 1;
    Ok(value)
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, DatabaseError> {
    if *cursor + 2 > bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading u16",
        )));
    }
    let value = LittleEndian::read_u16(&bytes[*cursor..*cursor + 2]);
    *cursor += 2;
    Ok(value)
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, DatabaseError> {
    if *cursor + 8 > bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading u64",
        )));
    }
    let value = LittleEndian::read_u64(&bytes[*cursor..*cursor + 8]);
    *cursor += 8;
    Ok(value)
}

fn read_f32(bytes: &[u8], cursor: &mut usize) -> Result<f32, DatabaseError> {
    if *cursor + 4 > bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading f32",
        )));
    }
    let value = LittleEndian::read_f32(&bytes[*cursor..*cursor + 4]);
    *cursor += 4;
    Ok(value)
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], DatabaseError> {
    if *cursor + len > bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading byte slice",
        )));
    }
    let slice = &bytes[*cursor..*cursor + len];
    *cursor += len;
    Ok(slice)
}

fn read_catalog_index_bytes(
    bytes: &[u8],
    cursor: &mut usize,
    width: u8,
) -> Result<CatalogIndex, DatabaseError> {
    match width {
        1 => Ok(read_u8(bytes, cursor)? as CatalogIndex),
        2 => Ok(read_u16(bytes, cursor)? as CatalogIndex),
        4 => Ok(read_u32(bytes, cursor)?),
        _ => Err(DatabaseError::UnsupportedIndexWidth(width)),
    }
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, DatabaseError> {
    if *cursor + 4 > bytes.len() {
        return Err(DatabaseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected EOF reading u32",
        )));
    }
    let value = LittleEndian::read_u32(&bytes[*cursor..*cursor + 4]);
    *cursor += 4;
    Ok(value)
}

fn catalog_index_width(n_stars: usize) -> u8 {
    if n_stars <= u8::MAX as usize {
        1
    } else if n_stars <= u16::MAX as usize {
        2
    } else {
        4
    }
}

fn write_catalog_index<W: Write>(
    w: &mut W,
    idx: CatalogIndex,
    width: u8,
) -> Result<(), DatabaseError> {
    match width {
        1 => w.write_u8(idx as u8)?,
        2 => w.write_u16::<LittleEndian>(idx as u16)?,
        4 => w.write_u32::<LittleEndian>(idx)?,
        _ => return Err(DatabaseError::UnsupportedIndexWidth(width)),
    }
    Ok(())
}

fn read_catalog_index<R: Read>(r: &mut R, width: u8) -> Result<CatalogIndex, DatabaseError> {
    match width {
        1 => Ok(r.read_u8()? as CatalogIndex),
        2 => Ok(r.read_u16::<LittleEndian>()? as CatalogIndex),
        4 => Ok(r.read_u32::<LittleEndian>()?),
        _ => Err(DatabaseError::UnsupportedIndexWidth(width)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{PatternRow, StarRow};
    use crate::properties::DatabaseProperties;
    use std::f64::consts::PI;
    use tempfile::NamedTempFile;

    fn sample_db() -> PatternDatabase {
        let properties = DatabaseProperties {
            pattern_mode: "edge_ratio".to_string(),
            pattern_size: 4,
            pattern_bins: 10,
            pattern_max_error: 0.01,
            max_fov: 10.0,
            min_fov: 5.0,
            star_catalog: "test".to_string(),
            epoch_equinox: 2000.0,
            epoch_proper_motion: 2000.0,
            verification_stars_per_fov: 20,
            catalog_stars_per_fov: 20,
            star_max_magnitude: 6.5,
            star_min_magnitude: 6.5,
            hash_table_type: "quadratic_probe".to_string(),
            num_patterns: 2,
        };

        let star_table = vec![
            StarRow::from_radec_mag(0.0, 0.0, 1.0),
            StarRow::from_radec_mag(PI / 4.0, 0.0, 2.0),
            StarRow::from_radec_mag(PI / 2.0, 0.0, 3.0),
            StarRow::from_radec_mag(PI, 0.0, 4.0),
        ];
        let pattern_catalog = vec![PatternRow([0, 1, 2, 3]), PatternRow([3, 2, 1, 0])];
        let pattern_largest_edge = vec![0.123_f32, 0.456_f32];
        let pattern_key_hashes = vec![0xABCD_u16, 0x1234_u16];
        let star_catalog_ids = vec![vec![1u8], vec![2u8], vec![3u8], vec![4u8]];

        PatternDatabase {
            properties,
            star_table,
            pattern_catalog,
            pattern_largest_edge,
            pattern_key_hashes,
            star_catalog_ids,
        }
    }

    #[test]
    fn round_trip_preserves_database() {
        let db = sample_db();
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        write(&db, path).unwrap();
        let loaded = load(path).unwrap();
        assert_eq!(loaded.properties, db.properties);
        assert_eq!(loaded.star_table, db.star_table);
        assert_eq!(loaded.pattern_catalog, db.pattern_catalog);
        assert_eq!(loaded.pattern_key_hashes, db.pattern_key_hashes);
        assert_eq!(loaded.star_catalog_ids, db.star_catalog_ids);
        for (a, b) in loaded
            .pattern_largest_edge
            .iter()
            .zip(&db.pattern_largest_edge)
        {
            let diff = (a - b).abs();
            assert!(diff < 1e-3, "f16 round-trip diff too large: {}", diff);
        }
    }

    #[test]
    fn legacy_fallbacks_populate_missing_fields() {
        let properties = DatabaseProperties {
            pattern_mode: "edge_ratio".to_string(),
            max_fov: 12.0,
            catalog_stars_per_fov: 15,
            star_min_magnitude: 7.0,
            hash_table_type: "linear_probe".to_string(),
            ..Default::default()
        };

        let db = PatternDatabase {
            properties,
            star_table: vec![StarRow::from_radec_mag(0.0, 0.0, 1.0)],
            pattern_catalog: vec![PatternRow([0, 0, 0, 0]); 10],
            pattern_largest_edge: vec![0.0_f32; 10],
            pattern_key_hashes: vec![0u16; 10],
            star_catalog_ids: vec![vec![0u8]],
        };

        let file = NamedTempFile::new().unwrap();
        write(&db, file.path()).unwrap();
        let loaded = load(file.path()).unwrap();
        assert_eq!(loaded.properties.num_patterns, 5);
        assert_eq!(loaded.properties.min_fov, 12.0);
        assert_eq!(loaded.properties.verification_stars_per_fov, 15);
        assert_eq!(loaded.properties.star_max_magnitude, 7.0);
    }

    fn linear_db() -> PatternDatabase {
        let properties = DatabaseProperties {
            pattern_mode: "edge_ratio".to_string(),
            pattern_size: 4,
            pattern_bins: 10,
            pattern_max_error: 0.01,
            max_fov: 10.0,
            min_fov: 5.0,
            star_catalog: "test".to_string(),
            epoch_equinox: 2000.0,
            epoch_proper_motion: 2000.0,
            verification_stars_per_fov: 20,
            catalog_stars_per_fov: 20,
            star_max_magnitude: 6.5,
            star_min_magnitude: 6.5,
            hash_table_type: "linear_probe".to_string(),
            num_patterns: 2,
        };

        let star_table = vec![
            StarRow::from_radec_mag(0.0, 0.0, 1.0),
            StarRow::from_radec_mag(PI / 4.0, 0.0, 2.0),
            StarRow::from_radec_mag(PI / 2.0, 0.0, 3.0),
            StarRow::from_radec_mag(PI, 0.0, 4.0),
        ];
        let pattern_catalog = vec![PatternRow([0, 1, 2, 3]), PatternRow([3, 2, 1, 0])];
        let pattern_largest_edge = vec![0.123_f32, 0.456_f32];
        let pattern_key_hashes = vec![0xABCD_u16, 0x1234_u16];
        let star_catalog_ids = vec![vec![1u8], vec![2u8], vec![3u8], vec![4u8]];

        PatternDatabase {
            properties,
            star_table,
            pattern_catalog,
            pattern_largest_edge,
            pattern_key_hashes,
            star_catalog_ids,
        }
    }

    #[test]
    fn mmap_round_trip_matches_ram_load() {
        let db = linear_db();
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        write(&db, path).unwrap();
        let ram = load(path).unwrap();
        let mmap = load_mmap(path).unwrap();
        assert_eq!(ram, mmap);
    }

    #[test]
    fn missing_num_patterns_derived_from_catalog_shape() {
        let mut db = linear_db();
        db.properties.num_patterns = 0;
        let file = NamedTempFile::new().unwrap();
        write(&db, file.path()).unwrap();
        let loaded = load(file.path()).unwrap();
        assert_eq!(loaded.properties.num_patterns, db.catalog_len() as u64 / 2);
    }

    #[test]
    fn missing_min_fov_defaults_to_max_fov() {
        let mut db = linear_db();
        db.properties.min_fov = 0.0;
        let file = NamedTempFile::new().unwrap();
        write(&db, file.path()).unwrap();
        let loaded = load(file.path()).unwrap();
        assert_eq!(loaded.properties.min_fov, db.properties.max_fov);
    }
}
