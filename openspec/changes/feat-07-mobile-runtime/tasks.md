## 1. Crate & bindings

- [ ] 1.1 Establish the mobile-runtime embedding layer over the `star-detection`, `plate-solver`, and `pattern-database` capabilities (optionally `grpc-service`), with UniFFI
- [ ] 1.2 Define the UniFFI interface (DB handle, `solve_from_image`, `Solution` mapping) and generate Swift/Kotlin scaffolding

## 2. Database & memory

- [ ] 2.1 Wire the mmap (linear-probe) database loading path and a reusable DB handle
- [ ] 2.2 Implement camera-frame → 8-bit grayscale conversion at the boundary

## 3. Concurrency & features

- [ ] 3.1 Implement the bounded, off-UI threading model and cooperative cancellation
- [ ] 3.2 Feature-gate `rayon` and the in-process gRPC server; default-minimal mobile build
- [ ] 3.3 Optionally start the in-process gRPC server on a local-only endpoint

## 4. Budgets & tests

- [ ] 4.1 Define per-platform budgets (extract/solve latency, DB size, peak RAM, startup) with concrete targets
- [ ] 4.2 Add regression tests/benchmarks that fail when a budget is exceeded

## 5. Packaging

- [ ] 5.1 Build the iOS `xcframework` (Swift API)
- [ ] 5.2 Build the Android `.aar`/JNI library (Kotlin API)
- [ ] 5.3 Bundle or reference a FOV-matched database in each package
