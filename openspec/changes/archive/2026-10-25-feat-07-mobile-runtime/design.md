## Context

`mobile-runtime` is where "runs on a phone" becomes a contract. The heavy lifting is in
`star-detection`/`plate-solver`/`pattern-database`; this capability is the embedding shell: bindings, the mmap database
path, the budgets, the threading model, and packaging. The reference quotes the numbers we scale
from — cedar-detect <10 ms/Mpx on RPi-4B, cedar ~10 ms/solve on desktop (docs 04/06/08) — and the
PRD fixes the non-functional envelope.

## Goals / Non-Goals

**Goals:**
- A UniFFI surface (Swift/Kotlin) for in-process `solve_from_image` and a reusable DB handle.
- An optional in-process gRPC server (local-only) for gRPC-native consumers.
- mmap (linear-probe) database loading with a bounded RAM ceiling.
- Documented, test-enforced per-platform latency/memory/size budgets.
- Bounded threading + cancellation; dependency minimization; iOS/Android packaging.

**Non-Goals:**
- The detection/solve/DB algorithms (other crates).
- Off-device serving, auth, or multi-tenant use.
- Streaming/video solving (post-v1).

## Decisions

- **UniFFI as the primary mobile path**, in-process gRPC as an alternative. A network hop on
  device is pure overhead for most apps; UniFFI gives a direct Swift/Kotlin call. Consumers
  already invested in gRPC can run the server in-process instead — both wrap the same core.
- **mmap + linear-probe table on device.** Quadratic probing assumes the table is RAM-resident;
  linear probing keeps probe chains contiguous so a memory-mapped, larger-than-RAM database
  works within the budget. Device databases are FOV-matched and built offline (`feat-04`).
- **Budgets are numbers with tests, not adjectives.** Each budget (extract latency, solve
  latency, DB size, peak RAM, startup time) gets a concrete target per reference device and a
  regression test that fails when exceeded — otherwise "bounded" is unfalsifiable.
- **Feature-gate everything optional.** `rayon` and the gRPC server are off by default in the
  mobile build; the core stays pure-Rust to keep binaries small and portable. Parallelism, when
  on, is bounded to a small thread count and never touches the UI thread.
- **8-bit at the boundary.** Camera frames convert to 8-bit grayscale before detection, matching
  the pipeline's input contract.

## Risks / Trade-offs

- [DB size vs RAM on device] → FOV-match the database offline, mmap with linear probing, and bound
  the verification star set; measure peak RAM against the ceiling.
- [UniFFI overhead / type marshalling] → keep the surface small (bytes + dims + params → struct);
  load the DB once; avoid per-call allocation of large buffers.
- [Thermals/power on sustained solving] → bounded threads, timeout, and off-UI execution; document
  a duty-cycle recommendation; continuous solving is post-v1.
- [Cross-compilation toolchains] → standard `cargo` targets for `aarch64-apple-ios` and Android
  NDK ABIs; package via UniFFI's scaffolding (xcframework / AAR).

## Migration Plan

Greenfield embedding crate. Budgets are validated on reference devices in CI device labs (or
documented as targets until hardware is available); parity is inherited from the underlying
crates' fixtures.

## Open Questions

- Exact per-device budget numbers (target phones not yet fixed) — recorded as placeholders to be
  pinned against real hardware; the test harness is specified now so numbers drop in later.
- Whether to bundle a default FOV database in the package or download on first run (size vs
  offline-readiness trade-off) — leaning: bundle a small FOV-matched DB; allow replacement.
