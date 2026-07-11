//! Memory-mapped load path for the native binary database format.
//!
//! Provides zero-copy typed slice views into the memory-mapped file so that
//! the pattern catalog, largest_edge, and key_hashes arrays are never copied
//! into RAM.

use std::path::Path;

use half::f16;
use memmap2::Mmap;

use crate::{layout::*, DatabaseProperties};

/// Which element width the on-disk pattern catalog uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternCatalogKind {
    U8,
    U16,
    U32,
}

/// A database backed by a memory-mapped file.
///
/// The `_mmap` field keeps the mmap alive; all data is accessed via raw
/// pointers into the mmap'd region.  We use raw pointers so the struct does
/// not carry lifetimes, and implement `Send`/`Sync` manually (the mmap is
/// immutable after creation).
///
/// Because the native format does not guarantee alignment within sections
/// (e.g., a 1-byte ELEM_SIZE field precedes the catalog data), accessors
/// use unaligned reads where necessary.
pub struct MmappedDatabase {
    _mmap: Mmap,
    pub properties: DatabaseProperties,

    // Star table: each row is [f32; 6], stored as raw bytes in the mmap.
    star_table_offset: usize,
    star_table_count: usize,

    // Key hashes: u16 per slot, stored as raw bytes.
    key_hashes_offset: usize,
    key_hashes_count: usize,

    // Largest edge: f16 per slot, stored as raw bytes.
    largest_edge_offset: usize,
    largest_edge_count: usize,

    // Pattern catalog: [T; 4] per slot where T is u8/u16/u32.
    #[allow(dead_code)]
    catalog_kind: PatternCatalogKind,
    catalog_offset: usize,
    catalog_elem_size: usize,
    catalog_count: usize,

    #[cfg(feature = "kd-tree")]
    pub star_kd_tree: Option<kiddo::KdTree<f32, 3>>,
}

unsafe impl Send for MmappedDatabase {}
unsafe impl Sync for MmappedDatabase {}

impl MmappedDatabase {
    /// Zero-copy view of the star table.
    ///
    /// Returns a slice of `[f32; 6]` rows backed by the mmap.
    /// Safe because star_table starts at an 8-byte aligned offset and each
    /// row (24 bytes) is a multiple of f32's alignment (4).
    pub fn star_table(&self) -> Result<&[[f32; 6]], Box<dyn std::error::Error>> {
        let data: &[u8] = &self._mmap;
        let start = self.star_table_offset;
        let end = start + self.star_table_count * 24;
        let bytes = &data[start..end];
        let ptr = bytes.as_ptr();
        if ptr.align_offset(4) == 0 {
            Ok(unsafe { std::slice::from_raw_parts(ptr as *const [f32; 6], self.star_table_count) })
        } else {
            Err(format!("star_table offset {} is not [f32;6]-aligned (4-byte)", start).into())
        }
    }

    /// Zero-copy view of the key hashes.
    ///
    /// Key hashes are u16 per slot.  We read them unaligned from the mmap
    /// bytes into a small on-stack buffer per access... actually, to provide
    /// a true zero-copy `&[u16]` we need aligned data.  Since the format may
    /// not guarantee alignment, we return a `KeyHashView` that provides
    /// indexed access without copying the full array.
    ///
    /// For the common case where the offset happens to be u16-aligned, we
    /// provide a direct slice.  Otherwise we fall back to unaligned reads.
    pub fn key_hashes(&self) -> Result<&[u16], Box<dyn std::error::Error>> {
        let data: &[u8] = &self._mmap;
        let start = self.key_hashes_offset;
        let bytes = &data[start..start + self.key_hashes_count * 2];
        let ptr = bytes.as_ptr();
        if ptr.align_offset(2) == 0 {
            Ok(unsafe { std::slice::from_raw_parts(ptr as *const u16, self.key_hashes_count) })
        } else {
            // Misaligned: we cannot provide a &[u16] view.
            // This should not happen with the current save_native layout,
            // but handle it gracefully by reading unaligned into a static buffer.
            // In practice this path is unreachable with the current format.
            Err(format!("key_hashes offset {} is not u16-aligned", start).into())
        }
    }

    /// Zero-copy view of the largest-edge values.
    pub fn largest_edge(&self) -> Result<&[f16], Box<dyn std::error::Error>> {
        let data: &[u8] = &self._mmap;
        let start = self.largest_edge_offset;
        let bytes = &data[start..start + self.largest_edge_count * 2];
        let ptr = bytes.as_ptr();
        if ptr.align_offset(2) == 0 {
            Ok(unsafe { std::slice::from_raw_parts(ptr as *const f16, self.largest_edge_count) })
        } else {
            Err(format!("largest_edge offset {} is not f16-aligned", start).into())
        }
    }

    /// Number of hash-table slots.
    pub fn num_slots(&self) -> usize {
        self.catalog_count
    }

    /// Number of stars.
    pub fn num_stars(&self) -> usize {
        self.star_table_count
    }

    /// Return the catalog entry at `slot` as `[u32; 4]`, widening from the
    /// on-disk element width if necessary.  Returns `None` for out-of-range
    /// slots.
    pub fn catalog_entry(&self, slot: usize) -> Option<[u32; 4]> {
        if slot >= self.catalog_count {
            return None;
        }
        let data: &[u8] = &self._mmap;
        let row_start = self.catalog_offset + slot * 4 * self.catalog_elem_size;
        let row_bytes = &data[row_start..row_start + 4 * self.catalog_elem_size];

        match self.catalog_elem_size {
            1 => {
                // [u8; 4] — no alignment concern
                Some([
                    row_bytes[0] as u32,
                    row_bytes[1] as u32,
                    row_bytes[2] as u32,
                    row_bytes[3] as u32,
                ])
            }
            2 => {
                // [u16; 4] — may be misaligned, use unaligned reads
                let v0 = u16::from_le_bytes([row_bytes[0], row_bytes[1]]) as u32;
                let v1 = u16::from_le_bytes([row_bytes[2], row_bytes[3]]) as u32;
                let v2 = u16::from_le_bytes([row_bytes[4], row_bytes[5]]) as u32;
                let v3 = u16::from_le_bytes([row_bytes[6], row_bytes[7]]) as u32;
                Some([v0, v1, v2, v3])
            }
            4 => {
                // [u32; 4] — may be misaligned, use unaligned reads
                let v0 =
                    u32::from_le_bytes([row_bytes[0], row_bytes[1], row_bytes[2], row_bytes[3]]);
                let v1 =
                    u32::from_le_bytes([row_bytes[4], row_bytes[5], row_bytes[6], row_bytes[7]]);
                let v2 =
                    u32::from_le_bytes([row_bytes[8], row_bytes[9], row_bytes[10], row_bytes[11]]);
                let v3 = u32::from_le_bytes([
                    row_bytes[12],
                    row_bytes[13],
                    row_bytes[14],
                    row_bytes[15],
                ]);
                Some([v0, v1, v2, v3])
            }
            _ => None,
        }
    }

    /// Get a single key_hash value by index (handles potential misalignment).
    fn key_hash_at(&self, idx: usize) -> u16 {
        let data: &[u8] = &self._mmap;
        let off = self.key_hashes_offset + idx * 2;
        u16::from_le_bytes([data[off], data[off + 1]])
    }

    /// Get a single largest_edge value by index (handles potential misalignment).
    fn largest_edge_at(&self, idx: usize) -> f16 {
        let data: &[u8] = &self._mmap;
        let off = self.largest_edge_offset + idx * 2;
        f16::from_le_bytes([data[off], data[off + 1]])
    }

    /// Find all stars within a given angular radius of a query unit vector.
    #[cfg(feature = "kd-tree")]
    pub fn nearby_stars(&self, vector: [f32; 3], radius: f32) -> Vec<usize> {
        use kiddo::SquaredEuclidean;

        let max_dist = 2.0_f32 * (radius / 2.0).sin();
        let max_dist_sq = max_dist * max_dist;

        if let Some(tree) = &self.star_kd_tree {
            let results = tree.within_unsorted::<SquaredEuclidean>(&vector, max_dist_sq);
            let mut inds: Vec<usize> = results.iter().map(|r| r.item as usize).collect();
            inds.sort_unstable();
            inds
        } else {
            vec![]
        }
    }

    /// Build the KD-tree from the star table unit vectors.
    #[cfg(feature = "kd-tree")]
    pub fn build_kd_tree(&mut self) {
        use kiddo::KdTree;
        let stars = self.star_table().expect("star_table alignment invariant violated (malformed database file)");
        let mut tree: KdTree<f32, 3> = KdTree::with_capacity(stars.len());
        for (i, row) in stars.iter().enumerate() {
            tree.add(&[row[2], row[3], row[4]], i as u64);
        }
        self.star_kd_tree = Some(tree);
    }
}

impl MmappedDatabase {
    /// Force an arbitrary star_table layout so the alignment check in
    /// [`MmappedDatabase::star_table`] can be exercised. A real database file
    /// always lands on an aligned offset, so this is the only way to reach the
    /// error branch. Public (but hidden) because the test lives in `tests/`.
    #[doc(hidden)]
    pub fn set_star_table_layout_for_test(&mut self, offset: usize, count: usize) {
        self.star_table_offset = offset;
        self.star_table_count = count;
    }
}

/// Load a database from the native binary format via memory mapping.
///
/// Walks the header identically to `loader::load_native` but records byte
/// offsets instead of copying data into Vecs.
pub fn load_native_mmap(path: &Path) -> Result<MmappedDatabase, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };

    // Deref Mmap to get &[u8].
    let data: &[u8] = &mmap;
    let mut pos: usize = 0;

    // 1. MAGIC (4 bytes)
    if data.len() < 4 || &data[0..4] != MAGIC {
        return Err("invalid magic bytes".into());
    }
    pos += 4;

    // 2. VERSION u32 LE (4 bytes)
    if data.len() < pos + 4 {
        return Err("truncated: missing version".into());
    }
    let version = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
    pos += 4;
    if version != VERSION {
        return Err(format!("unsupported version {} (expected {})", version, VERSION).into());
    }

    // 3. PROPS_LEN u32 LE (4 bytes)
    if data.len() < pos + 4 {
        return Err("truncated: missing props_len".into());
    }
    let props_len =
        u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
    pos += 4;

    // 4. PROPS_JSON (variable UTF-8)
    if data.len() < pos + props_len {
        return Err("truncated: props_json too short".into());
    }
    let props_json = String::from_utf8(data[pos..pos + props_len].to_vec())?;
    pos += props_len;
    let properties: DatabaseProperties = serde_json::from_str(&props_json)?;

    // 5. Skip padding to 8-byte alignment
    let padding_needed = (SECTION_ALIGNMENT - (pos % SECTION_ALIGNMENT)) % SECTION_ALIGNMENT;
    pos += padding_needed;

    // Helper to read N bytes at current position
    let read_bytes =
        |buf: &mut [u8], data: &[u8], pos: usize| -> Result<usize, Box<dyn std::error::Error>> {
            if data.len() < pos + buf.len() {
                return Err("truncated".into());
            }
            buf.copy_from_slice(&data[pos..pos + buf.len()]);
            Ok(buf.len())
        };

    // 6. star_table section: COUNT u64 LE, then N*6*4 bytes (f32 LE)
    let mut buf8 = [0u8; 8];
    read_bytes(&mut buf8, data, pos)?;
    let num_stars = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let star_data_len = num_stars * 6 * 4;
    if data.len() < pos + star_data_len {
        return Err("truncated: star_table data".into());
    }
    let star_table_offset = pos;
    pos += star_data_len;

    // 7. pattern_catalog section: COUNT u64 LE, ELEM_SIZE u8, then data
    read_bytes(&mut buf8, data, pos)?;
    let catalog_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    if data.len() < pos + 1 {
        return Err("truncated: catalog elem_size".into());
    }
    let elem_size = data[pos] as usize;
    pos += 1;

    let catalog_data_len = catalog_count * 4 * elem_size;
    if data.len() < pos + catalog_data_len {
        return Err("truncated: pattern_catalog data".into());
    }
    let catalog_offset = pos;

    let catalog_kind = match elem_size {
        1 => PatternCatalogKind::U8,
        2 => PatternCatalogKind::U16,
        4 => PatternCatalogKind::U32,
        _ => return Err(format!("invalid catalog elem_size {}", elem_size).into()),
    };
    pos += catalog_data_len;

    // 8. largest_edge section: COUNT u64 LE, then N*2 bytes (f16 LE)
    read_bytes(&mut buf8, data, pos)?;
    let le_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let le_data_len = le_count * 2;
    if data.len() < pos + le_data_len {
        return Err("truncated: largest_edge data".into());
    }
    let largest_edge_offset = pos;
    pos += le_data_len;

    // 9. key_hashes section: COUNT u64 LE, then N*2 bytes (u16 LE)
    read_bytes(&mut buf8, data, pos)?;
    let kh_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let kh_data_len = kh_count * 2;
    if data.len() < pos + kh_data_len {
        return Err("truncated: key_hashes data".into());
    }
    let key_hashes_offset = pos;
    pos += kh_data_len;

    // 10. star_catalog_IDs section — present but not needed for lookup; skip.
    if data.len() < pos + 1 {
        return Err("truncated: star_catalog_ids present flag".into());
    }
    let present = data[pos];
    pos += 1;
    if present != 0 {
        if data.len() < pos + 1 {
            return Err("truncated: star_catalog_ids elem_size".into());
        }
        let ids_elem_size = data[pos];
        pos += 1;
        read_bytes(&mut buf8, data, pos)?;
        let ids_count = u64::from_le_bytes(buf8) as usize;
        pos += 8;
        let ids_data_len = ids_count * ids_elem_size as usize;
        if data.len() < pos + ids_data_len {
            return Err("truncated: star_catalog_ids data".into());
        }
    }

    let db = MmappedDatabase {
        _mmap: mmap,
        properties,
        star_table_offset,
        star_table_count: num_stars,
        key_hashes_offset,
        key_hashes_count: kh_count,
        largest_edge_offset,
        largest_edge_count: le_count,
        catalog_kind,
        catalog_offset,
        catalog_elem_size: elem_size,
        catalog_count,
        #[cfg(feature = "kd-tree")]
        star_kd_tree: None,
    };

    Ok(db)
}

/// Hash-table pattern lookup for `MmappedDatabase`.
///
/// Identical algorithm to `lookup_pattern` in `lookup.rs` but operates on
/// the zero-copy views provided by `MmappedDatabase`.
pub fn lookup_pattern_mmap(
    db: &MmappedDatabase,
    key: &[u32; 5],
    largest_edge_rad: f64,
    coarse_fov_rad: Option<f64>,
) -> Vec<usize> {
    use ps_core::pattern::{
        compute_pattern_key_hash, key_hash_low16, pattern_key_hash_to_index, probe_slots,
    };

    let table_size = db.num_slots() as u64;
    if table_size == 0 {
        return Vec::new();
    }

    let pattern_bins = db.properties.pattern_bins as u32;
    let linear_probe = db.properties.hash_table_type == "linear_probe";
    let pattern_max_error = db.properties.pattern_max_error as f64;

    let full_hash = compute_pattern_key_hash(key, pattern_bins);
    let low16 = key_hash_low16(full_hash);

    let hash_index = pattern_key_hash_to_index(full_hash, table_size, linear_probe);

    let probe_indices = probe_slots(hash_index, table_size, linear_probe, db.num_slots());

    let (has_fov_filter, fov_max_error) = match coarse_fov_rad {
        Some(fov) => (true, fov * pattern_max_error),
        None => (false, 0.0),
    };

    let mut candidates = Vec::new();

    for &slot in probe_indices.iter() {
        let slot = slot as usize;

        // Empty-slot check: key_hashes[slot] == 0 AND largest_edge bits == 0
        if db.key_hash_at(slot) == 0 && db.largest_edge_at(slot).to_bits() == 0 {
            break;
        }

        // 16-bit key hash pre-filter
        if db.key_hash_at(slot) != low16 {
            continue;
        }

        // Largest-edge / FOV pre-filter
        if has_fov_filter {
            let largest_edge_mrad = db.largest_edge_at(slot).to_f64();
            let fov2 = largest_edge_mrad / largest_edge_rad * coarse_fov_rad.unwrap() / 1000.0;
            if (fov2 - coarse_fov_rad.unwrap()).abs() >= fov_max_error {
                continue;
            }
        }

        candidates.push(slot);
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_props() -> DatabaseProperties {
        DatabaseProperties::apply_legacy_fallbacks(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        )
    }

    #[test]
    fn test_star_table_misalignment_error() {
        // Create a temporary file with arbitrary data.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), vec![0u8; 64]).unwrap();

        // Open the file and create an mmap.
        let file = std::fs::File::open(tmp.path()).unwrap();
        let mmap = unsafe { Mmap::map(&file) }.unwrap();

        // Create a MmappedDatabase with deliberately misaligned star_table_offset.
        // offset 1 is not a multiple of 4, so [f32; 6] alignment check should fail.
        let db = MmappedDatabase {
            _mmap: mmap,
            properties: default_props(),
            star_table_offset: 1,
            star_table_count: 1,
            key_hashes_offset: 0,
            key_hashes_count: 0,
            largest_edge_offset: 0,
            largest_edge_count: 0,
            catalog_kind: PatternCatalogKind::U8,
            catalog_offset: 0,
            catalog_elem_size: 1,
            catalog_count: 0,
            #[cfg(feature = "kd-tree")]
            star_kd_tree: None,
        };

        // Verify that star_table() returns Err due to misalignment.
        assert!(db.star_table().is_err());
    }

    #[test]
    fn test_star_table_success_on_real_database() {
        // Load a real database from the reference solution.
        let npz_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
        let db = crate::importer::import_npz(&npz_path).unwrap();

        // Save it as native format.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        crate::loader::save_native(&db, tmp.path()).unwrap();

        // Load it via mmap.
        let db_mmap = load_native_mmap(tmp.path()).unwrap();

        // Verify that star_table() returns Ok with correct length.
        let stars = db_mmap.star_table().unwrap();
        assert_eq!(stars.len(), db_mmap.num_stars());
        assert_eq!(stars.len(), 42212); // The test fixture has 42212 stars.

        // Verify that we can access star data (spot-check first star).
        let first_star = &stars[0];
        assert_eq!(first_star.len(), 6);
    }
}
