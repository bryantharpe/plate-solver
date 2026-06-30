//! Save/load the native binary database format.
//!
//! See the module-level documentation in `lib.rs` for the full byte layout.

use std::io::Write;
use std::path::Path;

use crate::{layout::*, Database, DatabaseProperties};
use half::f16;

/// Write a database to the native binary format.
pub fn save_native(db: &Database, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::File::create(path)?;

    // 1. MAGIC (4 bytes)
    file.write_all(MAGIC)?;

    // 2. VERSION u32 LE (4 bytes)
    file.write_all(&VERSION.to_le_bytes())?;

    // 3. PROPS_LEN u32 LE (4 bytes) + 4. PROPS_JSON (variable UTF-8)
    let props_json = serde_json::to_string(&db.properties)?;
    let props_bytes = props_json.into_bytes();
    file.write_all(&(props_bytes.len() as u32).to_le_bytes())?;
    file.write_all(&props_bytes)?;

    // 5. Padding to 8-byte alignment
    let current_pos = 4 + 4 + 4 + props_bytes.len();
    let padding_needed =
        (SECTION_ALIGNMENT - (current_pos % SECTION_ALIGNMENT)) % SECTION_ALIGNMENT;
    for _ in 0..padding_needed {
        file.write_all(&[0u8])?;
    }

    // 6. star_table section: COUNT u64 LE, then N*6*4 bytes (f32 LE)
    let num_stars = db.star_table.len() as u64;
    file.write_all(&num_stars.to_le_bytes())?;
    for row in &db.star_table {
        for val in row {
            file.write_all(&val.to_le_bytes())?;
        }
    }

    // 7. pattern_catalog section: COUNT u64 LE, ELEM_SIZE u8, then data
    let (catalog_count, elem_size, catalog_data) = match (
        &db.pattern_catalog_u8,
        &db.pattern_catalog_u16,
        &db.pattern_catalog_u32,
    ) {
        (Some(cat), None, None) => (cat.len() as u64, 1u8, {
            let mut buf = Vec::with_capacity(cat.len() * 4);
            for row in cat {
                buf.extend_from_slice(row);
            }
            buf
        }),
        (None, Some(cat), None) => (cat.len() as u64, 2u8, {
            let mut buf = Vec::with_capacity(cat.len() * 8);
            for row in cat {
                for val in row {
                    buf.extend_from_slice(&val.to_le_bytes());
                }
            }
            buf
        }),
        (None, None, Some(cat)) => (cat.len() as u64, 4u8, {
            let mut buf = Vec::with_capacity(cat.len() * 16);
            for row in cat {
                for val in row {
                    buf.extend_from_slice(&val.to_le_bytes());
                }
            }
            buf
        }),
        _ => {
            return Err("exactly one pattern catalog variant must be Some".into());
        }
    };
    file.write_all(&catalog_count.to_le_bytes())?;
    file.write_all(&[elem_size])?;
    file.write_all(&catalog_data)?;

    // 8. largest_edge section: COUNT u64 LE, then N*2 bytes (f16 LE)
    let le_count = db.largest_edge.len() as u64;
    file.write_all(&le_count.to_le_bytes())?;
    for val in &db.largest_edge {
        file.write_all(&val.to_le_bytes())?;
    }

    // 9. key_hashes section: COUNT u64 LE, then N*2 bytes (u16 LE)
    let kh_count = db.key_hashes.len() as u64;
    file.write_all(&kh_count.to_le_bytes())?;
    for val in &db.key_hashes {
        file.write_all(&val.to_le_bytes())?;
    }

    // 10. star_catalog_IDs section: PRESENT u8, if 1: ELEM_SIZE u8, COUNT u64, DATA
    match (&db.star_catalog_ids_u16, &db.star_catalog_ids_u32) {
        (None, None) => {
            file.write_all(&[0u8])?;
        }
        (Some(ids), None) => {
            file.write_all(&[1u8])?; // PRESENT
            file.write_all(&[2u8])?; // ELEM_SIZE
            file.write_all(&(ids.len() as u64).to_le_bytes())?;
            for val in ids {
                file.write_all(&val.to_le_bytes())?;
            }
        }
        (None, Some(ids)) => {
            file.write_all(&[1u8])?; // PRESENT
            file.write_all(&[4u8])?; // ELEM_SIZE
            file.write_all(&(ids.len() as u64).to_le_bytes())?;
            for val in ids {
                file.write_all(&val.to_le_bytes())?;
            }
        }
        (Some(_), Some(_)) => {
            return Err("only one star_catalog_ids variant may be present".into());
        }
    }

    Ok(())
}

/// Read a database from the native binary format.
pub fn load_native(path: &Path) -> Result<Database, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;

    let mut pos = 0usize;

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
    read_bytes(&mut buf8, &data, pos)?;
    let num_stars = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let star_data_len = num_stars * 6 * 4;
    if data.len() < pos + star_data_len {
        return Err("truncated: star_table data".into());
    }
    let mut star_table = Vec::with_capacity(num_stars);
    for i in 0..num_stars {
        let base = pos + i * 24;
        let row: [f32; 6] = [
            f32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]),
            f32::from_le_bytes([
                data[base + 4],
                data[base + 5],
                data[base + 6],
                data[base + 7],
            ]),
            f32::from_le_bytes([
                data[base + 8],
                data[base + 9],
                data[base + 10],
                data[base + 11],
            ]),
            f32::from_le_bytes([
                data[base + 12],
                data[base + 13],
                data[base + 14],
                data[base + 15],
            ]),
            f32::from_le_bytes([
                data[base + 16],
                data[base + 17],
                data[base + 18],
                data[base + 19],
            ]),
            f32::from_le_bytes([
                data[base + 20],
                data[base + 21],
                data[base + 22],
                data[base + 23],
            ]),
        ];
        star_table.push(row);
    }
    pos += star_data_len;

    // 7. pattern_catalog section: COUNT u64 LE, ELEM_SIZE u8, then data
    read_bytes(&mut buf8, &data, pos)?;
    let catalog_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    if data.len() < pos + 1 {
        return Err("truncated: catalog elem_size".into());
    }
    let elem_size = data[pos];
    pos += 1;

    let catalog_data_len = catalog_count * 4 * elem_size as usize;
    if data.len() < pos + catalog_data_len {
        return Err("truncated: pattern_catalog data".into());
    }

    let (pattern_catalog_u8, pattern_catalog_u16, pattern_catalog_u32): (
        Option<Vec<[u8; 4]>>,
        Option<Vec<[u16; 4]>>,
        Option<Vec<[u32; 4]>>,
    ) = match elem_size {
        1 => {
            let mut cat = Vec::with_capacity(catalog_count);
            for i in 0..catalog_count {
                let base = pos + i * 4;
                cat.push([data[base], data[base + 1], data[base + 2], data[base + 3]]);
            }
            (Some(cat), None, None)
        }
        2 => {
            let mut cat = Vec::with_capacity(catalog_count);
            for i in 0..catalog_count {
                let base = pos + i * 8;
                cat.push([
                    u16::from_le_bytes([data[base], data[base + 1]]),
                    u16::from_le_bytes([data[base + 2], data[base + 3]]),
                    u16::from_le_bytes([data[base + 4], data[base + 5]]),
                    u16::from_le_bytes([data[base + 6], data[base + 7]]),
                ]);
            }
            (None, Some(cat), None)
        }
        4 => {
            let mut cat = Vec::with_capacity(catalog_count);
            for i in 0..catalog_count {
                let base = pos + i * 16;
                cat.push([
                    u32::from_le_bytes([
                        data[base],
                        data[base + 1],
                        data[base + 2],
                        data[base + 3],
                    ]),
                    u32::from_le_bytes([
                        data[base + 4],
                        data[base + 5],
                        data[base + 6],
                        data[base + 7],
                    ]),
                    u32::from_le_bytes([
                        data[base + 8],
                        data[base + 9],
                        data[base + 10],
                        data[base + 11],
                    ]),
                    u32::from_le_bytes([
                        data[base + 12],
                        data[base + 13],
                        data[base + 14],
                        data[base + 15],
                    ]),
                ]);
            }
            (None, None, Some(cat))
        }
        _ => {
            return Err(format!("invalid catalog elem_size {}", elem_size).into());
        }
    };
    pos += catalog_data_len;

    // 8. largest_edge section: COUNT u64 LE, then N*2 bytes (f16 LE)
    read_bytes(&mut buf8, &data, pos)?;
    let le_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let le_data_len = le_count * 2;
    if data.len() < pos + le_data_len {
        return Err("truncated: largest_edge data".into());
    }
    let mut largest_edge = Vec::with_capacity(le_count);
    for i in 0..le_count {
        let base = pos + i * 2;
        largest_edge.push(f16::from_le_bytes([data[base], data[base + 1]]));
    }
    pos += le_data_len;

    // 9. key_hashes section: COUNT u64 LE, then N*2 bytes (u16 LE)
    read_bytes(&mut buf8, &data, pos)?;
    let kh_count = u64::from_le_bytes(buf8) as usize;
    pos += 8;

    let kh_data_len = kh_count * 2;
    if data.len() < pos + kh_data_len {
        return Err("truncated: key_hashes data".into());
    }
    let mut key_hashes = Vec::with_capacity(kh_count);
    for i in 0..kh_count {
        let base = pos + i * 2;
        key_hashes.push(u16::from_le_bytes([data[base], data[base + 1]]));
    }
    pos += kh_data_len;

    // 10. star_catalog_IDs section: PRESENT u8, if 1: ELEM_SIZE u8, COUNT u64, DATA
    if data.len() < pos + 1 {
        return Err("truncated: star_catalog_ids present flag".into());
    }
    let present = data[pos];
    pos += 1;

    let (star_catalog_ids_u16, star_catalog_ids_u32) = if present == 0 {
        (None, None)
    } else {
        if data.len() < pos + 1 {
            return Err("truncated: star_catalog_ids elem_size".into());
        }
        let ids_elem_size = data[pos];
        pos += 1;

        read_bytes(&mut buf8, &data, pos)?;
        let ids_count = u64::from_le_bytes(buf8) as usize;
        pos += 8;

        let ids_data_len = ids_count * ids_elem_size as usize;
        if data.len() < pos + ids_data_len {
            return Err("truncated: star_catalog_ids data".into());
        }

        match ids_elem_size {
            2 => {
                let mut ids = Vec::with_capacity(ids_count);
                for i in 0..ids_count {
                    let base = pos + i * 2;
                    ids.push(u16::from_le_bytes([data[base], data[base + 1]]));
                }
                (Some(ids), None)
            }
            4 => {
                let mut ids = Vec::with_capacity(ids_count);
                for i in 0..ids_count {
                    let base = pos + i * 4;
                    ids.push(u32::from_le_bytes([
                        data[base],
                        data[base + 1],
                        data[base + 2],
                        data[base + 3],
                    ]));
                }
                (None, Some(ids))
            }
            _ => {
                return Err(format!("invalid star_catalog_ids elem_size {}", ids_elem_size).into());
            }
        }
    };

    Ok(Database {
        properties,
        star_table,
        pattern_catalog_u8,
        pattern_catalog_u16,
        pattern_catalog_u32,
        largest_edge,
        key_hashes,
        star_catalog_ids_u16,
        star_catalog_ids_u32,
        #[cfg(feature = "kd-tree")]
        star_kd_tree: None,
    })
}
