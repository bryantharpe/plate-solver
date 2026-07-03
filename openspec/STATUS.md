# Documentation Status — Plate Solver (Rust)

_Originally generated 2026-06-09 for the OpenSpec documentation milestone (M0).
Updated 2026-06-30 after implementation: feat-01–06 are implemented, their tasks checked off, and
the changes archived into `openspec/specs/`. Updated 2026-07-03: feat-08 (web-ui) — a retroactive
spec for the already-implemented `ps-web` HTTP API + browser UI harness — was proposed, audited
against the implementation (closing test-coverage gaps and fixing one real bug found in the
process), and archived. feat-07 (mobile runtime) remains an active, un-started change. This is a
review index for the spec set; it is not itself an OpenSpec artifact._

## What this is

A complete, validated OpenSpec documentation set specifying a from-scratch **Rust**
reimplementation of the tetra3/cedar "lost-in-space" plate solver — star detection → pattern-
database lookup → attitude (RA/Dec/Roll/FOV/distortion) recovery — delivered over **gRPC** and
embeddable on **mobile**. The product "why/what" is in [`PRD.md`](./PRD.md); shared context,
conventions, and the glossary are in [`project.md`](./project.md). Implementation status (test
counts, parity outcomes, deferred items) is tracked in
[`IMPLEMENTATION-STATUS.md`](./IMPLEMENTATION-STATUS.md); a defects/positives audit of the
implemented crates is in [`../CODEBASE-REVIEW.md`](../CODEBASE-REVIEW.md).

## Current state (post-implementation)

The implementation milestone landed feat-01 through feat-06: six crates (`ps-core`, `ps-detect`,
`ps-db`, `ps-dbgen`, `ps-solve`, `ps-grpc`), **182 tests pass / 0 fail / 1 ignored**, with the
numerical-parity gates held (sv6 RA/Dec within 10 arcsec, 19/19 matched catalog IDs, cedar-detect
interop). Each completed change's `tasks.md` was checked off and the change archived with
`openspec archive`, moving its `spec.md` into `openspec/specs/<capability>/spec.md` and the change
folder into `openspec/changes/archive/<date>-<change>/`.

`web-ui` (`ps-web`) is a dev/test harness built outside the core solver pipeline — an HTTP API +
browser UI for interactive plate solving, not part of the mobile/gRPC product path. feat-08
retroactively specified it, closed real test-coverage gaps found by the audit, fixed a bug the
audit surfaced (see the carried-forward-gaps section), and archived cleanly with 14/14 tasks.

**feat-07 (mobile-runtime)** is genuinely not implemented (deferred — no Xcode/Android NDK in CI)
and remains the sole active change with 0/12 tasks.

### Archived specs (implemented capabilities)

| Capability | Change (archived) | Crate | Reqs | Tasks |
|---|---|---|---|---|
| `math-core` | feat-01-foundation-math-core | `ps-core` | 19 | 18/18 ✅ |
| `star-detection` | feat-02-star-detection | `ps-detect` | 11 | 17/18 ⚠️ |
| `pattern-database` | feat-03-pattern-database | `ps-db` | 10 | 14/14 ✅ |
| `database-generation` | feat-04-database-generation | `ps-dbgen` | 10 | 15/16 ⚠️ |
| `plate-solver` | feat-05-plate-solver | `ps-solve` | 11 | 19/19 ✅ |
| `grpc-service` | feat-06-grpc-service | `ps-grpc` | 8 | 13/14 ⚠️ |
| `web-ui` | feat-08-web-ui | `ps-web` | 11 | 14/14 ✅ |

### Active changes (not yet implemented)

| Capability | Change | Crate | Reqs/Scenarios | Tasks |
|---|---|---|---|---|
| `mobile-runtime` | feat-07-mobile-runtime | `ps-mobile` | 8 / 12 | 0/12 |

Build order: `math-core` → `star-detection` → `pattern-database` → `database-generation` →
`plate-solver` → `grpc-service` → `mobile-runtime`.

## Carried-forward gaps (incomplete tasks in archived changes)

Three tasks were left unchecked because the work is genuinely incomplete or deferred. They
survive in the archived `tasks.md` as known gaps; resolving them is the work that closes each
capability fully. feat-08/web-ui archived with all tasks checked, but its own audit intentionally
left two scenarios untested (listed separately below) rather than force a flaky test.

- **feat-02 / star-detection — 7.2 `summarize_region_of_interest`**: the auto-exposure/focus
  helper was not ported from cedar-detect (it exists only in `reference-solutions/`). Non-core;
  intentionally skipped.
- **feat-04 / database-generation — 5.3 pattern-count parity vs `default_database.npz`**:
  DEFERRED — the Hipparcos/Tycho source catalogs are not in-repo, so `ps-dbgen` has never been
  checked to reproduce the reference pattern count (1,010,981). Structural validity and
  determinism are tested; count parity is logged via `eprintln!` in `ps-dbgen/tests/e2e.rs` for
  post-catalog-download verification. See [`IMPLEMENTATION-STATUS.md`](./IMPLEMENTATION-STATUS.md).
- **feat-06 / grpc-service — 4.2 gRPC-Web over HTTP/1**: TCP serving with a configurable address
  is done (`ps-grpc/src/main.rs`), and `accept_http1(true)` is set, but the `tonic-web` layer is
  not wired (`tonic-web` is a workspace dep but not a `ps-grpc` dependency; no `GrpcWebLayer` is
  added). gRPC-Web transcoding is therefore not actually active. Untested.
- **feat-08 / web-ui — `timeout` status end-to-end and the two 500 paths**: `ps-web/src/solve.rs`
  maps every `SolveStatus` (including `Timeout`) to its response shape, and unit tests pin that
  mapping, but no test forces the real solver to run past `timeout_ms` and observes the HTTP
  response end-to-end (would need a slow test or solver-latency injection). Likewise, the two 500
  branches (`solver panicked`, `solve gate unavailable`) are defensive code for conditions that
  don't occur in normal operation and have no fault-injection test. Also untested by design:
  `frontend/e2e/smoke.mjs` is a supplementary, manually-run Playwright script (this repo has no CI
  and no committed frontend test tooling), not part of the `cargo test` gate. While auditing
  feat-08's coverage, a real bug was found and fixed: `ps-web/src/solve.rs`'s multipart-error
  handling hardcoded HTTP 400 for every read failure, so an oversize request body incorrectly
  returned 400 instead of the spec's documented 413 (axum's `MultipartError::status()` already
  classified this correctly; the fix routes errors through it instead of a hardcoded status).

These are separate from the code-quality/robustness findings in
[`../CODEBASE-REVIEW.md`](../CODEBASE-REVIEW.md) (C1–C10), which are defects in *implemented* code
against these specs' own acceptance scenarios — most notably C1 (~618 MiB eager-combinations
allocation in the solver, a mobile blocker; fix plan saved at
`../notes/C1-lazy-combinations-fix.md`), C2 (`solve_from_image` hardcodes detection params and
ignores the request's), and C3 (`debug_assert!`-guarded `unsafe` slice in `ps-db` mmap = potential
UB in release). "Implemented and passing parity" does not yet mean "meets every spec scenario" —
the parity gates hold, but the mobile-readiness and boundary-robustness scenarios do not all hold
until those are resolved or explicitly waived.

## Definition of done (documentation milestone, met 2026-06-09; web-ui added 2026-07-03)

- `openspec/` contains `project.md`, `PRD.md`, `STATUS.md`, and **8 changes** (now 1 active + 7
  archived).
- Every change/spec passes `openspec validate <name> --strict` (exit 0). Archived changes show
  4/4 artifacts; the seven archived specs are strict-valid.
- Totals: **88 requirements / 145 scenarios** across the 8 capabilities (77 reqs / 122 scenarios
  from the original M0 set + 11 reqs / 23 scenarios from `web-ui`).

## Scope decisions (from review)

- **cedar throughout** — detection (cedar-detect), DB generation (lattice fields), and the solver
  all follow the cedar variant, which strictly supersedes the original tetra3.
- **Reference-only / non-goals:** tetra3's simpler detector (doc 03) and per-anchor DB
  enumeration (doc 05 §5.1); partial-sky databases; tracking mode; fisheye lens models;
  >8-bit detection.
- **gRPC = full `PlateSolver`** service (`ExtractCentroids`, `SolveFromCentroids`,
  `SolveFromImage`, `GetInfo`), reusing cedar-detect's `Image`/`ImageCoord` message shapes.
- **Parity is a tested contract** — scenarios assert numerical parity vs the Python reference
  within stated tolerances (RA/Dec arcsec, centroids ±0.1 px, identical matched catalog IDs).

## How to review

```sh
openspec list                    # 1 active change (feat-07)
openspec list --specs            # 7 archived specs
openspec show feat-07-mobile-runtime        # the remaining active change
openspec validate math-core --strict        # validate an archived spec
openspec validate web-ui --strict           # validate the web-ui spec
openspec status --change feat-07-mobile-runtime
openspec view                                 # interactive dashboard
```

Read order for a reviewer: [`PRD.md`](./PRD.md) → [`project.md`](./project.md) →
[`IMPLEMENTATION-STATUS.md`](./IMPLEMENTATION-STATUS.md) → the archived specs in dependency order
above (`web-ui` is a standalone dev-harness capability, not part of that build order) →
[`../CODEBASE-REVIEW.md`](../CODEBASE-REVIEW.md) for the defect list.

## Next steps

1. Resolve the four carried-forward gaps above (port `summarize_region_of_interest`; stand up a
   `ps-dbgen` count-parity check once HIP/TYC catalogs are available; wire the `tonic-web` layer
   and add a gRPC-Web interop test; force-test `web-ui`'s `timeout` status and 500 paths, or stand
   up a frontend CI test harness, if that coverage becomes worth the cost).
2. Address the `CODEBASE-REVIEW.md` C1–C10 findings in severity order (C1 fix plan already saved
   at `notes/C1-lazy-combinations-fix.md`).
3. Implement feat-07 (mobile runtime) when iOS/Android tooling is available, then archive it the
   same way.