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
  the console in one runtime over an in-memory transport.
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
node: { address: string|null, token_set: bool, connected: bool,
        last_frame_age_s: number|null, offset_s: number|null,
        camera: {model, width, height, exposure_ms: {min, max}, gain: {min, max}} | null,
        stars: number|null,
        out_of_range: [{setting: "exposure_ms"|"gain", stored: number, min, max}] }
camera_settings: { exposure_ms: "auto"|number, gain: "auto"|number, sigma: number }
aim: {x, y} | null
field: { fov_deg: number|null, db_min_fov_deg, db_max_fov_deg, unsolved_s: number }
pointing: { …012's fields…, source: "upload"|"live", frame_id? }
guidance: { …012's fields…, stale?: true }    // stale: no turn_deg or tilt_deg
```

The token is never in any answer.

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
  renamed over the old one. An existing file's mode is tightened on the next write.
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

**`e2e/simulate.spec.ts`.**
- It runs serially and sets the site and M78 first.
- It reads true directions from `GET /api/sim/telescope`. It compares the chart against
  the camera direction while the session has no aim point, and against the eyepiece
  direction once it does.
- The aim tap runs last.
- The night pixel check reuses 012's helper.

**`e2e/live-off.spec.ts`** runs in the default project, against the server with no node.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check` and
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked` and
    `cargo test --workspace --locked --doc`.
- **Rust stories** (US1–US7) also run
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`, and
  `cargo machete` when they add a dependency.
- **US5 onwards** must keep `finder-page` green: from US5, `cargo run -p finder` builds
  `scope-link`, which relies on the operator's protoc step.
- **Page stories** (US8, US9), in `crates/finder/web`: `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`. Then `npm run build`, and confirm that
  `git status` shows `dist/` matching the rebuild (CI's freshness check).
- **US10** also runs `npm run e2e`, which builds the release binary.
- **CI in the merge queue** runs all of the above and more: coverage at 80%, the cedar
  e2e eval and canary, msrv, docs, deny, machete and the secret scan. A story is not done
  until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **012 has landed.** This spec's `depends_on_landed` holds it back until then.
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
   same "Install protoc" step the Rust jobs use in `.github/workflows/ci.yml`.
4. **The owner approvals are given.** They are listed in spec § Owner approvals, and
   flipping to `ready` records them.
5. **The factory's coding agent can run cargo.** Every story from US1 to US7 changes
   `Cargo.lock`, and US2 adds `toml`, which a hand edit cannot lock. Until the agent's
   sandbox can run cargo, these stories cannot be written blind against `--locked` gates.

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
| A page story passes the cargo gates but fails `finder-page` in the queue. | Each page story's tasks run the npm checks before proposing. The queue failure is reported back to the node's ladder. |
