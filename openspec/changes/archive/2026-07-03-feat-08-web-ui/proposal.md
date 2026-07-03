# Proposal: feat-08-web-ui

## Why

The `ps-web` crate (HTTP web API + browser UI for the plate solver) was built as a test
harness outside the OpenSpec workflow — it shipped on PR #1 with real contract surface
(multipart solve API, error mapping, DoS limits, solve serialization, embedded SPA
serving) but no capability spec behind it. This change backfills the spec so `web-ui`
is governed like the six archived capabilities: requirements and scenarios documented,
validated strictly, and testable against the implementation that already exists.

## What Changes

- Add a `web-ui` capability spec documenting the implemented, tested behavior of `ps-web`:
  - `GET /healthz` — JSON server status + loaded-database properties.
  - `POST /api/solve` — multipart (image + `fov_estimate` + five optional params) to JSON
    plate solution; `status` = `match_found` | `no_match` | `timeout` | `cancelled` |
    `too_few` with human-readable `hint` on non-match; 400/413/415/500 error mapping.
  - Robustness: 32 MiB request body limit, decode dimension/allocation caps
    (decompression-bomb guard), 60 s timeout clamp, single-solve-at-a-time gate with
    decode inside the gate on a blocking thread, cancel-on-disconnect.
  - Embedded SPA serving: committed `frontend/dist` embedded via `rust-embed`,
    correct mime types, 404 for missing assets, SPA fallback to `index.html`.
  - Browser UI behavior: upload → solve → results (stat cards, matched-star overlay on
    the uploaded image, stars table, Aladin sky view with CDN-failure fallback).
- No implementation changes: this is a retroactive spec for code already merged into
  the `claude/plate-solver-web-harness-gpmeji` branch (PR #1) and covered by
  `ps-web/src/lib.rs` unit tests + `ps-web/tests/solve_integration.rs`.

## Capabilities

### New Capabilities

- `web-ui`: HTTP web API (`/healthz`, `/api/solve`) and embedded browser UI for
  interactive plate solving — request/response contracts, error mapping, resource
  limits, concurrency behavior, and SPA serving.

### Modified Capabilities

<!-- none: web-ui consumes plate-solver / star-detection / pattern-database via their
     existing public APIs without changing their requirements -->

## Impact

- **Code**: `ps-web/` (axum server: `src/lib.rs`, `src/solve.rs`, `src/main.rs`;
  React SPA: `frontend/`). Already implemented; no code changes expected from this
  change beyond anything a spec-vs-implementation audit surfaces.
- **APIs**: documents (does not alter) the HTTP contract consumed by the SPA and curl
  users.
- **Dependencies**: `axum`, `tokio`, `rust-embed`, `mime_guess` (server);
  Vite + React + Tailwind toolchain (frontend build only — cargo builds never invoke
  node; `frontend/dist` is committed).
- **Systems**: dev/test harness only; not part of the mobile runtime path (feat-07).
