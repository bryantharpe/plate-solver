#![deny(unsafe_code)]
//! A tiny, panic-free byte parser used to exercise the fuzzing, unsafe-census,
//! and Miri gates.
//!
//! `#![deny(unsafe_code)]` means every `unsafe` site must carry an explicit
//! allow attribute, so the census is **compiler-enforced and greppable** — the
//! anchored search `grep -rnE '^\s*#\[allow\(unsafe_code\)\]' src` returns
//! exactly the audited sites (here, one). That is a stronger guarantee than a
//! separate census tool, which could silently stop running.

/// A parsed record header: 4-byte magic `CNRY`, little-endian `u16` version,
/// little-endian `u32` payload length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Format version.
    pub version: u16,
    /// Declared payload length in bytes.
    pub payload_len: u32,
}

/// Four-byte magic that every record begins with.
pub const MAGIC: [u8; 4] = *b"CNRY";

/// Parse a [`Header`] from the front of `bytes`, or `None` if `bytes` is shorter
/// than the 10-byte header or does not begin with [`MAGIC`]. Never panics, for
/// any input — the fuzz target enforces this.
///
/// # Examples
/// ```
/// use canary_parse::{parse_header, Header, MAGIC};
/// let mut buf = MAGIC.to_vec();
/// buf.extend_from_slice(&7u16.to_le_bytes());
/// buf.extend_from_slice(&258u32.to_le_bytes());
/// assert_eq!(parse_header(&buf), Some(Header { version: 7, payload_len: 258 }));
/// ```
#[must_use]
pub fn parse_header(bytes: &[u8]) -> Option<Header> {
    if bytes.len() < 10 || bytes[0..4] != MAGIC {
        return None;
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let payload_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    Some(Header {
        version,
        payload_len,
    })
}

/// Interpret `bytes` as a `&str` **without** re-validating UTF-8, but only after
/// confirming every byte is ASCII. Returns `None` if any byte is non-ASCII.
///
/// This is the crate's one deliberate `unsafe` site — it exists so the census is
/// non-trivial (exactly one) and Miri has an `unsafe` path to prove sound.
#[must_use]
pub fn ascii_tag(bytes: &[u8]) -> Option<&str> {
    if !bytes.iter().all(u8::is_ascii) {
        return None;
    }
    // SAFETY: every byte was just confirmed to be ASCII (`< 0x80`). Every ASCII
    // byte sequence is valid UTF-8, so `from_utf8_unchecked` cannot construct an
    // ill-formed `str`. Exercised under Miri by `ascii_tag_roundtrip`.
    #[allow(unsafe_code)]
    let s = unsafe { core::str::from_utf8_unchecked(bytes) };
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::{ascii_tag, parse_header, Header, MAGIC};

    #[test]
    fn parses_a_valid_header() {
        let mut buf = MAGIC.to_vec();
        buf.extend_from_slice(&7u16.to_le_bytes());
        buf.extend_from_slice(&258u32.to_le_bytes());
        assert_eq!(
            parse_header(&buf),
            Some(Header {
                version: 7,
                payload_len: 258,
            })
        );
    }

    #[test]
    fn rejects_short_input() {
        assert_eq!(parse_header(b"CNRY"), None);
        assert_eq!(parse_header(&[]), None);
    }

    #[test]
    fn rejects_bad_magic() {
        assert_eq!(parse_header(b"XXXX000000"), None);
    }

    #[test]
    fn ascii_tag_roundtrip() {
        assert_eq!(ascii_tag(b"hello"), Some("hello"));
        assert_eq!(ascii_tag(b""), Some(""));
        assert_eq!(ascii_tag(&[0xff, 0x00]), None);
    }
}
