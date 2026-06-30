## 1. Crate & proto

- [x] 1.1 Create the `ps-grpc` crate depending on `ps-detect`, `ps-solve`, `ps-db`, `tonic`/`prost`, `tonic-web`, `prost-types`
- [x] 1.2 Author `plate_solver.proto` (PlateSolver service + Image/ImageCoord/StarCentroid/Solution/SolveParams/ServerInfo)
- [x] 1.3 Wire `build.rs` (`tonic-build`) to compile the proto

## 2. Extraction RPC

- [x] 2.1 Implement `ExtractCentroids` over `ps-detect` (inline image path)
- [x] 2.2 Implement the shared-memory mapping path with `reopen_shmem` handling and INTERNAL-on-failure
- [x] 2.3 Fill `noise_estimate`, `hot_pixel_count`, `peak_star_pixel`, `algorithm_time`; brightest-first centroids

## 3. Solve RPCs

- [x] 3.1 Implement `SolveFromCentroids` (the `(x,y)→(y,x)` swap, forward `SolveParams` to `ps-solve`)
- [x] 3.2 Implement `SolveFromImage` (extract then solve; record `t_extract`/`t_solve`)
- [x] 3.3 Map `ps-solve` outputs (incl. status enum, optional matched stars) to the `Solution` message

## 4. Metadata & transport

- [x] 4.1 Implement `GetInfo` (version + loaded database properties)
- [ ] 4.2 Serve over TCP (configurable port) and accept gRPC-Web over HTTP/1
- [x] 4.3 Error/timeout mapping to gRPC status codes

## 5. Validation

- [x] 5.1 `ExtractCentroids` interop test against a cedar-detect-style client
- [x] 5.2 End-to-end `SolveFromImage` test on a reference image (parity with `ps-solve`)
