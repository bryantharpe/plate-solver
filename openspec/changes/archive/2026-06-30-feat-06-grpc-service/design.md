## Context

`ps-grpc` is the consumer contract. cedar-detect already ships a proven `ExtractCentroids` gRPC
service (`cedar_detect.proto`, doc 07) with an `Image` inline-or-shared-memory duality and an
`ImageCoord (x,y)` convention. We **extend** that surface into a `PlateSolver` service that also
solves, reusing the message shapes verbatim where possible so existing cedar-detect clients keep
working. Stack mirrors cedar-detect: `tonic` 0.11 / `prost` 0.12, `tonic-web`, `prost-types`,
built by `tonic-build` in `build.rs`.

## Goals / Non-Goals

**Goals:**
- One service that does extraction and solving: `ExtractCentroids`, `SolveFromCentroids`,
  `SolveFromImage`, `GetInfo`.
- Reuse `Image`/`ImageCoord`/`StarCentroid` from cedar-detect; add `Solution` and request types.
- Shared-memory fast path with INTERNAL→inline fallback; TCP + gRPC-Web transport.
- The `(x,y)↔(y,x)` swap localized to the boundary.

**Non-Goals:**
- The detection and solve algorithms (those are `ps-detect`/`ps-solve`); this crate is transport
  + marshalling only.
- Authentication / multi-tenant serving (local/in-process or localhost use; out of scope for v1).
- Streaming/video RPC (post-v1).

## Decisions

- **Extend, don't fork, the cedar-detect proto.** Keep `Image`, `ImageCoord`, `StarCentroid`,
  `Rectangle`, and the `CentroidsRequest/Result` shapes so cedar-detect clients are
  drop-in for `ExtractCentroids`; add a `PlateSolver` service and solve messages alongside.
- **`(x,y)` on the wire, `(y,x)` in the solver.** The swap happens once, at the handler boundary,
  exactly as the reference clients do (`tetra_centroid = (sc.y, sc.x)`). Internal code is always
  `(y,x)`.
- **Shared memory optional, inline authoritative.** Prefer `shm_open`+`mmap` zero-copy on-host;
  on any shmem failure return `INTERNAL` so the client falls back permanently to inline bytes —
  the documented contract that makes the service host-portable.
- **`tonic`/`prost` 0.11/0.12** to match cedar-detect exactly (lowest interop risk); `build.rs`
  compiles the proto.
- **`GetInfo` exposes DB properties** so a client can confirm the loaded FOV range/catalog before
  solving.

### Proto sketch

```proto
syntax = "proto3";
package plate_solver;
import "google/protobuf/duration.proto";

service PlateSolver {
  rpc ExtractCentroids(CentroidsRequest) returns (CentroidsResult);
  rpc SolveFromCentroids(SolveFromCentroidsRequest) returns (Solution);
  rpc SolveFromImage(SolveFromImageRequest) returns (Solution);
  rpc GetInfo(InfoRequest) returns (ServerInfo);
}

message Image {              // reused from cedar_detect.proto
  int32 width = 1; int32 height = 2;
  bytes image_data = 3;                 // row-major uint8 gray; omitted if shmem_name set
  optional string shmem_name = 4; bool reopen_shmem = 5;
}
message ImageCoord { double x = 1; double y = 2; }   // (0.5,0.5)=center of top-left pixel
message StarCentroid { ImageCoord centroid_position = 1; double brightness = 4; int32 num_saturated = 6; }
message Rectangle { int32 origin_x = 1; int32 origin_y = 2; int32 width = 3; int32 height = 4; }

message CentroidsRequest {
  Image input_image = 1; double sigma = 2; optional int32 binning = 8;
  bool return_binned = 4; bool use_binned_for_star_candidates = 5;
  bool detect_hot_pixels = 6; bool normalize_rows = 9;
  optional Rectangle estimate_background_region = 7;
}
message CentroidsResult {
  double noise_estimate = 1; optional double background_estimate = 7;
  int32 hot_pixel_count = 2; int32 peak_star_pixel = 6;
  repeated StarCentroid star_candidates = 3;     // brightest first
  optional Image binned_image = 4; google.protobuf.Duration algorithm_time = 5;
}

message SolveParams {
  optional double fov_estimate = 1; optional double fov_max_error = 2;
  optional double match_radius = 3; optional double match_threshold = 4;
  optional int32 solve_timeout_ms = 5; optional double distortion = 6;
  bool return_matches = 7; bool return_catalog = 8;
}
message SolveFromCentroidsRequest {
  repeated ImageCoord centroids = 1;   // (x,y); brightest-first
  int32 width = 2; int32 height = 3; SolveParams params = 4;
}
message SolveFromImageRequest { CentroidsRequest extract = 1; SolveParams params = 2; }

enum SolveStatus { MATCH_FOUND = 0; NO_MATCH = 1; TIMEOUT = 2; CANCELLED = 3; TOO_FEW = 4; }
message MatchedStar { ImageCoord centroid = 1; double ra = 2; double dec = 3; double mag = 4; int64 cat_id = 5; }
message Solution {
  SolveStatus status = 1;
  double ra = 2; double dec = 3; double roll = 4; double fov = 5; double distortion = 6;
  double rmse = 7; double p90e = 8; double maxe = 9; int32 matches = 10; double prob = 11;
  double t_extract_ms = 12; double t_solve_ms = 13;
  repeated MatchedStar matched = 14;     // when return_matches
}

message InfoRequest {}
message ServerInfo {
  string version = 1; string star_catalog = 2;
  double min_fov = 3; double max_fov = 4; int64 num_patterns = 5;
  double epoch_equinox = 6; double epoch_proper_motion = 7;
}
```

## Risks / Trade-offs

- [Image transfer dominates RPC time] → shared-memory fast path on-host; document that it only
  works when client and server share a host, with inline fallback otherwise.
- [tonic/prost version drift] → pin 0.11/0.12 to match cedar-detect; revisit as a deliberate bump.
- [Coordinate confusion] → single boundary swap, covered by a dedicated requirement + tests.
- [Mobile transport overhead] → the in-process server / UniFFI alternative is addressed in
  `feat-07-mobile-runtime`; this crate stays transport-pure.

## Migration Plan

Greenfield service crate. `ExtractCentroids` parity is checked against cedar-detect clients;
solve RPCs are checked end-to-end against `ps-solve` parity fixtures.

## Open Questions

- Whether `SolveFromImage` should also return the extracted centroids (debugging aid) — leaning
  yes behind a `return_matches`/debug flag, not by default (keeps the response small).
