## Why

The product target is a phone. Everything else in this spec set is written so the pipeline can
run on-device at bounded latency and memory — but "runs on a phone" needs its own contract:
in-process embedding without a network hop, a memory-mapped database that fits the RAM budget,
explicit per-platform performance/memory budgets, a bounded threading model, and iOS/Android
packaging. This change specifies the `mobile-runtime` capability. Grounded in the performance notes of
`reference-solutions/docs/04, 06, 08` and the PRD's non-functional requirements.

## What Changes

- Introduce the `mobile-runtime` capability: UniFFI bindings (Rust ↔
  Swift/Kotlin) for direct in-process calls, an optional in-process gRPC server, a memory-mapped
  (linear-probe) database path, explicit latency/memory budgets, a bounded threading/cancellation
  model, dependency minimization for mobile targets, and packaging (xcframework / AAR).

## Capabilities

### New Capabilities

- `mobile-runtime`: on-device embedding of the solver — UniFFI bindings (and optional in-process
  gRPC), memory-mapped DB, performance/memory budgets, threading, and iOS/Android packaging.

### Modified Capabilities

(none.)

## Impact

- A new mobile-runtime layer over the `star-detection`, `plate-solver`, and `pattern-database` capabilities (optionally `grpc-service`),
  plus `uniffi`; builds to an iOS `xcframework` and an Android `.aar`/JNI library.
- Defines the budgets the rest of the system is measured against on-device.
- Constrains dependency and parallelism choices (feature-gated `rayon`, no heavy/native-incompatible deps).
