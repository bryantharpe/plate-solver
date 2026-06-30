## ADDED Requirements

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

### Requirement: Transport and metadata

The system SHALL serve over TCP (default `127.0.0.1:50051`, configurable) and additionally accept
gRPC-Web over HTTP/1, and `GetInfo` SHALL report the server version, the loaded database
properties (FOV range, catalog, epochs, pattern count), and supported features. (Ref: doc 07
§2–3.)

#### Scenario: GetInfo reports database FOV range
- **WHEN** a client calls `GetInfo`
- **THEN** the response includes the loaded database's `[min_fov, max_fov]` and pattern count
