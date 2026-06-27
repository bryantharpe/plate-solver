//! Native binary format constants and magic bytes for the ps-db on-disk layout.
//!
//! See the module-level documentation in `lib.rs` for the full byte layout.

/// File magic: identifies a ps-db native binary file.
pub const MAGIC: &[u8; 4] = b"PSDB";

/// Current format version. Increment when the layout changes in a breaking way.
pub const VERSION: u32 = 1;

/// Byte alignment for section boundaries (sections start at multiples of this).
pub const SECTION_ALIGNMENT: usize = 8;

/// Sentinel value for an empty slot in an 8-bit pattern catalog.
pub const EMPTY_SLOT_U8: u8 = u8::MAX;
/// Sentinel value for an empty slot in a 16-bit pattern catalog.
pub const EMPTY_SLOT_U16: u16 = u16::MAX;
/// Sentinel value for an empty slot in a 32-bit pattern catalog.
pub const EMPTY_SLOT_U32: u32 = u32::MAX;

/// Number of star-table columns per row.
pub const STAR_TABLE_COLS: usize = 6;

/// Number of star indices per pattern.
pub const PATTERN_SIZE: usize = 4;
