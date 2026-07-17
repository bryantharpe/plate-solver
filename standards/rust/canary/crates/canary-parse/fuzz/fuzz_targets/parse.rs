#![no_main]
//! Fuzz target: the contract is that neither parser panics, reads out of bounds,
//! or hangs on **any** input. cargo-fuzz feeds arbitrary bytes; Miri and ASan
//! (via libFuzzer) would catch memory unsoundness in the `unsafe` path.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = canary_parse::parse_header(data);
    let _ = canary_parse::ascii_tag(data);
});
