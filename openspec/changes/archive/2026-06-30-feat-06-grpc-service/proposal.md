## Why

Consumers integrate the solver through one clean, language-agnostic contract: hand in a camera
frame plus an approximate FOV, get back an attitude. This change specifies the `ps-grpc` crate —
the `PlateSolver` gRPC service that exposes detection and solving, reusing cedar-detect's proven
`Image`/`ImageCoord` message shapes and shared-memory fast path. Grounded in
`reference-solutions/docs/07-cedar-detect-service-api.md` and the `cedar_detect.proto`, extended
with the solve RPCs the product needs.

## What Changes

- Introduce the `ps-grpc` crate and the `grpc-service` capability: a `PlateSolver` service with
  `ExtractCentroids`, `SolveFromCentroids`, `SolveFromImage`, and `GetInfo`.
- Define the message schemas: `Image` (inline `image_data` or `shmem_name`), `ImageCoord`
  (`(x,y)`, `(0.5,0.5)` pixel center), `StarCentroid`, and a new `Solution` message
  (RA/Dec/Roll/FOV/distortion/RMSE/P90E/MAXE/matches/prob/status/timings).
- Specify the `(x,y)↔(y,x)` boundary swap, the shared-memory fast path with INTERNAL→inline
  fallback, transport (TCP + gRPC-Web), and errors/timeouts. Stack: `tonic`/`prost`.

## Capabilities

### New Capabilities

- `grpc-service`: the `PlateSolver` gRPC API — `ExtractCentroids`, `SolveFromCentroids`,
  `SolveFromImage`, `GetInfo`; message schemas; coordinate-boundary swap; shared-memory fast
  path with inline fallback.

### Modified Capabilities

(none.)

## Impact

- New crate `ps-grpc` depending on `ps-detect`, `ps-solve`, `ps-db`, `tonic`/`prost`,
  `tonic-web`, `prost-types`; a `build.rs` compiles the proto via `tonic-build`.
- The consumer-facing contract; also the basis for the on-device server in
  `feat-07-mobile-runtime`.
- Backward-compatible with `cedar_detect.proto` message shapes for `ExtractCentroids`.
