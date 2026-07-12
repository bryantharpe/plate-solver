# grpc-service Specification

## Purpose

The consumer contract — the `PlateSolver` gRPC service through which integrators in any language
reach the pipeline. Four RPCs: `ExtractCentroids` (image → centroids), `SolveFromCentroids`
(centroids + FOV → solution), `SolveFromImage` (image + FOV → solution, detecting internally),
and `GetInfo` (server/database metadata). It reuses cedar-detect's `Image` / `ImageCoord` message
shapes so the two services interoperate on the wire.

This is the boundary where two kinds of drift are caught. **Coordinate drift:** gRPC speaks
`(x, y)` and the solver speaks `(y, x)` — the swap happens here and nowhere else. **Detection
drift:** `SolveFromImage` is the only RPC that detects server-side, so it must honor the client's
detection parameters and estimate noise from the image exactly as `ExtractCentroids` does. If the
two paths disagree about detection, the same image solves through one and fails through the
other — which is a correctness bug at the public API, not a performance quirk.

The service also offers a shared-memory fast path for large images, with an inline fallback that
must always work.
## Requirements
### Requirement: PlateSolver service surface

The system SHALL expose a gRPC `PlateSolver` service with four RPCs: `ExtractCentroids`
(image → centroids), `SolveFromCentroids` (centroids + FOV → solution), `SolveFromImage`
(image + FOV → solution, extracting centroids internally), and `GetInfo` (server/database
metadata). (Ref: doc 07; PRD goals.)

#### Scenario: All four RPCs available
- **WHEN** a client connects to the `PlateSolver` service
- **THEN** it can call `ExtractCentroids`, `SolveFromCentroids`, `SolveFromImage`, and `GetInfo`

#### Scenario: SolveFromImage is one call end-to-end
- **WHEN** a client calls `SolveFromImage` with an image and a FOV estimate
- **THEN** the service extracts centroids and returns a `Solution` in a single response

### Requirement: Image message

The system SHALL define an `Image` message with `width`, `height`, row-major 8-bit grayscale
`image_data` (omitted when `shmem_name` is set), an optional `shmem_name` naming a POSIX shared
memory object holding the pixels, and a `reopen_shmem` flag signalling the server to reopen a
resized buffer. (Ref: doc 07 §1.)

#### Scenario: Inline image bytes
- **WHEN** `image_data` is populated and `shmem_name` is absent
- **THEN** the server reads pixels from `image_data`

#### Scenario: Shared-memory image
- **WHEN** `shmem_name` is set
- **THEN** the server maps that shared memory for pixels and ignores `image_data`

### Requirement: ImageCoord and centroid messages

The system SHALL define `ImageCoord { double x; double y; }` where `(0.5, 0.5)` is the center of
the top-left pixel, and `StarCentroid { ImageCoord centroid_position; double brightness; int32
num_saturated; }`. `ExtractCentroids` SHALL return centroids ordered brightest-first along with
`noise_estimate`, `hot_pixel_count`, and `algorithm_time`. (Ref: doc 07 §1.)

#### Scenario: Pixel-center convention on the wire
- **WHEN** a centroid is returned
- **THEN** `(0.5, 0.5)` denotes the center of the top-left pixel

#### Scenario: Brightest-first centroids
- **WHEN** `ExtractCentroids` returns candidates
- **THEN** they are ordered by descending brightness

### Requirement: Solution message

The system SHALL define a `Solution` message carrying `ra`, `dec`, `roll`, `fov` (degrees),
`distortion`, `rmse`, `p90e`, `maxe` (arcsec), `matches`, `prob`, a `status` enum
(`MATCH_FOUND`, `NO_MATCH`, `TIMEOUT`, `CANCELLED`, `TOO_FEW`), `t_extract` and `t_solve`
(milliseconds), and optional matched-star data (centroids, RA/Dec/mag, catalog IDs) when
requested. (Ref: doc 06 §6; doc 07.)

#### Scenario: Attitude fields on success
- **WHEN** a solve succeeds
- **THEN** the `Solution` has `status = MATCH_FOUND` with `ra`, `dec`, `roll`, `fov` populated

#### Scenario: Status on failure
- **WHEN** a solve fails or times out
- **THEN** the `Solution` carries the corresponding status and unset attitude fields

### Requirement: Coordinate boundary swap

The system SHALL convert between the wire `ImageCoord (x, y)` and the solver's `(y, x)`
convention at the service boundary: centroids received as `(x, y)` are passed to the solver as
`(y, x)`, and solver outputs are emitted as `(x, y)`. (Ref: doc 07 §1, §6; project.md §4.)

#### Scenario: x/y swapped inbound and outbound
- **WHEN** a centroid crosses the service boundary
- **THEN** `ImageCoord (x, y)` maps to solver `(y, x)` inbound and back to `(x, y)` outbound

### Requirement: Shared-memory fast path with fallback

The system SHALL support a shared-memory image transport to avoid copying megabytes per frame on
one host: the server maps `shmem_name` read-only without copying. When shared memory cannot be
accessed, the RPC SHALL return gRPC `INTERNAL`, signalling the client to fall back permanently to
inline `image_data`. (Ref: doc 07 §2.)

#### Scenario: INTERNAL triggers inline fallback
- **WHEN** the server cannot map the request's shared memory
- **THEN** it returns `INTERNAL` and the client retries with inline `image_data`

#### Scenario: Resized buffer reopened
- **WHEN** `reopen_shmem` is set because the client grew the buffer
- **THEN** the server drops its cached descriptor and reopens the shared memory

### Requirement: Request parameters

The system SHALL accept detection/solve parameters on requests: `sigma` (significance, typical
5–10), `binning` (2 or 4), `detect_hot_pixels`, `normalize_rows`, `return_binned`, and an
optional background-estimation region for extraction; and `fov_estimate`, `fov_max_error`,
`match_radius`, `match_threshold`, `solve_timeout`, and `distortion` for solving. (Ref: doc 07
§1; doc 06 §2.)

#### Scenario: Solve parameters forwarded
- **WHEN** a solve request sets `fov_estimate` and `fov_max_error`
- **THEN** the service forwards them to the solver to bound and speed the search

### Requirement: SolveFromImage detection fidelity

`SolveFromImage` SHALL honor the detection fields of its `CentroidsRequest` (`extract`) —
`sigma`, `binning`, `detect_hot_pixels`, `normalize_rows`, `use_binned_for_star_candidates`,
`return_binned` — and SHALL estimate noise from the input image using the same estimation
`ExtractCentroids` uses, rather than hardcoding detection parameters. It is the only RPC that
performs detection server-side as part of a solve. It SHALL resolve effective binning by the
same rule `ExtractCentroids` uses, so the two paths cannot drift on detection semantics. A
client that requests `binning=2` or `sigma=8` and silently receives default detection behavior
SHALL be treated as a correctness-contract violation at the public API boundary, not a
performance nuance: detection parameters set the detection threshold, which determines which
centroids exist, which determines whether the image solves at all. (Ref: doc 07 §1.)

#### Scenario: Client sigma honored
- **WHEN** a client sends `SolveFromImage` with `extract.sigma = 8.0`
- **THEN** the internal detection runs with sigma=8.0, not a hardcoded default

#### Scenario: Client binning honored
- **WHEN** a client sends `SolveFromImage` with `extract.binning = 2` and
  `use_binned_for_star_candidates = true`
- **THEN** the internal detection runs with effective binning 2

#### Scenario: Noise estimated, not hardcoded
- **WHEN** `SolveFromImage` runs
- **THEN** the detection noise estimate is derived from the input image — the same value an
  `ExtractCentroids` call on the same image would report as `noise_estimate`
- **AND** it is not a constant such as `1.0`

#### Scenario: Effective binning shared with ExtractCentroids
- **WHEN** `SolveFromImage` resolves effective binning
- **THEN** it uses the same effective-binning rule as `ExtractCentroids` (None→2,
  Some(2)|Some(4)→that value, other→`INVALID_ARGUMENT` when binning is requested; 1 otherwise)

#### Scenario: Consistent with the composed ExtractCentroids + SolveFromCentroids path
- **WHEN** `SolveFromImage` is called with the same `extract` detection fields as a paired
  `ExtractCentroids` call on the same image
- **THEN** the recovered attitude and matched catalog IDs agree with the composed
  `ExtractCentroids` + `SolveFromCentroids` path within parity tolerances (RA/Dec within 10
  arcsec, matched IDs exact)

#### Scenario: Hardcoded-noise regression guard
- **WHEN** `SolveFromImage` is called on `reference-solutions/cedar-detect/test_data/hale_bopp.jpg`
  with `extract.sigma = 8.0`
- **THEN** the result is `MATCH_FOUND` within parity tolerance of the attitude the composed
  `ExtractCentroids` + `SolveFromCentroids` path recovers for the same image and sigma
- **AND** it does not exhaust the combination space and return `NO_MATCH` — the failure mode a
  hardcoded `noise_estimate` produces on this image, because a wrong threshold yields too few
  and wrong centroids

### Requirement: Transport and metadata

The system SHALL serve over TCP (default `127.0.0.1:50051`, configurable) and additionally accept
gRPC-Web over HTTP/1, and `GetInfo` SHALL report the server version, the loaded database
properties (FOV range, catalog, epochs, pattern count), and supported features. (Ref: doc 07
§2–3.)

#### Scenario: GetInfo reports database FOV range
- **WHEN** a client calls `GetInfo`
- **THEN** the response includes the loaded database's `[min_fov, max_fov]` and pattern count

