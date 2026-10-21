## ADDED Requirements

### Requirement: Cedar's detection settings

The system SHALL provide `DetectParams::cedar()`: the settings cedar-solve's example script
passes to cedar-detect, with which the parity goldens' star lists were made. They are sigma 8,
2×2 binning, rows levelled, hot pixels rejected and star candidates taken from the binned
image, with no binned image returned and the noise estimated from the image.
`DetectParams::default()` SHALL be unchanged: binning 1, no row levelling and no hot-pixel
rejection. A frame turned gray with `star_detection::gray` and detected with these settings
solves to cedar's answer; the finder console and the `solve_image` example both read frames
this way. (Ref:
`crates/plate-solver/src/solve.rs` `DetectParams::cedar`,
`crates/plate-solver/examples/solve_image.rs`; tests: `cedar_detects_as_its_example_script_does`,
and `every_sample_gives_cedars_answer` in `crates/finder/tests/api.rs`.)

#### Scenario: The settings
- **WHEN** `DetectParams::cedar()` is read
- **THEN** sigma is 8 and binning 2, rows are levelled, hot pixels are rejected, candidates
  come from the binned image, nothing is returned binned and no noise estimate is fixed

#### Scenario: The default is unchanged
- **WHEN** `DetectParams::default()` is read
- **THEN** binning is 1, and rows are not levelled nor hot pixels rejected
