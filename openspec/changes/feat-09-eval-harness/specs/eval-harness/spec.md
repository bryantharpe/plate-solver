## ADDED Requirements

### Requirement: Shared image corpus

The system SHALL exercise all three plate-solving implementations against a single, fixed,
checked-in image corpus rather than per-system inputs. The corpus SHALL consist of the 11 images
byte-identical (verified by md5sum) between `reference-solutions/cedar-detect/test_data/` and
`reference-solutions/cedar-solve/examples/data/medium_fov/`: 9 astronomical images (the 8
`2019-07-29T204726_AltXX_AziYYY_Try1.jpg` real-sky photos plus `hale_bopp.jpg`) for which a valid
solve is expected, and 2 non-astronomical images (`tree.jpg`, `test_5mp_g100_e50ms.jpg`) for which
no valid solve is expected. (Ref: verified corpus overlap; `tools/parity/capture_solve.py`'s
existing reference image `2019-07-29T204726_Alt40_Azi-135_Try1.jpg` is included for continuity.)

#### Scenario: Corpus is identical across systems
- **WHEN** the harness runs
- **THEN** every one of the 11 corpus images is fed to all three systems (tetra3-original,
  cedar-flow, the new workflow) with no system receiving a different or additional image

#### Scenario: Astronomical images attempt a full solve
- **WHEN** one of the 9 astronomical images is processed
- **THEN** all three systems attempt detection and a full solve, with `MATCH_FOUND` expected as
  the nominal outcome

#### Scenario: Stress images are bounded, not skipped
- **WHEN** one of the 2 non-astronomical images is processed
- **THEN** all three systems still attempt the full detect+solve pipeline (not detection-only),
  but with 1 iteration and a 5-second solve timeout, bounding the cost of a known no-match
  combinatorial search in the new workflow's solver (`CODEBASE-REVIEW.md` `C1`)

### Requirement: Direct three-system comparison

The system SHALL measure each implementation through a uniform adapter interface, calling gRPC
endpoints directly for the two systems that expose a gRPC server (cedar-detect, the new
workflow's `ps-grpc`) rather than through any additional client-side wrapper, and calling
original tetra3 in-process (as a Python library call, since it has no gRPC server). (Ref: user
instruction that library performance is the target and gRPC is an acceptable, already-fair
interface for systems that expose it; `ps-grpc/proto/plate_solver.proto`'s `CentroidsRequest`/
`CentroidsResult` are wire-compatible with `cedar_detect.proto`, confirmed by an existing
`ps-grpc` interop test.)

#### Scenario: cedar-detect measured via its own gRPC server
- **WHEN** the cedar-flow adapter measures detection
- **THEN** it calls the `cedar-detect-server` binary's `CedarDetect/ExtractCentroids` RPC directly,
  and solves the returned centroids in-process via cedar-solve's Python `solve_from_centroids`

#### Scenario: New workflow measured via its own gRPC server
- **WHEN** the new-workflow adapter measures detection or solving
- **THEN** it calls the `ps-grpc` service's `ExtractCentroids`/`SolveFromImage`/
  `SolveFromCentroids` RPCs directly, against a database converted from cedar-solve's catalog

#### Scenario: tetra3-original measured in-process
- **WHEN** the tetra3-original adapter measures detection or solving
- **THEN** it calls the Python `tetra3` library's `get_centroids_from_image` /
  `solve_from_image` directly, with no gRPC server involved, because tetra3-original has none

### Requirement: Dual timing capture

The system SHALL record, for every stage of every system where available, both the client-observed
wall-clock time (measured around the call) and the system's own self-reported algorithm-only time,
excluding image decode from all timed regions. (Ref: `CentroidsResult.algorithm_time` on both
`cedar_detect.proto` and `plate_solver.proto`, self-reported server-side via `Instant::now()`/
`elapsed()`; tetra3/cedar-solve's own solve-result timing keys.)

#### Scenario: Both timings recorded for a gRPC call
- **WHEN** the harness calls `ExtractCentroids` on either gRPC server
- **THEN** it records both the round-trip wall-clock time and the response's `algorithm_time`
  field

#### Scenario: New workflow's `t_extract_ms` is never used for extraction timing
- **WHEN** the harness needs the new workflow's extraction-only time
- **THEN** it obtains it from a standalone `ExtractCentroids` call's `algorithm_time`, not from
  `Solution.t_extract_ms`, because `ps-grpc/src/service.rs` hard-codes that field to `0.0` for
  both `SolveFromCentroids` and `SolveFromImage` (`CODEBASE-REVIEW.md` `C2`)

#### Scenario: Image decode excluded from timing
- **WHEN** any system's detect or solve stage is timed
- **THEN** the image has already been decoded to raw grayscale pixels before the timed region
  begins, for every system and every iteration

### Requirement: Correctness parity check

The system SHALL compare each astronomical image's solve output (RA, Dec, Roll, FOV, matched-star
count, matched catalog IDs, status) across systems using tolerances already established in this
repo, treating the same-catalog comparison as primary and any comparison involving
tetra3-original as an explicitly labeled cross-catalog check. (Ref:
`openspec/IMPLEMENTATION-STATUS.md`: RA/Dec within 10 arcsec, matched catalog IDs exact/near-exact,
detection centroids within ±0.1px; tetra3-original's catalog incompatibility, verified by
comparing both `default_database.npz` files' `props_packed` contents directly.)

#### Scenario: Same-catalog comparison is the primary parity gate
- **WHEN** the new workflow and cedar-flow solve the same astronomical image
- **THEN** their RA/Dec are compared within 10 arcsec and their matched catalog IDs compared for
  exact/near-exact agreement, since both solve against the same converted cedar-solve catalog

#### Scenario: Cross-catalog comparisons are labeled, not silently strict
- **WHEN** tetra3-original's output is compared against either the new workflow's or cedar-flow's
  output
- **THEN** the comparison is recorded and reported as a "cross-catalog sanity check", not folded
  into the primary parity pass/fail

#### Scenario: A mismatch is flagged, not fatal
- **WHEN** any pairwise comparison for any image exceeds its tolerance
- **THEN** the run continues to completion, the mismatch is recorded with `flagged: true`, and it
  appears in the rendered report rather than being silently dropped or aborting the run

#### Scenario: Stress-image status is still checked
- **WHEN** either non-astronomical image is solved by any system
- **THEN** its solve `status` is checked to be `NO_MATCH` or `TOO_FEW` (RA/Dec are not compared,
  since no valid solution exists)

### Requirement: Human- and machine-readable output

The system SHALL persist raw results as machine-readable `results.json` and deterministically
render two human-readable documents, `docs/benchmarks/report.md` and `docs/benchmarks/report.html`
(self-contained, with no CDN or external asset dependency), from that same `results.json`. (Ref:
`docs/screenshots/` establishes `docs/` as the repo's existing home for generated documentation
artifacts.)

#### Scenario: Reports regenerate deterministically
- **WHEN** the report renderer runs twice against the same, unmodified `results.json`
- **THEN** it produces byte-identical `report.md` and `report.html` output both times

#### Scenario: HTML report is self-contained
- **WHEN** `report.html` is opened directly from disk with no network access
- **THEN** it renders fully, with no broken references to a CDN, external stylesheet, or script

### Requirement: Re-runnable on demand

The system SHALL be executable via a single documented command sequence after one-time
environment setup, so it can be re-run after future changes to `ps-detect`, `ps-solve`, or
`ps-grpc` to judge whether performance changed. (Ref: user's stated purpose — "judge performance
over time after we change things".)

#### Scenario: A fresh run after a code change
- **WHEN** `ps-detect`, `ps-solve`, or `ps-grpc` changes and the documented run command is
  executed again (release rebuild + `run_benchmark.py` + `report.py`)
- **THEN** a new `results.json` and pair of reports are produced reflecting the current code,
  without requiring any change to the harness itself

### Requirement: Environment disclosure

The rendered report SHALL state the measurement host's architecture and CPU count, note
explicitly that it is not the PRD's RPi-4B-class/mobile target hardware, and cite the known,
already-tracked limitations that affect interpretation of the results (the tetra3-original
cross-catalog caveat, `CODEBASE-REVIEW.md` `C1`, and the `t_extract_ms`/`C2` caveat) rather than
omitting them. (Ref: `openspec/PRD.md`'s non-functional requirements table specifies RPi-4B-class
detection and desktop-class solve latency targets that this harness's host does not represent.)

#### Scenario: Report states its measurement environment
- **WHEN** a report is rendered
- **THEN** its methodology section names the host OS/architecture/CPU count and states that these
  results do not represent the PRD's mobile/RPi-4B target hardware

#### Scenario: Known limitations are cited, not hidden
- **WHEN** a report includes a number affected by a known limitation (e.g. the new workflow's
  extraction timing, or tetra3-original's cross-catalog comparison)
- **THEN** that number's table or caption cites the relevant limitation by name

### Requirement: Reference-solutions stay read-only

The system SHALL treat `reference-solutions/` as read-only, writing any artifact derived from it
(such as a native-format copy of a reference `.npz` catalog) to generated, gitignored output paths
rather than modifying the reference trees. (Ref: `tools/parity/README.md`: "`reference-solutions/`
is **read-only**; we install it (editable) but never modify it.")

#### Scenario: Derived database is written outside reference-solutions
- **WHEN** the shared native-format catalog is generated from `default_database.npz`
- **THEN** it is written under `tools/parity/benchmark/generated/`, and no file under
  `reference-solutions/` is created, modified, or deleted
