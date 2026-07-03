# Design: feat-08-web-ui

## Context

`ps-web` already exists and is tested (PR #1 on `claude/plate-solver-web-harness-gpmeji`):
an axum server exposing `/healthz` and `/api/solve` plus an embedded Vite + React +
Tailwind SPA. It was built as a dev/test harness outside the OpenSpec loop, so this
change is retroactive: the design decisions below were made during implementation and
are recorded here; the remaining work is an audit of spec vs. implementation rather
than greenfield coding.

## Goals / Non-Goals

**Goals:**
- Document the implemented HTTP contract, resource limits, concurrency behavior, and
  SPA serving as a strict-valid `web-ui` capability spec.
- Verify each spec scenario is covered by an existing test (or add the missing test);
  fix any genuine divergence found by the audit.

**Non-Goals:**
- Changing the API surface, UI design, or frontend stack.
- gRPC/gRPC-Web serving (grpc-service capability), mobile runtime (feat-07),
  authentication/multi-tenancy (harness is local single-user), TLS, HTTP/2.

## Decisions

- **axum over the existing tonic stack for the harness**: browser-first multipart +
  JSON is simpler to consume from a web page than gRPC-Web, and keeps the harness
  decoupled from the ps-grpc service surface.
- **Always HTTP 200 for completed solve attempts**: `no_match`/`timeout`/`cancelled`/
  `too_few` are solver outcomes, not transport errors; HTTP status codes are reserved
  for request problems (400/413/415) and server faults (500). Clients branch on the
  JSON `status` field.
- **Single-permit solve gate, decode inside the gate, `spawn_blocking`**: decode is
  CPU/allocation-heavy, so it must be throttled by the same one-heavy-op-at-a-time
  gate as the solve and must not block tokio workers. Alternative (per-request
  concurrency) rejected: the target host is a dev box or SBC; parallel solves would
  thrash memory.
- **`CancelOnDrop` guard for disconnects**: `spawn_blocking` tasks outlive a dropped
  `JoinHandle`, so the handler holds a drop-guard that trips the solver's cancel flag —
  a disconnect frees the gate promptly instead of after the full timeout.
- **Decode limits (20k px/side, 256 MiB alloc) → 413**: rejects decompression bombs
  before allocation; distinct from 415 (genuinely undecodable bytes).
- **Committed `frontend/dist` embedded via `rust-embed` (no build.rs → npm)**: the
  repo's quality gate is cargo-only; a build.rs shelling to npm would make every cargo
  build depend on node and fail confusingly without it. Committed dist keeps clean
  checkouts self-contained; a lib test asserts the embedded shell's asset references
  all resolve, so a stale/half-committed dist fails the gate. Alternatives rejected:
  `tower-http` ServeDir (loses the self-contained binary), build.rs npm (node in the
  cargo gate).
- **404 for missing dotted paths, SPA fallback only for extension-less paths**: a
  missing hashed bundle must surface as a broken build, not silently serve index.html.
- **Star overlay as SVG in the image's natural-pixel viewBox**: matched-star `x`/`y`
  map 1:1 into the viewBox so markers stay registered at any display size; works
  offline, unlike the CDN-loaded Aladin widget (which degrades to a link).

## Risks / Trade-offs

- [Committed dist churn: hashed bundles change on every frontend edit] → acceptable for
  a harness; the diff doubles as a review signal, and the self-consistency test catches
  forgotten rebuilds.
- [`debug-embed` means dist edits need a `ps-web` recompile to show up in `cargo run`]
  → the vite dev server (proxying `/api`/`/healthz`) is the iteration loop.
- [Solve gate serializes all requests: one slow solve delays others] → intended for the
  target hardware; timeout clamp (60 s) bounds the wait.
- [Aladin depends on an external CDN] → bounded load timeout + fallback link; the
  offline overlay carries the primary "did it solve correctly" signal.

## Migration Plan

None — code is already on the branch; archiving this change lands the spec under
`openspec/specs/web-ui/`.

## Open Questions

- None blocking. Follow-up candidates (out of scope): serving the gRPC and web
  harnesses from one binary; persisting solve history in the UI.
