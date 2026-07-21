//! Acceptance test for the `Deterministic, offline generation` requirement.
//!
//! Scenario: *Reproducible build* — "generation runs twice with identical inputs
//! and parameters" ⇒ "the two output databases are byte-for-byte identical".
//!
//! This drives the real `tetra3-gen-db` binary end to end rather than calling the
//! library, because the determinism guarantee is a property of the whole pipeline
//! (catalog order, thinning, lattice enumeration, hash insertion, ZIP framing) and
//! a library-level test would not exercise the archive writer where the most
//! likely nondeterminism — an embedded timestamp — actually lives.

use std::fs;
use std::path::Path;
use std::process::Command;

const GEN_DB: &str = env!("CARGO_BIN_EXE_database-generation");

/// A 28-byte BSC5 header. A negative entry count selects the J2000 equinox.
fn bsc5_header(starn: i32) -> Vec<u8> {
    let mut header = vec![0u8; 28];
    header[8..12].copy_from_slice(&starn.to_le_bytes());
    header
}

/// One 32-byte BSC5 record.
fn bsc5_entry(id: u32, ra: f64, dec: f64, mag_raw: i16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    buf.extend_from_slice(&(id as f32).to_le_bytes());
    buf.extend_from_slice(&ra.to_le_bytes());
    buf.extend_from_slice(&dec.to_le_bytes());
    buf.extend_from_slice(&0i16.to_le_bytes()); // spectral type, ignored
    buf.extend_from_slice(&mag_raw.to_le_bytes());
    buf.extend_from_slice(&0f32.to_le_bytes()); // pmRA
    buf.extend_from_slice(&0f32.to_le_bytes()); // pmDec
    buf
}

/// A deterministic synthetic catalog: `n` stars on a Fibonacci sphere, so they are
/// spread widely enough to survive density thinning and dense enough to form
/// patterns. No RNG — the fixture must be identical on every run and machine, or
/// the test would be measuring the fixture rather than the generator.
fn synthetic_bsc5(n: u32) -> Vec<u8> {
    let mut data = bsc5_header(-(n as i32));
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    for i in 0..n {
        let t = (i as f64 + 0.5) / n as f64;
        let dec = (1.0 - 2.0 * t).asin(); // radians, −π/2..π/2
        let ra = (golden * i as f64).rem_euclid(std::f64::consts::TAU);
        // Magnitudes 1.00..~5.00 so brightest-first ordering is well defined and
        // every star clears the explicit magnitude limit used below.
        let mag_raw = 100 + (i as i16 % 400);
        data.extend_from_slice(&bsc5_entry(i + 1, ra, dec, mag_raw));
    }
    data
}

/// Run the generator. Every parameter that could otherwise float is pinned —
/// notably `--epoch-proper-motion`, which defaults to the *current year* and would
/// make output depend on the wall clock.
fn generate(catalog: &Path, out: &Path, extra: &[&str]) {
    let mut cmd = Command::new(GEN_DB);
    cmd.arg(catalog)
        .arg(out)
        .args(["--max-fov", "90"])
        .args(["--epoch-proper-motion", "2026.0"])
        .args(["--star-max-magnitude", "6.0"])
        .args(["--lattice-field-oversampling", "1"])
        .args(["--patterns-per-lattice-field", "4"])
        .args(extra);
    let output = cmd.output().expect("failed to run tetra3-gen-db");
    assert!(
        output.status.success(),
        "tetra3-gen-db failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reproducible_build() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = dir.path().join("bsc5.bin");
    fs::write(&catalog, synthetic_bsc5(400)).unwrap();

    let first = dir.path().join("first.npz");
    let second = dir.path().join("second.npz");
    generate(&catalog, &first, &[]);
    generate(&catalog, &second, &[]);

    let a = fs::read(&first).unwrap();
    let b = fs::read(&second).unwrap();

    // Guard against a vacuous pass: two empty or trivially small files would
    // compare equal while proving nothing about the pipeline.
    assert!(
        a.len() > 1024,
        "generated database is implausibly small ({} bytes) — the run produced \
         nothing to compare",
        a.len()
    );
    assert_eq!(
        a,
        b,
        "two runs with identical inputs produced different databases \
         ({} vs {} bytes)",
        a.len(),
        b.len()
    );
}

/// The byte comparison above has a blind spot, and this closes it.
///
/// Two runs 10 ms apart land in the same clock tick, so an archive that embedded
/// the *current* time would still compare equal — the test would only fail on the
/// rare run that straddled a boundary. Measured: re-deriving the ZIP mtime from
/// `SystemTime::now()` at one-second granularity left `reproducible_build`
/// passing. So assert the pinned timestamp directly, where run duration cannot
/// hide it.
///
/// Read from the raw bytes rather than through the `zip` crate: `zip` is a normal
/// dependency of this crate, not a dev-dependency, so an integration test cannot
/// link it — and parsing the four bytes the format defines is not worth adding one.
#[test]
fn archive_timestamps_are_pinned_not_wall_clock() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = dir.path().join("bsc5.bin");
    fs::write(&catalog, synthetic_bsc5(400)).unwrap();
    let out = dir.path().join("db.npz");
    generate(&catalog, &out, &[]);

    // MS-DOS date/time, as stored in every local file header at offsets 10..14:
    //   time = (hour << 11) | (minute << 5) | (second / 2)
    //   date = ((year - 1980) << 9) | (month << 5) | day
    // The generator pins 2000-01-01 00:00:00 ⇒ time 0x0000, date 0x2821.
    const PINNED_TIME: u16 = 0x0000;
    const PINNED_DATE: u16 = (20 << 9) | (1 << 5) | 1;

    let bytes = fs::read(&out).unwrap();
    let mut headers = 0;
    for i in 0..bytes.len().saturating_sub(14) {
        if &bytes[i..i + 4] != b"PK\x03\x04" {
            continue;
        }
        headers += 1;
        let time = u16::from_le_bytes([bytes[i + 10], bytes[i + 11]]);
        let date = u16::from_le_bytes([bytes[i + 12], bytes[i + 13]]);
        assert_eq!(
            (time, date),
            (PINNED_TIME, PINNED_DATE),
            "local file header {headers} carries mtime {time:#06x}/{date:#06x} \
             instead of the pinned 2000-01-01 — the archive is clock-dependent \
             and builds are not reproducible across a timestamp boundary"
        );
    }
    assert!(
        headers > 0,
        "found no ZIP local file headers in the generated database — this test \
         asserted nothing"
    );
}

/// Calibration for the two-run comparison. `assert_eq!` on two byte vectors only means
/// something if it is capable of failing, so change one parameter that must alter
/// the output — the hash-table type, which resizes the table from `next_prime(2·N)`
/// to `next_prime(3·N)` — and require the bytes to differ. If this ever passes by
/// finding the files equal, `reproducible_build` has stopped testing anything.
#[test]
fn byte_comparison_can_detect_a_difference() {
    let dir = tempfile::tempdir().unwrap();
    let catalog = dir.path().join("bsc5.bin");
    fs::write(&catalog, synthetic_bsc5(400)).unwrap();

    let quadratic = dir.path().join("quadratic.npz");
    let linear = dir.path().join("linear.npz");
    generate(&catalog, &quadratic, &[]);
    generate(&catalog, &linear, &["--linear-probe"]);

    assert_ne!(
        fs::read(&quadratic).unwrap(),
        fs::read(&linear).unwrap(),
        "--linear-probe produced a byte-identical database, so the byte \
         comparison in reproducible_build cannot detect a difference"
    );
}
