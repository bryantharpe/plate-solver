## ADDED Requirements

### Requirement: In-process embedding via UniFFI

The system SHALL expose the detect→solve pipeline through UniFFI bindings so an iOS (Swift) or
Android (Kotlin) app can call it in-process without a network hop, providing at least a
`solve_from_image(bytes, width, height, fov_estimate, …) → Solution` entry point and a database
handle that is loaded once and reused. (Ref: project.md §5; PRD goals.)

#### Scenario: In-process solve without a socket
- **WHEN** a mobile app calls the UniFFI `solve_from_image` with a camera frame and FOV estimate
- **THEN** it receives a `Solution` without any network round-trip

#### Scenario: Database loaded once and reused
- **WHEN** multiple solves run in a session
- **THEN** the database handle is loaded once and shared across calls

### Requirement: Optional in-process gRPC server

The system SHALL optionally provide (behind a feature flag) the `PlateSolver` gRPC service
running in-process over a local channel as an alternative to UniFFI, so a consumer that already
speaks gRPC can integrate unchanged. When run on-device, it SHALL bind to a local endpoint only.
(Ref: feat-06-grpc-service; doc 07.)

#### Scenario: Local-only binding on device
- **WHEN** the in-process gRPC server runs on a phone
- **THEN** it binds to a local endpoint and is not exposed off-device

### Requirement: Memory-mapped database

The system SHALL load the pattern database via memory mapping using the linear-probe table
layout so that probe chains are contiguous and the whole table need not be resident in RAM,
keeping peak memory bounded for narrow-FOV / large databases on device. (Ref: feat-03-pattern-database;
doc 02 §6.3; doc 05 §6.1.)

#### Scenario: Bounded resident memory
- **WHEN** a large device database is opened via mmap
- **THEN** lookups touch only the probe-chain pages, not the entire table, keeping resident
  memory within the budget

### Requirement: Performance and memory budgets

The system SHALL document and enforce per-platform budgets as measurable targets: an extraction
latency target consistent with cedar-detect's <10 ms/Mpx class scaled to the device, a per-solve
latency target (reference ~10 ms/solve on desktop; a documented mobile target), a bounded
`solve_timeout`, a database on-disk size bound, a peak-RAM ceiling, and a startup/mmap time
bound. Each budget SHALL have a concrete number recorded for the target device and a test that
fails if exceeded. (Ref: PRD non-functional requirements; docs 04/06/08.)

#### Scenario: Solve within the documented latency budget
- **WHEN** a reference image is solved on the target phone with the bundled FOV-matched database
- **THEN** the solve completes within the documented per-platform latency budget

#### Scenario: Database fits the RAM ceiling
- **WHEN** the bundled database is loaded on device
- **THEN** peak resident memory stays within the documented ceiling

### Requirement: Threading and cancellation model

The system SHALL use a bounded threading model on device: parallelism (e.g. `rayon`) SHALL be
feature-gated and either off or bounded to a small thread count, work SHALL not block the UI
thread, and a solve SHALL be cancellable (cooperatively, surfacing `CANCELLED`). (Ref: doc 06
§7; project.md §5.)

#### Scenario: Solve does not block the UI thread
- **WHEN** a solve runs from a mobile app
- **THEN** it executes off the UI thread and can be cancelled mid-solve

#### Scenario: Parallelism is bounded
- **WHEN** the on-device build enables parallelism
- **THEN** it is bounded to a small, configured thread count (not unbounded)

### Requirement: Input handling

The system SHALL accept the platform camera frame, convert it to 8-bit grayscale at the boundary
(the detection pipeline is 8-bit only), and apply the `(y,x)` / `(0.5,0.5)` pixel-center
conventions consistently. (Ref: project.md §4; feat-02-star-detection.)

#### Scenario: Color frame converted at the boundary
- **WHEN** a color or high-bit camera frame is supplied
- **THEN** it is converted to 8-bit grayscale before detection

### Requirement: Dependency minimization

The system SHALL avoid heavy or native-incompatible dependencies in the mobile build, keep the
core pure-Rust, and feature-gate anything optional (parallelism, the gRPC server) so a minimal
on-device build excludes them. (Ref: PRD portability requirement; project.md §5.)

#### Scenario: Minimal build excludes optional features
- **WHEN** the mobile crate is built with default features for a phone
- **THEN** optional heavy features (e.g. the gRPC server, rayon) are excluded unless explicitly enabled

### Requirement: Packaging

The system SHALL package the bindings for both platforms: an iOS `xcframework` exposing the Swift
API and an Android `.aar`/JNI library exposing the Kotlin API, each bundling or referencing a
FOV-matched database. (Ref: project.md §5; PRD goals.)

#### Scenario: iOS artifact
- **WHEN** the iOS package is built
- **THEN** it produces an `xcframework` an app can link, with the Swift `solve_from_image` API

#### Scenario: Android artifact
- **WHEN** the Android package is built
- **THEN** it produces an `.aar`/JNI library with the Kotlin `solve_from_image` API
