//! Import a plate-solver database from a reference `.npz` file.
//!
//! Each entry in the zip is a `.npy` file. We parse the NPY header to find the
//! data offset, then read raw bytes using known dtypes.

use std::io::Read;
use std::path::Path;

use crate::{layout::*, Database, DatabaseProperties};
use half::f16;
use zip::ZipArchive;

/// Parse a UCS-4 LE string from raw bytes.
fn parse_ucs4_le(bytes: &[u8], max_chars: usize) -> String {
    let mut s = String::new();
    for i in 0..max_chars {
        let offset = i * 4;
        if offset + 4 > bytes.len() {
            break;
        }
        let cp = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        if cp == 0 {
            break;
        }
        if let Some(c) = char::from_u32(cp) {
            s.push(c);
        }
    }
    s
}

/// Parse the NPY header and return the data offset in bytes.
///
/// NPY v1: magic(6) + major(1) + minor(1) + header_len(u16 LE, 2) = 10 + header_len
/// NPY v2: magic(6) + major(1) + minor(1) + header_len(u32 LE, 4) = 12 + header_len
fn npy_data_offset(bytes: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err("invalid NPY magic".into());
    }
    let major = bytes[6];
    if major == 1 {
        let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        Ok(10 + header_len)
    } else if major == 2 {
        if bytes.len() < 12 {
            return Err("NPY v2: too short for header_len".into());
        }
        let header_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        Ok(12 + header_len)
    } else {
        Err(format!("unsupported NPY version {}", major).into())
    }
}

/// Read all bytes of a named entry from the zip archive.
fn read_entry_bytes(
    archive: &mut ZipArchive<std::fs::File>,
    name: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("entry '{}' not found in zip: {}", name, e))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Extract raw data bytes from an NPY blob (skip the header).
fn npy_data(bytes: &[u8]) -> Result<&[u8], Box<dyn std::error::Error>> {
    let offset = npy_data_offset(bytes)?;
    if offset > bytes.len() {
        return Err(format!(
            "npy data offset {} exceeds file length {}",
            offset,
            bytes.len()
        )
        .into());
    }
    Ok(&bytes[offset..])
}

/// Parse props_packed raw data bytes into a DatabaseProperties.
fn parse_props(data: &[u8]) -> Result<DatabaseProperties, Box<dyn std::error::Error>> {
    if data.len() < 828 {
        return Err(format!(
            "props_packed too small: expected >= 828 bytes, got {}",
            data.len()
        )
        .into());
    }

    let pattern_mode = parse_ucs4_le(&data[0..256], 64);
    let hash_table_type = parse_ucs4_le(&data[256..512], 64);
    let pattern_size = u16::from_le_bytes([data[512], data[513]]);
    let pattern_bins = u16::from_le_bytes([data[514], data[515]]);
    let pattern_max_error = f32::from_le_bytes([data[516], data[517], data[518], data[519]]);
    let max_fov = f32::from_le_bytes([data[520], data[521], data[522], data[523]]);
    let min_fov = f32::from_le_bytes([data[524], data[525], data[526], data[527]]);
    let star_catalog = parse_ucs4_le(&data[528..784], 64);
    let epoch_equinox = u16::from_le_bytes([data[784], data[785]]);
    let epoch_proper_motion = f32::from_le_bytes([data[786], data[787], data[788], data[789]]);
    // offset 790: lattice_field_oversampling
    let lattice_field_oversampling = u16::from_le_bytes([data[790], data[791]]);
    // offset 792: anchor_stars_per_fov (legacy — same value in reference)
    // offset 794: pattern_stars_per_fov (legacy — ignore)
    // offset 796: patterns_per_lattice_field
    let patterns_per_lattice_field = u16::from_le_bytes([data[796], data[797]]);
    // offset 798: patterns_per_anchor_star (legacy — ignore)
    // offset 800: verification_stars_per_fov
    let verification_stars_per_fov = u16::from_le_bytes([data[800], data[801]]);
    // offset 802: star_max_magnitude
    let star_max_magnitude = f32::from_le_bytes([data[802], data[803], data[804], data[805]]);
    // offset 806: simplify_pattern (legacy — ignore)
    // offset 807-814: range_ra (legacy — ignore)
    // offset 815-822: range_dec (legacy — ignore)
    // offset 823: presort_patterns
    let presort_patterns = data[823] != 0;
    // offset 824: num_patterns
    let num_patterns = u32::from_le_bytes([data[824], data[825], data[826], data[827]]);

    Ok(DatabaseProperties::apply_legacy_fallbacks(
        Some(pattern_mode),
        Some(hash_table_type),
        Some(pattern_size),
        Some(pattern_bins),
        Some(pattern_max_error),
        Some(max_fov),
        Some(min_fov),
        Some(star_catalog),
        Some(epoch_equinox),
        Some(epoch_proper_motion),
        Some(lattice_field_oversampling),
        Some(patterns_per_lattice_field),
        Some(verification_stars_per_fov),
        Some(star_max_magnitude),
        Some(presort_patterns),
        Some(num_patterns),
    ))
}

/// Import a database from a `.npz` file produced by the Python reference implementation.
pub fn import_npz(path: &Path) -> Result<Database, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // --- Read all entries ---
    let star_table_npy = read_entry_bytes(&mut archive, "star_table.npy")?;
    let pattern_catalog_npy = read_entry_bytes(&mut archive, "pattern_catalog.npy")?;
    let largest_edge_npy = read_entry_bytes(&mut archive, "pattern_largest_edge.npy")?;
    let key_hashes_npy = read_entry_bytes(&mut archive, "pattern_key_hashes.npy")?;
    let star_ids_npy = read_entry_bytes(&mut archive, "star_catalog_IDs.npy")?;
    let props_npy = read_entry_bytes(&mut archive, "props_packed.npy")?;

    // --- Parse props ---
    let props_data = npy_data(&props_npy)?;
    let properties = parse_props(props_data)?;

    // --- Parse star_table: N*6*4 bytes -> Vec<[f32; 6]> ---
    let st_data = npy_data(&star_table_npy)?;
    let num_stars = st_data.len() / (6 * 4);
    let mut star_table = Vec::with_capacity(num_stars);
    for i in 0..num_stars {
        let base = i * 24;
        let row: [f32; 6] = [
            f32::from_le_bytes([
                st_data[base],
                st_data[base + 1],
                st_data[base + 2],
                st_data[base + 3],
            ]),
            f32::from_le_bytes([
                st_data[base + 4],
                st_data[base + 5],
                st_data[base + 6],
                st_data[base + 7],
            ]),
            f32::from_le_bytes([
                st_data[base + 8],
                st_data[base + 9],
                st_data[base + 10],
                st_data[base + 11],
            ]),
            f32::from_le_bytes([
                st_data[base + 12],
                st_data[base + 13],
                st_data[base + 14],
                st_data[base + 15],
            ]),
            f32::from_le_bytes([
                st_data[base + 16],
                st_data[base + 17],
                st_data[base + 18],
                st_data[base + 19],
            ]),
            f32::from_le_bytes([
                st_data[base + 20],
                st_data[base + 21],
                st_data[base + 22],
                st_data[base + 23],
            ]),
        ];
        star_table.push(row);
    }

    // --- Determine catalog element size from star count ---
    let catalog_elem_size = if num_stars <= 255 {
        1usize
    } else if num_stars <= 65534 {
        2usize
    } else {
        4usize
    };

    // --- Parse pattern_catalog: M*4*elem_size bytes ---
    let pc_data = npy_data(&pattern_catalog_npy)?;
    let kh_data = npy_data(&key_hashes_npy)?;
    let num_slots = kh_data.len() / 2;
    debug_assert_eq!(pc_data.len(), num_slots * 4 * catalog_elem_size);

    // Parse key_hashes
    let key_hashes: Vec<u16> = kh_data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    // Build pattern catalog with empty-slot handling
    let mut catalog_u8: Option<Vec<[u8; 4]>> = None;
    let mut catalog_u16: Option<Vec<[u16; 4]>> = None;
    let mut catalog_u32: Option<Vec<[u32; 4]>> = None;

    match catalog_elem_size {
        1 => {
            let mut cat = Vec::with_capacity(num_slots);
            for i in 0..num_slots {
                if key_hashes[i] == 0 {
                    // Empty slot: set all indices to EMPTY_SLOT_U8
                    cat.push([EMPTY_SLOT_U8; 4]);
                } else {
                    let base = i * 4;
                    cat.push([
                        pc_data[base],
                        pc_data[base + 1],
                        pc_data[base + 2],
                        pc_data[base + 3],
                    ]);
                }
            }
            catalog_u8 = Some(cat);
        }
        2 => {
            let mut cat = Vec::with_capacity(num_slots);
            for i in 0..num_slots {
                if key_hashes[i] == 0 {
                    cat.push([EMPTY_SLOT_U16; 4]);
                } else {
                    let base = i * 8;
                    cat.push([
                        u16::from_le_bytes([pc_data[base], pc_data[base + 1]]),
                        u16::from_le_bytes([pc_data[base + 2], pc_data[base + 3]]),
                        u16::from_le_bytes([pc_data[base + 4], pc_data[base + 5]]),
                        u16::from_le_bytes([pc_data[base + 6], pc_data[base + 7]]),
                    ]);
                }
            }
            catalog_u16 = Some(cat);
        }
        4 => {
            let mut cat = Vec::with_capacity(num_slots);
            for i in 0..num_slots {
                if key_hashes[i] == 0 {
                    cat.push([EMPTY_SLOT_U32; 4]);
                } else {
                    let base = i * 16;
                    cat.push([
                        u32::from_le_bytes([
                            pc_data[base],
                            pc_data[base + 1],
                            pc_data[base + 2],
                            pc_data[base + 3],
                        ]),
                        u32::from_le_bytes([
                            pc_data[base + 4],
                            pc_data[base + 5],
                            pc_data[base + 6],
                            pc_data[base + 7],
                        ]),
                        u32::from_le_bytes([
                            pc_data[base + 8],
                            pc_data[base + 9],
                            pc_data[base + 10],
                            pc_data[base + 11],
                        ]),
                        u32::from_le_bytes([
                            pc_data[base + 12],
                            pc_data[base + 13],
                            pc_data[base + 14],
                            pc_data[base + 15],
                        ]),
                    ]);
                }
            }
            catalog_u32 = Some(cat);
        }
        _ => unreachable!(),
    }

    // --- Parse pattern_largest_edge: M*2 bytes -> Vec<f16> ---
    let le_data = npy_data(&largest_edge_npy)?;
    let largest_edge: Vec<f16> = le_data
        .chunks_exact(2)
        .map(|chunk| f16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    // --- Parse star_catalog_IDs: N*4 bytes -> Vec<u32> ---
    let ids_data = npy_data(&star_ids_npy)?;
    let star_catalog_ids_u32: Option<Vec<u32>> = if ids_data.is_empty() {
        None
    } else {
        Some(
            ids_data
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect(),
        )
    };

    Ok(Database {
        properties,
        star_table,
        pattern_catalog_u8: catalog_u8,
        pattern_catalog_u16: catalog_u16,
        pattern_catalog_u32: catalog_u32,
        largest_edge,
        key_hashes,
        star_catalog_ids_u16: None,
        star_catalog_ids_u32,
        #[cfg(feature = "kd-tree")]
        star_kd_tree: None,
    })
}
