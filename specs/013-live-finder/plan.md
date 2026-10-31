# Implementation Plan: The Live Finder

**Branch**: `013-live-finder` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add three crates and a live loop:
- `crates/scope-link` holds the `scope_node.v1` protocol, its pairing token and the clock
  maths.
- `crates/scope-node` is the service for the Raspberry Pi 3B. It holds the camera port,
  the Arducam adapter, the console's star detection, auto-exposure and a config file.
- `crates/simulator` is a full stand-in node. It renders the pattern database's stars on
  a virtual clock and runs the real node service over its simulated camera.

`finder` gains:
- a link client;
- the live loop (newest frame wins, timed at mid-exposure, at the calibrated aim point,
  stale after 5 s);
- the session's node, camera settings and aim point;
- `finder --simulate`;
- a server-sent event stream;
- the page's live chart and Setup section.

The scope node captures and detects; the console solves (ADR-0001). `crates/math-core`,
`proto/` and every existing test are unchanged.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- `tonic`, `prost`, `tonic-prost` and `tonic-prost-build` 0.14, as `crates/grpc-service`
  already uses them. Its in-memory transport also needs `tower`, `hyper-util` and
  `tokio-stream`. All of these are already in `Cargo.lock`.
- `toml` (new) for the node's config file.
- `image` with only its `jpeg` feature in `scope-node`, for previews. It is already in the
  lockfile through `finder`.
- `futures-util` moves from `finder`'s dev-dependencies to its dependencies, for the
  event stream. It is already in the lockfile.
- the page: Vite, Vitest and Playwright. No npm package is added.

Each new dependency is a change on a critical path. It must build on 1.96, pass
`cargo-deny` and `cargo-machete`, and be justified in its story.

**Storage**: 012's `session.json`, which gains the node's address and token, the camera
settings and the aim point, and is now written mode 0600. The node reads
`/etc/scope-node/config.toml`.

**Testing**:
- `cargo test -p scope-link`, `-p scope-node`, `-p simulator` and `-p finder`, all with
  `--locked`. Unit tests are in-module, and integration tests are in new files under each
  crate's `tests/`.
- Every live test runs under `#[tokio::test(start_paused = true)]`, with the simulator and
  the console in one runtime over an in-memory transport. `start_paused` needs tokio's
  `test-util` feature, which no manifest enables today, so `simulator` and `finder` add it
  to their `tokio` dev-dependency. A feature changes no entry in `Cargo.lock`.
- Vitest and Playwright in `crates/finder/web`.

**Target Platform**:
- `scope-node` runs on Raspberry Pi OS (64-bit) on a Pi 3B, as a static
  `aarch64-unknown-linux-musl` binary built in WSL.
- `finder` runs on desktop Linux, WSL and the Raspberry Pi.
- The page runs on Android phones and laptops.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the 80% coverage
  floor, which now counts the three new crates.
- No `unsafe`.
- `web/dist` is committed and must be the build of its source.
- The night mode is red only.
- Frames cross 2.4 GHz Wi-Fi: star lists of about 1 KB, and previews at most one per 5 s
  and 640 px.

**Scale/Scope**: one node per console, 1–5 frames a second, up to 200 centroids a frame,
one observer and a handful of devices.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The node detects with `star-detection` and cedar's settings, and US2 proves its centroids equal the console's. The solve is `solve_from_centroids`, unchanged. `crates/math-core` is used, not changed. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/` file, `.github/` file or `CODEOWNERS` entry is edited by a story. The `ci.yml` protoc step and the third Playwright server are owner-approved (spec § Owner approvals), and the operator lands the `ci.yml` step. New tests go in new files. |
| III. Tests first, independent expectations | The oracles are the worked clock exchanges, saved camera fixtures, `DetectParams::cedar()`, the simulator's true directions, `sky::alt_az`, and Meeus's example 13.b inverted for the simulator's own sky maths. Each is computed outside the code under test. |
| IV. Critical paths | The new protocol is internal to `crates/scope-link`, and `proto/` is unchanged. There is no `unsafe` and no database-format change. The dependency changes are listed above, each justified in its story. |
| V. Standalone Rust, honest licensing | Everything is Rust. The Arducam is driven through `rpicam-apps` as a subprocess, so no C library is linked. No data is vendored beyond the operator's camera fixtures, which are the project's own captures. |

## Project Structure

```text
Cargo.lock                               # US1–US7: new crates and dependencies
crates/scope-link/                       # US1 (new)
├── Cargo.toml                           # machete ignore for prost, tonic-prost
├── build.rs                             # tonic-prost-build, as grpc-service's
├── proto/scope_node/v1/scope_node.proto
├── src/lib.rs                           # the generated module (clippy allow), re-exports
├── src/auth.rs                          # TokenInterceptor, BearerToken, tokens_match
├── src/clock.rs                         # Exchange, ClockMap (8 s window)
├── src/memory.rs                        # in-memory transport helpers for tests
└── tests/link.rs
crates/scope-node/                       # US2 (new); US4 adds the service
├── Cargo.toml                           # star-detection, scope-link; toml, image(jpeg)
├── README.md                            # PREREQUISITE: bring-up notes; US2: musl, systemd
├── src/lib.rs
├── src/camera.rs                        # US2: the camera port, ranges, GrayFrame
├── src/synthetic.rs                     # US2: the database-free `simulated` camera
├── src/arducam/{mod,listing,yuv,metadata}.rs  # US2
├── src/detect.rs                        # US2: cedar's settings, top 200
├── src/capture.rs                       # US2: the capture loop and restarts; US4 extends
├── src/config.rs                        # US2
├── src/clock.rs                         # US2: the node's monotonic clock
├── src/service.rs                       # US4: the scope_node.v1 server
├── src/preview.rs                       # US4: JPEG ≤640 px, ≤1 per 5 s
├── src/exposure.rs                      # US4: auto-exposure
├── src/main.rs                          # US2 (--config, --check-config); US4 (serves)
├── tests/data/                          # PREREQUISITE: listing, stream, metadata, frame
└── tests/{arducam,detect,cli}.rs        # US2
crates/simulator/                        # US3 (new); US4 adds the node
├── Cargo.toml                           # scope-node, scope-link, pattern-database, math-core
├── src/lib.rs
├── src/optics.rs                        # US3
├── src/sky.rs                           # US3: alt/az to J2000, parallactic roll
├── src/render.rs                        # US3: PSFs, noise
├── src/noise.rs                         # US3: SplitMix64
├── src/clock.rs                         # US3: VirtualClock
├── src/camera.rs                        # US3: the adapter behind scope-node's port
├── src/node.rs                          # US4: the node service over the simulated camera
├── src/controls.rs                      # US4: push, stop/start frames, unresponsive
└── tests/{sky,node}.rs                  # US3, US4
crates/finder/
├── Cargo.toml                           # US5: scope-link, tonic; simulator (dev); US7: simulator
├── README.md                            # US5, US6, US7
├── src/lib.rs                           # US5, US6, US7: `pub mod` lines only
├── src/link.rs                          # US5 (new): link client, health, reconnection
├── src/session.rs                       # US5: node, camera settings; US6: aim, live pointing
├── src/live.rs                          # US6 (new): the live loop, Solver seam, previews
├── src/solve.rs                         # US6: solve_centroids, a sibling of solve_frame
├── src/sky.rs                           # US6: Frame::unproject
├── src/pushto.rs                        # US6: stale guidance
├── src/simulate.rs                      # US7 (new): --simulate wiring, sim routes
├── src/api.rs                           # US5 (session fields); US6 (aim routes, field
│                                        # report); US7 (sim routes, /api/live)
├── src/main.rs                          # US5 (link start); US7 (--simulate, --sim-start)
├── tests/
│   ├── link.rs, session_node.rs, session_node_cli.rs           # US5 (new)
│   ├── live.rs, aim_api.rs, freshness_api.rs, aim_cli.rs       # US6 (new)
│   └── simulate_cli.rs, sim_api.rs, live_stream.rs, e2e_live.rs  # US7 (new)
└── web/
    ├── src/api.ts                       # US8 (live types, session fields); US9 (aim, put)
    ├── src/useLive.ts (+ .test.ts)      # US8
    ├── src/components/LivePreview.tsx (+ test)    # US8
    ├── src/components/GuidanceBlock.tsx, ChartHeader.tsx  # US8: the stale state
    ├── src/App.tsx, src/App.live.test.tsx         # US8
    ├── src/components/ScopeNodeSection.tsx (+ test)  # US9
    ├── src/components/FieldWarning.tsx (+ test)      # US9
    ├── src/components/AimTap.tsx (+ test)            # US9
    ├── src/components/SetupPage.tsx                  # US9: hosts the new section
    ├── dist/                            # US8, US9: rebuilt
    ├── playwright.config.ts             # US10 (owner approval 2)
    └── e2e/{simulate,live-off}.spec.ts  # US10 (new)
DESIGN.md, .impeccable/design.json       # US9
.github/workflows/ci.yml                 # PREREQUISITE (operator, owner approval 1)
```

## Interfaces

**The protocol** (`crates/scope-link/proto/scope_node/v1/scope_node.proto`, package
`scope_node.v1`):

```text
service ScopeNode {
  rpc Describe(DescribeRequest) returns (CameraInfo);
  rpc Frames(FramesRequest) returns (stream StarFrame);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatReply);   // t0 in; t1, t2 out
  rpc Configure(Settings) returns (Settings);                  // answers the settings in force
  rpc RequestPreview(PreviewRequest) returns (PreviewAck);
}
CameraInfo { model, width, height, repeated Mode modes, active_mode,
             exposure_min_ms, exposure_max_ms, gain_min, gain_max }
Settings   { optional double exposure_ms /* absent: automatic */,
             optional double gain /* absent: automatic */, Detection detection }
Detection  { sigma, binning, normalize_rows, detect_hot_pixels }
StarFrame  { uint64 id, double start_s, double end_s /* node clock */, width, height,
             double exposure_ms, double gain, Settings settings,
             repeated Centroid centroids /* ≤200, brightest first */, optional bytes preview_jpeg }
Centroid   { double x, double y, double brightness }   // full-frame pixels
```

- `StarFrame.settings` reports whether exposure and gain were automatic, and
  `exposure_ms` and `gain` give the values used.
- Every call carries `authorization: Bearer <token>`. No call in this spec is exempt.
  014's stop call will be the only exempt one.
- No comment in the proto contains `[`, because rustdoc runs with `-D warnings`.

**`scope-link`'s Rust surface**:
- `scope_link::proto` is the generated module.
- `TokenInterceptor::new(token)` is the server side, and `BearerToken::new(token)` is the
  client side.
- `tokens_match(a: &[u8], b: &[u8]) -> bool`.
- `Exchange { t0, t1, t2, t3 }` has `delay()` and `offset()`.
- `ClockMap` has `push(exchange)`, `offset() -> Option<f64>` and
  `to_console(node_s) -> Option<f64>`. Its window is the last 8 s of `t3`.
- `memory::{channel_pair, serve_on}` build a tonic client and server over
  `tokio::io::duplex`.

**`scope-node`'s Rust surface**:
- `trait Camera` is the camera port. It offers `info() -> CameraInfo`,
  `apply(&Settings) -> Result<(), RangeError>` and `next_frame() -> Result<GrayFrame,
  CameraError>`.
- `RangeError` names the setting and the range in its message.
- `GrayFrame { pixels, width, height, start_s, end_s, exposure_ms, gain }`.
- `NodeClock` is `now() -> f64` on the node's monotonic clock.
- `detect::DetectSettings::cedar()` and `detect::detect(&GrayFrame, &DetectSettings) ->
  Vec<Centroid>`.
- `Config` is parsed from TOML.
- `service::NodeService::new(camera, clock, token, initial settings)` is built in US4.

**`simulator`'s Rust surface**:
- `Optics { width, height, fov_deg, eyepiece_px }` defaults to 1456 × 1088, 12° and
  (700, 420).
- `Site { lat_deg, lon_deg }`.
- `VirtualClock::new(start_unix_s)` has `now()`.
- `SimCamera` is the adapter behind `Camera`.
- `Simulator` has:
  - `telescope() -> TrueDirections { camera: {alt, az, ra, dec}, eyepiece: {...} }`;
  - `push_to(alt, az)` and `push_by(turn, tilt)`;
  - `point_eyepiece_at(ra, dec)`;
  - `stop_frames()` and `start_frames()`;
  - `set_unresponsive(bool)`.
- `SimNode` is the real `NodeService` over a `SimCamera`. It is served over `memory` or
  over loopback TCP on a random port.

**`GET /api/session`** gains:

```text
node?: { address: string|null, token_set: bool, connected: bool,
         last_frame_age_s: number|null, offset_s: number|null,
         camera: {model, width, height, exposure_ms: {min, max}, gain: {min, max}} | null,
         stars: number|null,
         out_of_range: [{setting: "exposure_ms"|"gain", stored: number, min, max}] }
                        // absent: no node address or token set, and no node connected
camera_settings?: { exposure_ms: "auto"|number, gain: "auto"|number, sigma: number }
                        // absent: none set (automatic, automatic, cedar's 8σ)
aim?: {x, y}            // absent: none set (the frame centre)
field?: { fov_deg: number|null, db_min_fov_deg, db_max_fov_deg, unsolved_s: number }
                        // absent: the live loop has received no frame
pointing: { …012's fields…, source: "upload"|"live", frame_id? }
guidance: { …012's fields…, stale?: true }    // stale: no turn_deg or tilt_deg
```

The token is never in any answer. Every new field is left out until it has a value,
rather than read as `null`: `crates/finder/tests/session_api.rs` compares a fresh read's
whole body with 012's five fields, and the oracle rule keeps that test as it is
(§ Shapes that existing tests pin).

**`PUT /api/session`** also takes these fields:
- `node?: {address?, token?} | null`, where `null` clears both;
- `camera_settings?: {exposure_ms?: "auto"|number, gain?: "auto"|number, sigma?}`.

Both use `deny_unknown_fields`, behind 012's same-origin check.

**Aim routes**, behind the same-origin check:
- `POST /api/aim {frame_id, x, y}`, in full-frame pixels, answers the new session.
- A tap with no centroid within 10 px answers 422 with `{"error": "no star is close
  enough to the tap"}`.
- A frame the console no longer holds answers 409 with an `error`.
- `DELETE /api/aim` resets it.

**The live stream**:
- `GET /api/live` is `text/event-stream`. Each `live` event's data is a live report:

  ```text
  { frame_id, time /* console clock, mid-exposure */, pointing_seq: number|null,
    preview_seq: number|null, stars: number, report: SolveReport, guidance? }
  ```

  `report` is the same `SolveReport` that `/api/solve` answers, with `name: null`, so the
  chart draws it unchanged. `guidance` is the session's guidance as of that solve, so the
  guidance block follows each solve, not the 5 s poll.
- `GET /api/live/preview.jpg?seq=N` answers the JPEG preview of frame `N` while the
  console holds it, or 404.

**Simulator routes** (only under `--simulate`, behind the same-origin check):
- `GET /api/sim/telescope` answers the true directions.
- `POST /api/sim/telescope` takes `{alt_deg, az_deg}`, `{turn_deg, tilt_deg}` or
  `{eyepiece_on_star: true}`. The last puts the eyepiece on the brightest database star
  in the field.
- `POST /api/sim/frames {running: bool}`.

Without `--simulate` every `/api/sim/*` path answers 404.

**CLI**:
- `finder` gains `--simulate` and `--sim-start <YYYY-MM-DDTHH:MM:SSZ>`.
- `scope-node` takes `--config <path>` (default `/etc/scope-node/config.toml`) and
  `--check-config`.

## Shapes that existing tests pin

These are not changed by any story:
- **Rust signatures:**
  - from before 012: `AppState::new`, `AppState::shared`, `router`, `similar()` and
    `solve_frame`;
  - added by 012: `Session`'s public constructors, `Clock`, `SystemClock` and
    `FixedClock`.

  New behaviour arrives through `with_*` builders and new modules.
- **Rust structs:** the fields of `Observer`, `Frame`, `SolveOptions` and `SolveReport`.
  `SolveOptions`' defaults are pinned by a unit test, so the live loop sets `fov` and
  `fov_max_error` per solve rather than changing the defaults.
- **HTTP shapes:** every field 012 put in `GET /api/session`, and 012's
  `PUT /api/session` fields. The new fields are additive and optional on the page. 012's
  test files are unchanged.
- **Geometry:** `sky::alt_az` stays geometric (the Meeus 13.b test). Refraction arrives
  with 014.
- **The page:**
  - every fixture and the fetch mock in `App.test.tsx`, which answers unknown routes with
    the health body, so `/api/session` there has no `node` and opens no stream;
  - the legend's chips;
  - 012's `useSession`, Setup page, guidance block and their tests.
- **End to end:** the default and `session` projects, their servers on 8097 and 8098, and
  `e2e/console.spec.ts` and `e2e/session.spec.ts`.
- **`crates/grpc-service`** and `proto/` are untouched.
- **Whole bodies and files**, read on `main` at 108a5a8 (012 landed):
  - `session_api.rs`'s `fresh_session_and_fixed_clock_are_exact_on_repeated_reads`
    compares a fresh read with exactly `site`, `target`, `pointing`, `min_alt_deg` and
    `time`, so a new read field is left out until it has a value;
  - `session_cli.rs` asserts that a session holding a site, a minimum altitude and a
    target writes a state file of exactly those three keys, so the node, the camera
    settings and the aim point are written only once set. `StoredSession` keeps
    `deny_unknown_fields`, and its new fields default when absent, so 012's files load;
  - `useSession.test.ts` pins the empty session with `toEqual`, which passes over a
    missing or `undefined` key but not a `null` one;
  - `SetupPage.test.tsx` renders `SetupPage` with `session`, `put` and `onBack` only, and
    finds its controls by role and accessible name, which `getBy…` needs to be unique.
- **012's US7**, landed at cf09362: `GuidanceBlock` takes `session` and an optional
  `onSetup`, and derives everything from `session.guidance`. Its tests and
  `App.target.test.tsx` find the guidance block as the one `region` whose name matches
  `/Target/`, so no new labelled section on the console may name a target. `useSession`
  spreads an optional field in only when it is present.
- **012's US8**, landed at 108a5a8:
  - `playwright.config.ts` builds each server with one `server(port, name)` helper:
    `cargo run --release --locked -p finder -- --listen 127.0.0.1:<port> --state <fresh
    folder>`, with `reuseExistingServer: false`. The `default` project (8097) ignores
    `**/session.spec.ts`, and the `session` project (8098) matches only that file.
  - `e2e/session.spec.ts` pins the first screen with a target at 412 × 915 and
    390 × 844: the guidance block, the title block, the chart and its legend lie in the
    viewport in that order, and the roster starts below the legend. On `main` the legend
    ends at 858 px and 836 px, so 390 × 844 has 8 px to spare (§ Plan traps, trap 10).
  - It also carries a site saved on one device to another, opens `/setup` directly, and
    finds no off-red pixel on the console with M78 or on Setup, at 412 × 915. So
    Setup's new section keeps the red-only rule.
  - US8 fitted the first screen by tightening the phone layout with a target: the
    section gap is 4 px, and the chart is inset 20 px on each side.

## Decisions and the alternatives rejected

**Three crates, layered so the Pi never links the solver.** The layering is
`scope-link` ← `scope-node` ← `simulator` ← `finder`, and `scope-node` depends only on
`star-detection` and `scope-link` of the workspace's crates. The simulator renders the
sky with `math-core`'s `PinholeCamera` and `RotationMatrix`, used and not changed. It
cannot use `finder`'s `Frame` or `sky` module, since `finder` depends on it.
- *Rejected:* the node as a module of `finder`. The Pi binary would link the solver and
  the database loader.
- *Rejected:* the simulator as a camera adapter only. Running the real node service over
  the simulated camera is what makes it the same surface.

**The protocol is `scope_node.v1`, generated as `grpc-service` does.**
- The proto is `crates/scope-link/proto/scope_node/v1/scope_node.proto`, built by
  `build.rs` with `tonic-prost-build` (and `--experimental_allow_proto3_optional`, as
  `grpc-service`).
- `Cargo.toml` has a `[package.metadata.cargo-machete]` ignore for `prost` and
  `tonic-prost`, which only generated code uses.
- The generated module carries the clippy allow `grpc-service` uses.
- The proto lives in the crate, not `proto/`, because it is an internal link between two
  programs this repository ships together, not the public solver API.

**Pairing is a bearer token checked by a tonic interceptor in constant time.**
- `tokens_match` folds the XOR of every byte of both tokens, and their length difference,
  into one value before deciding, so no early return leaks a prefix.
- No dependency is added for it.
- The token keeps other devices and mistakes out, not an attacker already on the network.
  It travels in clear, and TLS is out of scope. 014's stop needs no token, so safety never
  depends on it.

**The `finder-page` CI job installs protoc (owner approval 1).** `finder` depends on
`scope-link` from US5, whose build runs protoc, and `finder-page` runs `cargo run -p
finder` for Playwright. The job gets the same "Install protoc" step the clippy,
test-coverage and docs jobs run. The operator lands it before the spec is ready, because
no story may edit `.github/`.
- *Rejected:* committing the generated code with a freshness check. That gives two ways to
  generate gRPC code in one repository, and the check itself needs protoc.
- *Rejected:* `protoc-bin-vendored`. It puts prebuilt binaries in the dependency graph for
  one job's convenience.
- The factory needs no change for it. Its `test` gate already builds `grpc-service` with
  the host's `/usr/bin/protoc` (libprotoc 3.21.12), inside the `/usr` the gate boundary
  binds, and `ergane-gates.yml`'s `test` job installs protoc. Only `ci.yml`'s
  `finder-page` job lacks the step (§ Plan traps, trap 7).

**The node keeps one monotonic clock.**
- Exposure times and heartbeat t1 and t2 come from the same monotonic clock: whichever
  one `SensorTimestamp` uses, `CLOCK_BOOTTIME` or `CLOCK_MONOTONIC`. Bring-up confirms
  which, and NTP steps on the Pi never move it.
- `NodeClock`'s system implementation reads it safely: std's `Instant` for
  `CLOCK_MONOTONIC`, or `rustix`'s `clock_gettime(Boottime)` for `CLOCK_BOOTTIME`.
  `rustix` is already in the lockfile.
- On a Pi that never suspends, the two are equal.

**The console maps clocks with the least-delay exchange of the last 8 s.**
- That is 32 heartbeats at 250 ms. Crystal drift over 8 s is microseconds, against 15″
  per second of error.
- An exchange older than 8 s by `t3` leaves the window even if its delay was the least.
- The heartbeat is unary, every 250 ms.

**Link health.**
- The console counts the node disconnected 1 s after the last heartbeat answered, and
  when the frame stream ends.
- It then reconnects with backoff of 0.5, 1, 2, 4 and then 5 s, never more than 5 s
  between tries.
- A late answer does not revive a link already counted disconnected; the next answer in
  time does.

**One link client for every node.**
- `finder::link` takes a transport: a TCP address, or a connector, for the in-memory
  duplex in tests.
- Under `--simulate` the simulator's node listens on loopback TCP on a random port with a
  random token, and the client connects to it exactly as to a real node.
- The session's own node address is ignored in simulate mode.

**The node streams one star frame per exposure.**
- The id counts exposures, so a frame dropped on the way shows as a gap. Ids keep counting
  across camera restarts.
- The 200 brightest centroids are sent, in full-frame pixels.
- Settings the console sends apply from the next exposure. The exposure in progress
  finishes with the old settings, so they are reported from the second frame after.

**Previews.**
- A frame carries a JPEG of itself, scaled to at most 640 px on its longer side, when 5 s
  have passed since the last preview or the console has asked.
- After `RequestPreview` the next frame carries one.

**The node's detection is the console's.**
- `DetectSettings::cedar()` equals `plate_solver::DetectParams::cedar()` field for field:
  sigma 8, binning 2, rows levelled and hot pixels rejected.
- `detect` calls `star_detection::detect_stars` exactly as `finder::solve::solve_frame`
  does.
- The console's settings change only sigma. The others stay cedar's.
- `plate-solver` is a dev-dependency only, for the parity tests.

**The camera port refuses, never clamps.** An exposure or gain outside the camera's range
is an error naming the range. The node never silently clamps what the console set.

**`simulated` in the node's config is a database-free synthetic camera.** It lives in
`scope-node` and draws seeded random stars with no catalog. That lets the Pi binary be
checked end to end without the Arducam and without linking the pattern database, and it
is the camera behind scope-node's own unit tests. The catalog-drawn sky is the
`simulator` crate's, which runs in-process under `finder --simulate` and in tests.
(Decided in this conversion, because the change named the `simulated` adapter without
saying what it renders.)

**The Arducam adapter runs `rpicam-vid` as a subprocess.**
- **Capture:** `rpicam-vid --codec yuv420 -o - --metadata <fifo> --metadata-format json
  --nopreview -t 0`, with the mode, shutter, gain and frame rate set. Denoise and
  sharpening are off, and contrast and saturation are neutral, so the ISP's tone curve
  does not distort star brightness.
- **Frames:** each is read from stdout at the configured stride (rows padded past the
  width), and its Y plane is kept without the padding. `SensorTimestamp` gives the
  exposure start, and `ExposureTime` its length.
- **Settings changes** restart the subprocess, since `rpicam-vid` takes no runtime
  control. At 1–5 frames a second, auto-exposure's few steps cost a few seconds at most.
- **Camera info** comes from parsing `rpicam-hello --list-cameras`.
- **Tests:**
  - parsing and framing run on the saved listing and the saved stream with its metadata;
  - the subprocess handling runs on CI with a fake `rpicam-vid` script first on `PATH`,
    which replays the fixture and exits;
  - only spawning the real binary stays untested.
- *Rejected:* libcamera Rust bindings. They need libcamera headers and libclang, which the
  `--all-features` CI lacks.

**Auto-exposure is a proportional step in log exposure.**

```text
exposure ← exposure · (30 / max(N, 1))^0.7
```

- Each step is clamped to ×4 either way.
- Gain rises once exposure reaches its cap or the frame-rate limit, and falls first when
  there are too many stars.
- Nothing changes while N is within 25–40.
- On the simulator it settles in 3–5 frames from 5 stars.
- A setting the console fixed is never touched. With exposure fixed and gain automatic,
  only gain moves.

**Config and the Pi build.**
- `/etc/scope-node/config.toml` (`--config` moves it) holds:
  - the listen address;
  - the token;
  - the adapter, `arducam` or `simulated`;
  - the starting exposure and gain, each a number or `"auto"`.
- It is parsed with serde's `deny_unknown_fields`. Errors name the key, such as
  `exposre_ms`, or the field with the bad value, such as `gain`.
- `--check-config` prints every setting with the token as `****`, and exits 0.
- The binary is static `aarch64-unknown-linux-musl`, built from WSL, because Pi OS's glibc
  is older than Ubuntu 26.04's. The target and flags go on the command line or in the
  user's own `~/.cargo/config.toml`. The repository gets no `.cargo/config.toml`, which
  `scripts/ci/e2e-eval.sh` refuses.
- `crates/scope-node/README.md` documents the systemd unit (`Restart=always`, a dedicated
  user in the `video` group) and the config file's mode 0600.

**The simulator.**
- **Optics:**
  - a 1456 × 1088 sensor, the IMX296's size and the sensor cedar-detect works around;
  - a 12° horizontal field, inside the bundled 10–30° database;
  - the eyepiece at pixel (700, 420);
  - the camera's roll fixed to the tube.

  The field and the eyepiece pixel are settable.
- **What the telescope's direction means:** the simulated telescope's altitude and azimuth
  are the finder camera's centre. The eyepiece looks through pixel (700, 420) of it.
  `point_eyepiece_at` solves for the telescope direction that puts a sky direction on
  that pixel. The default start is altitude 45°, azimuth 180°.
- **Rendering:**
  - stars come from the database's `star_table` (brightest first), down to a limit that
    follows exposure and gain (`m_lim = m₀ + 2.5·log10(exposure·gain / ref)`). The
    constants are chosen so the default settings give 25–40 stars in an average field;
  - each star is a Gaussian PSF (σ 1.2 px) with flux from its magnitude, the exposure and
    the gain;
  - the noise is sky background, then Poisson and read noise, from an in-crate SplitMix64
    seeded from a base seed and the frame id. Runs repeat exactly, and no dependency is
    added.
- **Sky to camera:**
  - the telescope's alt/az becomes RA/Dec of date for the site and virtual time, which is
    the inverse of `sky::alt_az`'s formulas, re-implemented in `simulator::sky`;
  - that is precessed back to J2000;
  - the camera's roll comes from the parallactic angle.

  US3 tests the inverse against Meeus's example 13.b. US7's push test cross-checks it
  against `finder::sky::alt_az`.
- **Time:** `VirtualClock` is the `--sim-start` instant plus `tokio::time::Instant`
  elapsed.
  - Under `#[tokio::test(start_paused = true)]` it advances only when the test's tasks
    idle, so long runs take milliseconds and repeat exactly.
  - Under `finder --simulate` it runs at the real rate.
  - It is built in only those two places.

  The simulated node's own clock is virtual time minus the simulator's start, plus
  1000 s, like a monotonic clock since boot. The console therefore has a real offset to
  recover.
- **Transport:** in tests, `tokio::io::duplex`. Under `--simulate`, loopback TCP on a
  random port. Both exercise every call a real node gets.
- **State:** `--simulate` defaults `--state` to `<default state folder>/simulate`, so a
  desk session never overwrites the night's site, target, node address or aim point.
  `--sim-start` without `--simulate` is refused, so a virtual clock never reaches a real
  node.
- **Pushed by hand:** `POST /api/sim/telescope` is routed only under `--simulate`, behind
  the same-origin check. Its siblings are listed under Interfaces.
- **Field size for the 200-centroid test:** the bundled database holds 42,212 stars,
  about one per square degree. US4's test sets a 20° field on a Milky Way direction and
  first asserts that more than 200 database stars are in the frame.

**The live loop: the newest frame wins.**
- The link client writes each frame into a `tokio::sync::watch` channel, which holds only
  the newest. The live task waits on it, takes the newest frame, and solves. Frames that
  arrive meanwhile overwrite each other, which drops stale frames with no queue.
- Each solve:
  - goes through a `Solver` seam, an async trait whose real implementation runs
    `solve_centroids` on `spawn_blocking`. Tests wrap it to sleep (in virtual time) or to
    fail;
  - uses `solve_from_centroids`, through `finder::solve::solve_centroids`, a sibling of
    `solve_frame` that starts from centroids. It takes the brightest
    `SolveOptions::default().stars` and keeps every other default;
  - uses the last solved field as the estimate, with a 5% allowance
    (`fov_max_error = 0.05 · estimate`);
  - after three failures in a row, uses no estimate. A frame with fewer than 4 stars
    counts as a failure;
  - takes the pointing at the aim point, through `Frame::unproject`: undistort, unproject
    through the solution's `PinholeCamera`, then inverse-rotate. With no aim point it uses
    the frame centre, so the result equals `Frame.center`;
  - times the pointing at mid-exposure, `(start + end) / 2`, mapped to the console's
    clock;
  - records the pointing in the session with 012's `seq`, `source: "live"` and the frame
    id;
  - broadcasts the live report on a `tokio::sync::broadcast` (capacity 16; a lagging
    subscriber skips).
- The console keeps the last 8 frames that carried a preview: their JPEG and centroids,
  by frame id. Taps and `/api/live/preview.jpg` read from them.

**Fresh pointings only.**
- A live pointing is stale once `clock.now() − pointing.time > 5 s`. The guidance then
  carries `stale: true` and no turn or tilt.
- An uploaded frame's pointing never goes stale, as 012 says.
- The rule is evaluated on every session read and every live report.

**The page opens the event stream only while connected.**
- `GET /api/session`, polled every 5 s, carries `node.connected`.
- The page opens `EventSource('/api/live')` only while it is true, and closes it when it
  turns false. So a console with no node never holds a long-lived request, and the
  offline end-to-end test still reaches `networkidle`.
- The page decides staleness itself, because it would otherwise wait for the next poll:
  - each poll and each event give it the console's time against `Date.now()`;
  - a 250 ms timer marks the pointing stale once the console's time is more than 5 s
    past the pointing's time;
  - the next live event clears it.
- Previews are fetched as `/api/live/preview.jpg?seq=N` and drawn under the chart, inside
  the existing `.photo` red-only filter.

**Aim calibration snaps on the server.**
- The page sends the tapped pixel, scaled from the shown preview to full-frame pixels,
  with the frame's id. The server snaps to that frame's nearest centroid within 10 px, and
  stores the aim point in the session. So the oracle, the simulator's (700, 420), is
  checked in a Rust test, not only through the page.
- The aim point is kept until reset. Moving the camera is the observer's cue to reset it.

**The session's new fields.**
- **The node's address** is a host and a port, as `IP:port` or `name:port` with a port of
  1–65535 and no whitespace.
- **The token** is a non-empty string, written only. A read gives `token_set`.
- **The state file** is written mode 0600: the temporary file is created 0600, then
  renamed over the old one. An existing file's mode is tightened on the next write. The
  node, the camera settings and the aim point are written only once set, and default
  when absent, so a state file 012 wrote loads unchanged.
- **The exposure and gain** are each `"auto"` or a positive number. With a camera
  connected, a number outside its range is refused with 400 naming the range. With none
  connected, any positive number is kept.
- **Sigma** is a finite number from 1 to 50.
- **On connect, and whenever the settings change,** the console sends `Configure`. A
  stored value outside the connected camera's range is sent as automatic, kept as stored,
  and listed in `node.out_of_range`.

**`field` in the session.**
- `fov_deg` is the last solved field.
- `db_min_fov_deg` and `db_max_fov_deg` are the loaded database's range, as
  `/api/health` reports it.
- `unsolved_s` is how long frames of 10 or more stars have gone without a solve. It is
  reset by any solve.
- The page warns when the field is within 1° of either end of the range or outside it, or
  when `unsolved_s ≥ 30`.

**`--sim-start` is parsed in-crate.** It takes `YYYY-MM-DDTHH:MM:SSZ`, with optional
fractional seconds, and converts it with a days-from-civil function. Adding `time` as a
direct dependency for one parse is not worth it.

**The event stream is axum's `Sse`** over a `futures_util::stream::unfold` of a broadcast
receiver, with a keep-alive comment every 15 s. Axum's `sse` module needs no new feature.

**Non-goals:** hinted solving, the mount (014), TLS, broadcast discovery, and more than
one node per console.

## User Story 1 — The scope link

**The crate.**
- `crates/scope-link` has `build.rs`, the proto, a `[package.metadata.cargo-machete]`
  ignore, and `#[allow(clippy::double_must_use)]` on `pub mod proto`, as `grpc-service`
  does.
- It is added to the workspace by the `crates/*` glob, so the root `Cargo.toml` is
  unchanged. `Cargo.lock` gains it.

**`auth.rs`.**
- `TokenInterceptor` reads `authorization` and requires the `Bearer ` prefix, then checks
  the token with `tokens_match`. It answers `Status::unauthenticated` otherwise.
- `BearerToken` adds the header on the client side.

**`clock.rs`.**
- `Exchange::delay() = (t3 − t0) − (t2 − t1)`, and
  `Exchange::offset() = ((t1 − t0) + (t2 − t3)) / 2`.
- `ClockMap::push` drops every exchange whose `t3` is more than 8 s before the newest
  `t3`.
- `offset()` is the least-delay exchange's offset. `to_console(t) = t − offset`.

**`memory.rs`** builds a duplex server and client pair with `hyper_util::rt::TokioIo`:
`Endpoint::connect_with_connector` on the client side, and `serve_with_incoming` on the
server side. It is public, so `scope-node`, `simulator` and `finder` tests share it.

**Tests.** `tests/link.rs` serves a test `ScopeNode` that streams numbered frames, behind
the interceptor.

## User Story 2 — The scope node's port, adapter, detection and config

**The crate.**
- `crates/scope-node` is a library plus the `scope-node` binary.
- Its dependencies are `star-detection`, `scope-link`, `tokio`, `serde`, `toml` (new),
  `clap` and `rustix`, the last only if bring-up names `CLOCK_BOOTTIME`.
- `plate-solver` is a dev-dependency.

**`camera.rs`.**
- It holds the `Camera` trait, `CameraInfo`, `GrayFrame`, `Settings`, `RangeError` and
  `CameraError`.
- A `check_range` helper is shared by every adapter, so refusals read alike, for example
  "exposure 1500 ms is outside the camera's range 0.1–1000 ms".

**`synthetic.rs`** is the `simulated` camera, which draws seeded random stars.

**`arducam/`.**
- `listing.rs` parses `rpicam-hello --list-cameras`.
- `yuv.rs` splits a byte stream into frames at `stride × height × 3/2`, keeping
  `width` bytes of each Y row.
- `metadata.rs` reads the JSON lines for `SensorTimestamp` (ns) and `ExposureTime` (µs).
- `mod.rs` spawns `rpicam-vid` with the arguments above, from `PATH`, pairs frames with
  metadata, and reports exit with the captured stderr.

**`capture.rs`.** The capture loop takes a frame, detects it, and hands on the frame and
its centroids. On a `CameraError` it logs the error and the stderr, waits 1 s, and
restarts the camera, without the process restarting.

**`config.rs` and `main.rs`.** `main` parses `--config` and `--check-config`, loads and
validates the config, and then runs the capture loop. Serving is added in US4.

**Fixtures.** The fixtures are the operator's (Prerequisites). Their sizes and the mode
they were taken in are recorded in `crates/scope-node/tests/data/README.md`, by the
operator.

## User Story 3 — The simulator

**The crate.** `crates/simulator` depends on `scope-node`, `scope-link`,
`pattern-database`, `math-core` and `tokio`. `plate-solver` is a dev-dependency, for the
solve tests.

**The modules.** `optics.rs`, `sky.rs`, `noise.rs`, `render.rs`, `clock.rs` and
`camera.rs`, as decided above. `SimCamera` implements `scope_node::Camera`. Its exposure
and gain ranges are settable, defaulting to 1–2000 ms and 1–16.

**The tests.**
- They load the bundled database the way `crates/finder/tests/api.rs` loads it.
- They detect with `scope_node::detect`, and solve with `solve_from_centroids` and the
  defaults `finder::SolveOptions` uses. They cannot use `finder` itself.
- They check the inverse sky maths against Meeus's example 13.b.

## User Story 4 — The node's service, previews, settings and auto-exposure

**`scope-node`.**
- `service.rs` implements `ScopeNode` over a capture loop. `Frames` subscribes to a
  broadcast of star frames. `Heartbeat` answers t1 and t2 from `NodeClock`.
- `preview.rs` scales by box filter and encodes with `image`'s JPEG encoder (quality 80).
- `exposure.rs` holds the controller.
- `main.rs` now serves on the configured address behind `TokenInterceptor`.

**`simulator`.**
- `node.rs` runs `NodeService` over a `SimCamera`. Its frame interval is settable, at
  2 frames a second in tests.
- `controls.rs` provides push, stop and start frames, and unresponsive.
- The in-memory and loopback TCP transports are both offered.

## User Story 5 — The link client and the session's node and camera settings

**`finder::link`.**
- `LinkManager` watches the session's node address and token, and its camera settings.
  It connects with `BearerToken`, calls `Describe` and `Configure`, opens `Frames`, and
  runs the heartbeat.
- Frames go into the `watch` channel, and `ClockMap` is updated.
- It reports `LinkStatus { connected, last_frame_at, offset_s, camera, stars,
  out_of_range }`.
- It is started by `AppState::with_link(...)`, which the binary calls. Library users
  without it get `node.connected: false`.

**`finder::session`** gains the node, the token, the camera settings and their validation
against `LinkStatus.camera`, and 0600 writes.

**Dependencies.** `finder` gains `scope-link`, `tonic`, `tower` and `hyper-util`, the last
two for the connector, plus `simulator` as a dev-dependency.

## User Story 6 — The live loop, the aim point and freshness

**`finder::live`.**
- It holds the live task, the `Solver` trait, `DbSolver`, the FOV estimate state, the
  preview store and the broadcast.
- It is started by `AppState::with_live(...)` alongside the link.

**Other modules.**
- `finder::solve` gains `solve_centroids(db, centroids, size, options) -> (SolveReport,
  Option<Frame>)`. `solve_frame` keeps its signature and behaviour.
- `finder::sky` gains `Frame::unproject((x, y)) -> Option<UnitVector>`.
- `finder::pushto` gains the stale rule.
- `finder::session` gains the aim point, `source` and `frame_id` on the pointing, and the
  field report.

**`api.rs`** gains `POST`/`DELETE /api/aim` and the `field` and `aim` fields.

## User Story 7 — `finder --simulate` and the live stream

**`finder::simulate`.**
- It builds a `Simulator` with the default optics, at the session's site, or at
  40° N, 75° W when none is set.
- It builds a `VirtualClock` from `--sim-start`, wrapped as `finder`'s `Clock`.
- It serves the `SimNode` on `127.0.0.1:0` with a random token, and points the link at it.
- It adds the `/api/sim/*` routes to the router, which happens only when `AppState` holds
  a simulator.

**`main.rs`.**
- It refuses `--sim-start` without `--simulate` before binding anything.
- It moves the default `--state` to the `simulate` folder.

**Dependencies and the stream.** `simulator` moves to `finder`'s dependencies.
`/api/live` and `/api/live/preview.jpg` are added.

**`tests/e2e_live.rs`.** It runs the whole loop in one runtime in paused time:
- site and target through the router;
- `eyepiece_on_star`;
- a tap 6 px off through `POST /api/aim`;
- the guidance checked against `sky::alt_az` of M78 and the simulator's true eyepiece
  alt/az.

## User Story 8 — The live chart and the stale state

- `useLive(session)` opens and closes the `EventSource`. It holds the latest live report
  and the console's time offset, and runs the 250 ms stale timer.
- `App` draws the live report's `report` in the chart when one is newer than the last
  upload's.
- `LivePreview` draws `/api/live/preview.jpg?seq=N` under the frame, inside `.photo`.
- `GuidanceBlock` and `ChartHeader` gain an optional stale state, which says "pointing
  stale" and shows no turn or tilt. Their props stay backwards compatible, so their
  existing tests are unchanged.
- **Last step:** rebuild `web/dist`.

## User Story 9 — Setup's scope node and camera section

- `ScopeNodeSection` sits inside `SetupPage`:
  - the address;
  - the token, an empty password field that says "a token is set" when `token_set`;
  - the exposure and gain, each auto or a value;
  - sigma;
  - the link's state, the camera's model and the star count, updated from live events.
- `FieldWarning` compares `field.fov_deg` against the database's range and
  `field.unsolved_s`.
- `AimTap` shows the latest preview. A tap posts the scaled pixel with the frame id. It
  shows the 422 message, and has a reset control.
- `DESIGN.md` and `.impeccable/design.json` gain the live chart, the preview, the stale
  state, the section and the tap, as built.
- **Last step:** rebuild `web/dist`.

## User Story 10 — End to end

**`playwright.config.ts`.**
- A third server is added: `finder --listen 127.0.0.1:8099 --simulate --sim-start
  2026-01-16T02:00:00Z`. It gets a fresh `--state` folder under `target/`, made the way
  012 makes the session server's, and `reuseExistingServer: false`.
- It serves a `simulate` project, which matches only `e2e/simulate.spec.ts`. The default
  and `session` projects ignore that file.
- The `test` gate already runs `CI=1 npm run e2e`, so the third server is gate evidence
  with no change to the gate (§ Plan traps, traps 6 and 7).

**`e2e/simulate.spec.ts`.**
- It runs serially and sets the site and M78 first.
- It reads true directions from `GET /api/sim/telescope`. It compares the chart against
  the camera direction while the session has no aim point, and against the eyepiece
  direction once it does.
- The aim tap runs last.
- The night pixel check reuses 012's helper.

**`e2e/live-off.spec.ts`** runs in the default project, against the server with no node.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`, as of 3fc6a24):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`, `npm ci`,
    `npm run typecheck`, `npm run format:check`, `npm test`, `npm run build`,
    `git status --porcelain -- dist` (which must list nothing) and `CI=1 npm run e2e`.

  Every story runs both, Rust and page stories alike, and the judge sees their results.
- **US5 onwards** must keep CI's `finder-page` green: from US5, `cargo run -p finder`
  builds `scope-link`, which relies on the operator's protoc step.
- **CI in the merge queue** runs every `ci.yml` job. Some have no factory counterpart:
  coverage at 80%, the cedar e2e eval and canary, `cargo machete`, `cargo deny`, the
  secret scan, and the build on Rust 1.96 (`msrv`). The sandbox has neither cargo tool
  and builds on stable, so a story meets those checks by construction (§ Plan traps,
  trap 5). A story is not done until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **012 has landed**, and § Shapes that existing tests pin has been re-read against
   it. Met: 012 completed on 2026-10-31 with US8 at 108a5a8, and the shapes above were
   re-read against US7 and US8 as they landed.
2. **The bring-up fixtures and notes are on `main`**, staged by the operator from the Pi
   3B and its Arducam.
   - In `crates/scope-node/tests/data/`:
     - `list-cameras.txt`, a saved `rpicam-hello --list-cameras`;
     - `stream.yuv` and `stream.json`, a short `rpicam-vid --codec yuv420` stream with
       padded rows and its metadata;
     - `finder-frame.png`, a saved gray frame;
     - a `README.md` giving each file's mode, stride, frame count and size.

     The stream and the frame are each well over 200 KiB. Take the smallest mode whose
     rows are padded, keep two or three frames, and compress the stream if that keeps the
     test simple. That is why no story can carry them.
   - In `crates/scope-node/README.md`, the bring-up notes:
     - the Arducam's model and modes;
     - the row stride;
     - which monotonic clock `SensorTimestamp` uses;
     - the finder camera's measured field of view;
     - detection time per frame on the Pi 3B;
     - Wi-Fi throughput.
3. **The `finder-page` job installs protoc** (owner approval 1). The operator lands the
   same "Install protoc" step the Rust jobs use in `.github/workflows/ci.yml`. The
   factory's `test` gate and `ergane-gates.yml` need no change.
4. **The owner approvals are given.** They are listed in spec § Owner approvals, and
   flipping to `ready` records them.
5. **The factory's coding agent can run cargo.** Met since ergane#896 (deployed
   2026-10-28): 012's stories built and tested with it. Every story from US1 to US7
   changes `Cargo.lock`, and US2 adds `toml`, which a hand edit cannot lock.

## Plan traps

Lessons from building 012 on this factory, each with its mechanism and the check that
keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only the gate results and the story's
   diff. A Then clause that names a check only CI runs, or a test no gate runs, fails
   correct work: 012's page scenarios did, until #118 put the `finder-page` checks in the
   `test` gate and #119 restated the scenarios. *Check:* every Then clause names a
   committed test, or a command a gate runs; US8-S6 and US9-S10 name the `test` gate's
   build-freshness commands.
2. **A named command is read literally.** The judge refused 012's US3 docs scenario until
   the `lint` gate ran the exact `cargo doc -p finder --no-deps` it named; a broader
   command did not count (#113, #114). *Check:* a scenario that names a command copies it
   from `ergane.yaml` verbatim.
3. **One evidence budget.** The story's diff and both gates' output share the judge's
   128 KiB, each gate's output tail can be 32 KiB, and evidence past the budget is
   refused before the judge reads it. 012's stories were 22–44 KB of diff each. *Check:*
   a story keeps its diff under 64 KiB, `Cargo.lock` included (`dist/` is `-diff` and
   shows only as changed files), measured before proposing with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. US4, US6 and US7
   are the largest; one that cannot fit escalates rather than dropping a test.
4. **An additive field still breaks a whole-body test.** 012's US5 added `guidance` to
   session reads, and the existing `partial_edits_and_null_keep_every_other_field`
   compared whole bodies, so the story stopped on a question until the owner approved a
   one-line change (question 8a357f48cda5). *Check:* 013's new session fields are left out
   until they have a value (§ Interfaces), the tests in § Shapes that existing tests pin
   stay as they are, and no story edits an existing test.
5. **The sandbox has only the gates' tools.** A step that needs a tool the sandbox lacks
   stops the story on a question, and in a chain a question stops the epic: 012's
   builders asked, story after story, about the `ergane` CLI their sandbox lacks. The
   factory's toolchain is stable Rust (1.99.1) with rustfmt and clippy, Node 24 and the
   seeded Chromium. It has no `cargo-machete`, no `cargo-deny` and no 1.96 toolchain.
   *Check:* no task runs a tool outside the gates. A new dependency is used, or sits in a
   `cargo-machete` ignore with its reason, and builds on Rust 1.96, because only the merge
   queue checks either.
6. **The gate's ports are the host's.** The gate boundary unshares only the PID namespace,
   so the e2e servers bind the host's 127.0.0.1:8097, 8098 and 8099. A console left
   running on one of them, or a second e2e run at the same time, fails the gate before a
   test runs (`reuseExistingServer: false`). *Check:* the operator keeps 8097–8099 free on
   the factory host while a story builds; 013 is a chain, so one story's gates run at a
   time; a test binds `127.0.0.1:0`, and only `playwright.config.ts` names a fixed port.
7. **The gate can run it.** A probe on 2026-10-31 ran one gate through Ergane's own
   `run_gates`, inside the real gate boundary with this repository's caches, on `main` at
   5f61b89, and passed. In it, `/usr/bin/protoc` compiled a proto3 `optional`,
   server-streaming service. Three Playwright web servers on 8097–8099 started in one
   `CI=1` run, and Chromium 1194 loaded each. A `#!/bin/sh` script in a temporary folder
   ran first on `PATH`, as US2-S4's fake `rpicam-vid` will. (Operator's record:
   `~/ergane-ops/plate-solver/probes/2026-10-31-gate-boundary/`.) *Check:* nothing beyond
   trap 6. A story that changes the Playwright version fails e2e until the operator seeds
   its browser, because `playwright install` refuses this host's Ubuntu 26.04
   (`ergane.yaml` § caches).
8. **Each worktree builds on its own.** A shared `CARGO_TARGET_DIR` let one story's gate
   run another story's test binary, so every gate now builds in the worktree's own
   `target/` (#118). A cold story compiles the workspace twice, once for the tests and
   once for the release binary Playwright serves. *Check:* no story sets
   `CARGO_TARGET_DIR`, and a test's temporary state goes under `target/` (ignored) or the
   system's temporary folder, as 012's tests put theirs. A gate that leaves an unignored
   path behind is reported as having dirtied the worktree.
9. **Fixes land on `main` first.** When the operator fixes a gate or this spec mid-epic,
   the fix merges first and the story merges `origin/main`. 012's US3 cherry-picked two
   gate fixes, collided with their squash on `main` and spent a recovery cycle. *Check:* a
   story never cherry-picks an operator's commit.
10. **The phone's first screen has 8 px to spare.** 012's US8 measured FR-033's first
    screen in Chromium for the first time. US7's layout overflowed it by 7 px at
    412 × 915 and 56 px at 390 × 844, and the story stopped on a question until the owner
    let it tighten the layout. With a target on `main`, the legend now ends at 836 px of
    844. So anything 013 adds above the legend on a phone with a target fails the `test`
    gate, whether it is the live preview, a stale line or a taller block. *Check:* the
    preview goes below the legend (T066), the stale state makes neither block taller
    (T067), and US8 and US9 run `CI=1 npm run e2e` before proposing, with
    `e2e/session.spec.ts` unchanged and green.

## Risks

| Risk | Mitigation |
|---|---|
| The Arducam needs Arducam's own drivers, or doesn't support the Pi 3B. | Bring-up runs before the spec is ready. The camera port makes another camera an adapter, not a respec. |
| The finder camera's field lies outside the bundled 10–30° database. | Bring-up measures it, and the operator builds a matching database with `database-generation`, outside the factory. Setup warns when frames don't solve (FR-037). |
| The ISP's tone curve compresses star brightness and shifts the threshold. | Neutral ISP settings. Bring-up compares against a raw capture, and sigma can be adjusted from Setup. |
| 2.4 GHz-only Wi-Fi is slow. | Star lists are about 1 KB. Previews are capped at one per 5 s and 640 px. Bring-up measures throughput. |
| NTP steps the Pi's clock. | The node uses a monotonic clock only. |
| Detection on a Pi 3B is slower than the estimated 40 ms. | Bring-up measures it. Live push-to needs only 1–5 frames a second. |
| The token travels in clear on the house Wi-Fi. | It keeps out other devices and mistakes. 014's stop needs no token, so safety never depends on it. |
| Paused-time tests could advance virtual time while a `spawn_blocking` solve runs, dropping frames that should be solved. | Tokio does not auto-advance paused time while a blocking task is in flight. US6's first test asserts every frame is solved, which would catch a regression. |
| The `ci.yml` step is not landed, and `finder-page` cannot build `finder` from US5 on. | It is a prerequisite (item 3), not a story task. |
| A story passes the gates but fails a check only the queue runs (coverage, the cedar eval, `cargo machete`, `cargo deny`, the secret scan, `msrv`). | The `test` gate runs `finder-page`'s checks, so those no longer differ. The rest are met by construction (§ Plan traps, trap 5), and a queue failure is reported back to the node's ladder. |

## Refinement Record

Reviewed on 2026-10-31 by the plate-solver operator against `main` at 108a5a8 and the
gate boundary probed in § Plan traps. Writes come from the task text; shared writes name
the merge edge that orders them, and the 012 writers that have not landed yet.

```yaml
version: 1
reviewed_revision: 108a5a8d66675c53c124679087ecb48774b6b0eb
stories:
  US1:
    tests:
      - path: crates/scope-link/src/clock.rs
        status: new
        disposition: add
        assertion: "exchanges A and B give delays of 0.040 s and 0.120 s and A's offset of +0.050 s; a least-delay exchange 9 s old leaves the window, and node time 200.100 s maps to 200.050 s; each within 1 µs"
        decision: "spec.md:US1-S3"
      - path: crates/scope-link/src/auth.rs
        status: new
        disposition: add
        assertion: "tokens_match accepts only the equal token, and refuses one that differs in its last byte and one of another length"
        decision: "spec.md:US1-S7"
      - path: crates/scope-link/tests/link.rs
        status: new
        disposition: add
        assertion: "no token and wrong-token are answered UNAUTHENTICATED with no frame sent; test-token opens the stream and frames arrive"
        decision: "spec.md:US1-S1"
      - path: crates/grpc-service/tests/service_acceptance.rs
        status: existing
        disposition: preserve
        assertion: "the public plate_solver gRPC service still answers as before, because proto/ and crates/grpc-service are not touched"
        decision: "spec.md:US1-S6"
    preserved_behavior:
      - "proto/ and crates/grpc-service are unchanged (spec.md US1-S6, FR-002)"
      - "Cargo.lock keeps every existing entry at its version and gains the scope-link package"
    checks:
      development:
        - "cargo test -p scope-link --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue's ci.yml jobs, among them unused-deps for the cargo-machete ignore and msrv for Rust 1.96"
    writes:
      - Cargo.lock
      - crates/scope-link/Cargo.toml
      - crates/scope-link/build.rs
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-link/src/lib.rs
      - crates/scope-link/src/auth.rs
      - crates/scope-link/src/clock.rs
      - crates/scope-link/src/memory.rs
      - crates/scope-link/tests/link.rs
    shared_writes:
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, so US2 locks toml on top of US1's lockfile"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/src/camera.rs
        status: new
        disposition: add
        assertion: "the simulated camera refuses an exposure past its range with an error naming the range, and takes no frame"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/arducam.rs
        status: new
        disposition: add
        assertion: "the saved listing parses to the model, every mode and the active mode; the padded stream gives each Y plane byte for byte with the saved times; a fake rpicam-vid that exits is reported with its stderr and restarted in the same process"
        decision: "spec.md:US2-S3"
      - path: crates/scope-node/tests/detect.rs
        status: new
        disposition: add
        assertion: "the saved finder frame gives identical centroid lists through the node and through detect_stars with DetectParams::cedar(), and DetectSettings::cedar() equals DetectParams::cedar() field for field"
        decision: "spec.md:US2-S5"
      - path: crates/scope-node/tests/cli.rs
        status: new
        disposition: add
        assertion: "--check-config prints every setting with the token masked and never test-token, and exits 0; exposre_ms and a gain of loud each exit nonzero naming the key"
        decision: "spec.md:US2-S7"
      - path: crates/star-detection/tests/parity.rs
        status: existing
        disposition: preserve
        assertion: "star-detection still agrees with cedar-detect; the node calls it and never forks it"
        decision: "spec.md:FR-006"
    preserved_behavior:
      - "the operator's fixtures in crates/scope-node/tests/data/ and bring-up notes in crates/scope-node/README.md are unchanged (tasks.md T020, T021)"
      - "the repository has no .cargo/config.toml, which scripts/ci/e2e-eval.sh refuses (spec.md FR-008)"
      - "scope-node's [dependencies] name no plate-solver, pattern-database or finder (spec.md US2-S9, SC-005)"
    checks:
      development:
        - "cargo test -p scope-node --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; outside the factory, the static aarch64-unknown-linux-musl build and the on-sky checklist (spec.md Assumptions)"
    writes:
      - Cargo.lock
      - crates/scope-node/Cargo.toml
      - crates/scope-node/README.md
      - crates/scope-node/src/lib.rs
      - crates/scope-node/src/camera.rs
      - crates/scope-node/src/synthetic.rs
      - crates/scope-node/src/clock.rs
      - crates/scope-node/src/arducam/mod.rs
      - crates/scope-node/src/arducam/listing.rs
      - crates/scope-node/src/arducam/yuv.rs
      - crates/scope-node/src/arducam/metadata.rs
      - crates/scope-node/src/detect.rs
      - crates/scope-node/src/capture.rs
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/tests/arducam.rs
      - crates/scope-node/tests/detect.rs
      - crates/scope-node/tests/cli.rs
    shared_writes:
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US2"
      - path: crates/scope-node/Cargo.toml
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2 as well as US3, and adds image"
      - path: crates/scope-node/src/lib.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and declares service, preview and exposure"
      - path: crates/scope-node/src/capture.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and extends the capture loop"
      - path: crates/scope-node/src/main.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US2, and adds serving beside --check-config"
    open_questions: []
  US3:
    tests:
      - path: crates/simulator/src/optics.rs
        status: new
        disposition: add
        assertion: "the default optics are a 1456 x 1088 sensor, a 12 degree field and the eyepiece at (700, 420), each settable"
        decision: "spec.md:US3-S5"
      - path: crates/simulator/src/clock.rs
        status: new
        disposition: add
        assertion: "a paused-time VirtualClock started at 2026-01-16T02:00:00Z reads 1768528810 exactly after 10 s"
        decision: "spec.md:US3-S6"
      - path: crates/simulator/src/render.rs
        status: new
        disposition: add
        assertion: "the same direction, time, exposure and frame number render byte-identical frames, and the next frame number changes the noise"
        decision: "spec.md:US3-S4"
      - path: crates/simulator/src/sky.rs
        status: new
        disposition: add
        assertion: "the inverse of alt/az gives back the RA and Dec of Meeus's example 13.b"
        decision: "plan.md:Decisions and the alternatives rejected (The simulator, Sky to camera)"
      - path: crates/simulator/tests/sky.rs
        status: new
        disposition: add
        assertion: "SimCamera reports its model, size and ranges; a frame at alt 50, az 120 from 40 N, 75 W at 2026-01-16 02:00 UT solves within 30 arcseconds of the true camera direction; a star put in the eyepiece is detected at (700, 420) within 0.3 px"
        decision: "spec.md:US3-S2"
      - path: crates/finder/src/sky.rs
        status: existing
        disposition: preserve
        assertion: "the Meeus 13.b test of sky::alt_az passes unchanged; the simulator re-implements the inverse in its own crate"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "crates/math-core is used and not changed (constitution Principle IV)"
      - "crates/finder is untouched; the simulator cannot depend on it"
    checks:
      development:
        - "cargo test -p simulator --locked"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue; the coverage floor now counts the simulator crate (spec.md SC-003)"
    writes:
      - Cargo.lock
      - crates/simulator/Cargo.toml
      - crates/simulator/src/lib.rs
      - crates/simulator/src/optics.rs
      - crates/simulator/src/sky.rs
      - crates/simulator/src/noise.rs
      - crates/simulator/src/render.rs
      - crates/simulator/src/clock.rs
      - crates/simulator/src/camera.rs
      - crates/simulator/tests/sky.rs
    shared_writes:
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3"
      - path: crates/simulator/Cargo.toml
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds tonic for the loopback server"
      - path: crates/simulator/src/lib.rs
        other_spec: 013-live-finder
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and declares node and controls"
    open_questions: []
  US4:
    tests:
      - path: crates/scope-node/src/exposure.rs
        status: new
        disposition: add
        assertion: "the controller clamps each step to x4, holds within 25-40 stars, moves gain only at the exposure cap or when exposure is fixed, and never touches a fixed setting"
        decision: "spec.md:FR-015"
      - path: crates/scope-node/src/preview.rs
        status: new
        disposition: add
        assertion: "a preview is at most 640 px on its longer side, and at most one is sent per 5 s unless asked"
        decision: "spec.md:FR-013"
      - path: crates/simulator/tests/node.rs
        status: new
        disposition: add
        assertion: "over the in-memory link in paused time, frames carry every field with ids per exposure and 200 brightest centroids; previews obey the 5 s limit and RequestPreview; settings apply from the second frame; auto-exposure settles 5 stars to 25-40 within 6 frames; Describe agrees over memory and loopback TCP"
        decision: "spec.md:US4-S1"
    preserved_behavior:
      - "crates/scope-node/tests/cli.rs, added by US2, stays green unchanged: --check-config still masks the token and exits 0 (tasks.md T033, T035)"
      - "with no settings sent, every frame reports cedar's detection settings (spec.md US4-S5)"
      - "a setting the console fixed is never moved by auto-exposure (spec.md US4-S8)"
    checks:
      development:
        - "cargo test -p scope-node --locked"
        - "cargo test -p simulator --locked --test node"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - Cargo.lock
      - crates/scope-node/Cargo.toml
      - crates/scope-node/src/lib.rs
      - crates/scope-node/src/exposure.rs
      - crates/scope-node/src/preview.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/src/capture.rs
      - crates/scope-node/src/main.rs
      - crates/simulator/Cargo.toml
      - crates/simulator/src/lib.rs
      - crates/simulator/src/node.rs
      - crates/simulator/src/controls.rs
      - crates/simulator/tests/node.rs
    shared_writes:
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4"
    open_questions: []
  US5:
    tests:
      - path: crates/finder/tests/link.rs
        status: new
        disposition: add
        assertion: "against the simulator in paused time: at least 40 heartbeats in 10 s; disconnected 1 s (within 0.1 s) after the last heartbeat and back within 5.5 s; only Describe, Frames, Heartbeat and Configure, each with the bearer token; stored settings sent on connect; node in the session read while connected"
        decision: "spec.md:US5-S2"
      - path: crates/finder/tests/session_node.rs
        status: new
        disposition: add
        assertion: "the read gives the address and token_set true with test-token nowhere in it; a bad address and an out-of-range exposure answer 400 and change nothing; a foreign Origin answers 403"
        decision: "spec.md:US5-S4"
      - path: crates/finder/tests/session_node_cli.rs
        status: new
        disposition: add
        assertion: "the address, token and camera settings survive a restart exactly, in a state file of mode 0600"
        decision: "spec.md:US5-S5"
      - path: crates/finder/tests/session_api.rs
        status: existing
        disposition: preserve
        assertion: "a fresh read is still exactly site, target, pointing, min_alt_deg and time, because node and camera_settings are absent until set"
        decision: "spec.md:FR-021"
      - path: crates/finder/tests/session_cli.rs
        status: existing
        disposition: preserve
        assertion: "a session holding only a site, a minimum altitude and a target still writes a three-key state file, and 012's files still load"
        decision: "spec.md:FR-018"
      - path: crates/finder/tests/session_writes.rs
        status: existing
        disposition: preserve
        assertion: "a foreign Origin, a rebound Host and a non-JSON body are still refused with 403 and 415, and the node edits pass through the same check"
        decision: "spec.md:US5-S12"
      - path: crates/finder/tests/api.rs
        status: existing
        disposition: preserve
        assertion: "AppState::new, AppState::shared and router keep their shapes, and /api/health its body"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "012's test files and tests/api.rs are unchanged (spec.md US5-S12)"
      - "the token appears in no read, answer or log, and is written only to the 0600 state file (spec.md FR-018, Edge Cases)"
    checks:
      development:
        - "cargo test -p finder --locked --test link --test session_node --test session_node_cli"
        - "cargo test -p finder --locked --test session_api --test session_cli --test session_writes"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; from this story on, ci.yml's finder-page job needs the Install protoc step (owner approval 1, plan.md Prerequisites item 3)"
    writes:
      - Cargo.lock
      - crates/finder/Cargo.toml
      - crates/finder/README.md
      - crates/finder/src/lib.rs
      - crates/finder/src/link.rs
      - crates/finder/src/session.rs
      - crates/finder/src/api.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/link.rs
      - crates/finder/tests/session_node.rs
      - crates/finder/tests/session_node_cli.rs
    shared_writes:
      - path: crates/finder/src/lib.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5"
      - path: crates/finder/src/session.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5"
      - path: crates/finder/src/api.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5"
      - path: crates/finder/src/main.rs
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5"
      - path: crates/finder/README.md
        other_spec: 013-live-finder
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5"
      - path: Cargo.lock
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5 as well as US6; US6 changes no dependency"
      - path: crates/finder/Cargo.toml
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US5, and moves simulator into the dependencies"
    open_questions: []
  US6:
    tests:
      - path: crates/finder/tests/live.rs
        status: new
        disposition: add
        assertion: "in one paused runtime, every frame is solved within 30 arcseconds of the true eyepiece direction; the newest frame always wins; the field estimate carries and resets after three failures; mid-exposure maps through the link offset; a tap 6 px off calibrates the aim point to (700, 420); unsolved_s and the field report"
        decision: "spec.md:US6-S1"
      - path: crates/finder/tests/aim_api.rs
        status: new
        disposition: add
        assertion: "a tap more than 10 px from every centroid answers 422 and leaves the aim point; DELETE /api/aim returns pointings to the frame centre"
        decision: "spec.md:US6-S6"
      - path: crates/finder/tests/freshness_api.rs
        status: new
        disposition: add
        assertion: "a live pointing 5.1 s old gives stale guidance with no turn or tilt, connected or not, and the next solve restores them"
        decision: "spec.md:US6-S9"
      - path: crates/finder/tests/aim_cli.rs
        status: new
        disposition: add
        assertion: "the aim point (700, 420) survives a restart exactly, and the reset leaves none"
        decision: "spec.md:US6-S8"
      - path: crates/finder/tests/pointing_api.rs
        status: existing
        disposition: preserve
        assertion: "an uploaded frame's pointing keeps the RA, Dec, time and alt/az 012 pins, and the site still recomputes it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/pushto_api.rs
        status: existing
        disposition: preserve
        assertion: "a pointing from an uploaded frame never goes stale, and guidance keeps 012's fields"
        decision: "spec.md:FR-027"
      - path: crates/finder/src/solve.rs
        status: existing
        disposition: preserve
        assertion: "defaults_match_the_solve_image_example still pins SolveOptions' defaults; the live loop sets fov per solve"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "solve_frame keeps its signature and behaviour (plan.md US6)"
      - "012's test files are unchanged (tasks.md T052)"
    checks:
      development:
        - "cargo test -p finder --locked --test live --test aim_api --test freshness_api --test aim_cli"
        - "cargo test -p finder --locked --test pointing_api --test pushto_api"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue, including the cedar e2e eval, which the solver's unchanged path must still pass"
    writes:
      - crates/finder/README.md
      - crates/finder/src/lib.rs
      - crates/finder/src/solve.rs
      - crates/finder/src/sky.rs
      - crates/finder/src/live.rs
      - crates/finder/src/session.rs
      - crates/finder/src/pushto.rs
      - crates/finder/src/api.rs
      - crates/finder/src/main.rs
      - crates/finder/tests/live.rs
      - crates/finder/tests/aim_api.rs
      - crates/finder/tests/freshness_api.rs
      - crates/finder/tests/aim_cli.rs
    shared_writes:
      - path: crates/finder/src/lib.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6"
      - path: crates/finder/src/api.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6"
      - path: crates/finder/src/main.rs
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6"
      - path: crates/finder/README.md
        other_spec: 013-live-finder
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6"
    open_questions: []
  US7:
    tests:
      - path: crates/finder/tests/simulate_cli.rs
        status: new
        disposition: add
        assertion: "--simulate --sim-start keeps the session's time within 2 s and frames arrive; it never connects to the session's address; --sim-start alone exits nonzero naming it; the default state file stays byte for byte and the simulate folder is written"
        decision: "spec.md:US7-S3"
      - path: crates/finder/tests/sim_api.rs
        status: new
        disposition: add
        assertion: "a push to alt 40, az 200 solves within 30 arcseconds of it; every /api/sim path answers 404 outside simulate mode; frames stop and start; true directions read back; a foreign Origin answers 403 and nothing moves"
        decision: "spec.md:US7-S6"
      - path: crates/finder/tests/live_stream.rs
        status: new
        disposition: add
        assertion: "two solves give two live events carrying their reports, and /api/live/preview.jpg?seq=N answers the JPEG"
        decision: "spec.md:US7-S10"
      - path: crates/finder/tests/e2e_live.rs
        status: new
        disposition: add
        assertion: "from the first frame through eyepiece_on_star and a tap 6 px off to M78's guidance, the aim point is within 0.5 px and the turn and tilt within 30 arcseconds on the sky"
        decision: "spec.md:US7-S11"
      - path: crates/finder/tests/session_cli.rs
        status: existing
        disposition: preserve
        assertion: "outside --simulate the session is still written to the default XDG file"
        decision: "spec.md:FR-030"
      - path: crates/finder/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "--licenses still prints exactly the notices"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/session_writes.rs
        status: existing
        disposition: preserve
        assertion: "the same-origin check the simulator routes reuse still refuses foreign Origins and rebound Hosts"
        decision: "spec.md:FR-031"
    preserved_behavior:
      - "a virtual clock never reaches a real node: --sim-start without --simulate is refused before anything binds (spec.md FR-029)"
      - "012's test files are unchanged"
    checks:
      development:
        - "cargo test -p finder --locked --test simulate_cli --test sim_api --test live_stream --test e2e_live"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan, through the merge queue"
    writes:
      - Cargo.lock
      - crates/finder/Cargo.toml
      - crates/finder/README.md
      - crates/finder/src/lib.rs
      - crates/finder/src/simulate.rs
      - crates/finder/src/main.rs
      - crates/finder/src/api.rs
      - crates/finder/tests/simulate_cli.rs
      - crates/finder/tests/sim_api.rs
      - crates/finder/tests/live_stream.rs
      - crates/finder/tests/e2e_live.rs
    shared_writes: []
    open_questions: []
  US8:
    tests:
      - path: crates/finder/web/src/useLive.test.ts
        status: new
        disposition: add
        assertion: "no EventSource with no node or a disconnected one; one opened on connect and closed on disconnect; under fake timers, stale within 1 s of passing 5 s old"
        decision: "spec.md:US8-S2"
      - path: crates/finder/web/src/components/LivePreview.test.tsx
        status: new
        disposition: add
        assertion: "the preview image comes from /api/live/preview.jpg?seq=N, inside the element that carries the .photo red-only filter"
        decision: "spec.md:US8-S4"
      - path: crates/finder/web/src/App.live.test.tsx
        status: new
        disposition: add
        assertion: "a recorded live event draws its pointing and stars in the chart; with M78 set and the pointing past 5 s, the guidance block and the chart say stale and show no turn or tilt"
        decision: "spec.md:US8-S3"
      - path: crates/finder/web/src/App.test.tsx
        status: existing
        disposition: preserve
        assertion: "every App case passes unchanged; its fetch mock answers /api/session with the health body, so no EventSource opens"
        decision: "spec.md:US8-S1"
      - path: crates/finder/web/src/useSession.test.ts
        status: existing
        disposition: preserve
        assertion: "the empty session still equals site null, min_alt_deg undefined and target null, and polling stays at every 5 s while visible"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/Chart.test.tsx
        status: existing
        disposition: preserve
        assertion: "the legend's chips and the existing chart layers are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/GuidanceBlock.test.tsx
        status: existing
        disposition: preserve
        assertion: "012's guidance cases pass unchanged: the block still renders from session alone, and is still the one region named for the target"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/App.target.test.tsx
        status: existing
        disposition: preserve
        assertion: "with a target, the reading order is still the guidance block, the title block, the chart and its legend, then the rest"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "with M78 set, the guidance block, title block, chart and legend still fit the first screen at 412 x 915 and 390 x 844, and the console has no off-red pixel"
        decision: "plan.md:Plan traps"
    preserved_behavior:
      - "012's ChartHeader and GuidanceBlock props stay as they are; the stale state is optional (tasks.md T067)"
      - "package.json dependencies are unchanged (spec.md US8-S6)"
    checks:
      development:
        - "cd crates/finder/web && npm test -- src/useLive.test.ts src/components/LivePreview.test.tsx src/App.live.test.tsx"
        - "cd crates/finder/web && CI=1 npm run e2e -- --project session"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; the maintainer's Impeccable review of the live chart after US9 lands, outside the factory (spec.md Assumptions)"
    writes:
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/useLive.ts
      - crates/finder/web/src/useLive.test.ts
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/App.live.test.tsx
      - crates/finder/web/src/components/LivePreview.tsx
      - crates/finder/web/src/components/LivePreview.test.tsx
      - crates/finder/web/src/components/GuidanceBlock.tsx
      - crates/finder/web/src/components/ChartHeader.tsx
    shared_writes:
      - path: crates/finder/web/src/api.ts
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US8"
      - path: crates/finder/web/dist
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "US9 depends_on_merged US8, and rebuilds the bundle US8 rebuilt"
      - path: crates/finder/web/src/App.tsx
        other_spec: 012-targets-and-push-to
        other_story: US8
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US8 last wrote App.tsx (108a5a8)"
      - path: crates/finder/web/src/components/GuidanceBlock.tsx
        other_spec: 012-targets-and-push-to
        other_story: US7
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US7 created GuidanceBlock.tsx (cf09362)"
      - path: crates/finder/web/dist
        other_spec: 012-targets-and-push-to
        other_story: US8
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US8 rebuilt the bundle last (108a5a8)"
    open_questions: []
  US9:
    tests:
      - path: crates/finder/web/src/components/ScopeNodeSection.test.tsx
        status: new
        disposition: add
        assertion: "the token field stays empty and says a token is set, with no token in any request or answer; it shows connected, the model and the star count, updated by live events; a stored 2000 ms reads as outside the range; one PUT carries address, token, exposure, gain and sigma"
        decision: "spec.md:US9-S1"
      - path: crates/finder/web/src/components/FieldWarning.test.tsx
        status: new
        disposition: add
        assertion: "12 degrees against 10-30 shows no warning; 10.5 degrees warns; 30 s unsolved warns that the field may be outside the database's range"
        decision: "spec.md:US9-S4"
      - path: crates/finder/web/src/components/AimTap.test.tsx
        status: new
        disposition: add
        assertion: "a tap posts the scaled pixel with the frame id, the 422 message is shown, and reset clears the aim point"
        decision: "spec.md:US9-S7"
      - path: crates/finder/web/src/components/SetupPage.test.tsx
        status: existing
        disposition: preserve
        assertion: "012's Setup cases pass unchanged: SetupPage still renders with session, put and onBack alone, and its controls keep unique accessible names"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/components/SiteDialog.test.tsx
        status: existing
        disposition: preserve
        assertion: "site entry and its range messages are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/settings.test.ts
        status: existing
        disposition: preserve
        assertion: "the browser's stored settings behave as before"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "/setup opened directly still renders Setup, and Setup still has no off-red pixel with the new section in it"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no npm package is added (spec.md US9-S10)"
      - "every existing Vitest file is unchanged (tasks.md T074)"
    checks:
      development:
        - "cd crates/finder/web && npm test -- src/components/ScopeNodeSection.test.tsx src/components/FieldWarning.test.tsx src/components/AimTap.test.tsx src/components/SetupPage.test.tsx"
        - "cd crates/finder/web && CI=1 npm run e2e -- --project session"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; the maintainer's Impeccable review of Setup's new section, outside the factory (spec.md Assumptions)"
    writes:
      - DESIGN.md
      - .impeccable/design.json
      - crates/finder/web/dist
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/SetupPage.tsx
      - crates/finder/web/src/components/ScopeNodeSection.tsx
      - crates/finder/web/src/components/ScopeNodeSection.test.tsx
      - crates/finder/web/src/components/FieldWarning.tsx
      - crates/finder/web/src/components/FieldWarning.test.tsx
      - crates/finder/web/src/components/AimTap.tsx
      - crates/finder/web/src/components/AimTap.test.tsx
    shared_writes:
      - path: DESIGN.md
        other_spec: 012-targets-and-push-to
        other_story: US7
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US7 records its parts in DESIGN.md"
      - path: .impeccable/design.json
        other_spec: 012-targets-and-push-to
        other_story: US7
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US7 records its parts in design.json"
    open_questions: []
  US10:
    tests:
      - path: crates/finder/web/e2e/simulate.spec.ts
        status: new
        disposition: add
        assertion: "on the simulate server: the chart follows a push within 2 s and a post-push preview appears within 5.5 s; stopped frames give the stale state within 6 s; Setup shows connected, the model and 25-40 stars; the night pixel check passes; the aim tap lands at (700, 420) within 0.5 px"
        decision: "spec.md:US10-S2"
      - path: crates/finder/web/e2e/live-off.spec.ts
        status: new
        disposition: add
        assertion: "on the default server with no node, no request goes to /api/live and the page reaches network idle"
        decision: "spec.md:US10-S7"
      - path: crates/finder/web/e2e/console.spec.ts
        status: existing
        disposition: preserve
        assertion: "every console test passes unchanged in the default project"
        decision: "spec.md:US10-S1"
      - path: crates/finder/web/e2e/session.spec.ts
        status: existing
        disposition: preserve
        assertion: "every session test passes unchanged in the session project, on its own server on 8098"
        decision: "spec.md:US10-S1"
    preserved_behavior:
      - "012's default and session projects, their servers on 8097 and 8098, and e2e/session.spec.ts are unchanged (spec.md US10-S1)"
    checks:
      development:
        - "cd crates/finder/web && CI=1 npm run e2e"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: Bryan; 8097-8099 free on the factory host while it builds (plan.md Plan traps, trap 6); the on-sky checklist outside the factory"
    writes:
      - crates/finder/web/playwright.config.ts
      - crates/finder/web/e2e/simulate.spec.ts
      - crates/finder/web/e2e/live-off.spec.ts
    shared_writes:
      - path: crates/finder/web/playwright.config.ts
        other_spec: 012-targets-and-push-to
        other_story: US8
        disposition: merge_order
        reason: "013 depends_on_landed 012, whose US8 last wrote it (108a5a8)"
    open_questions: []
```
