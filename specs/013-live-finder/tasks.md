---
description: "Task list for 013-live-finder: The Live Finder"
---

# Tasks: The Live Finder

**Prerequisites** (plan.md § Prerequisites):
- 012 has landed;
- the bring-up fixtures are in `crates/scope-node/tests/data/`, and the bring-up notes are
  in `crates/scope-node/README.md`;
- the `finder-page` job installs protoc;
- the owner approvals in spec.md are given;
- the coding agent can run cargo (met since ergane#896, 2026-10-28);
- `npm run e2e` waits its turn on one host-wide lock (met by #125, at 80cbd51).

**Rules for every story:**
- Never edit an existing test, golden, `scripts/ci/`, `.github/` or `CODEOWNERS`.
- New Rust tests go in new files under the crate's `tests/`, or in-module.
- `crates/math-core`, `crates/grpc-service` and `proto/` are never touched.
- No `unsafe`, and no `.cargo/config.toml`.
- Hardware is reached only through the operator's saved fixtures, a fake `rpicam-vid` on
  `PATH`, and the simulator.
- Run only what the gates and the toolchain provide. The sandbox has no `cargo machete`
  or `cargo deny`; the merge queue runs both, so a new dependency is used by the code or
  sits in a `cargo-machete` ignore with its reason.
- Write to Rust 1.96, the MSRV. The gates build on stable; only CI's `msrv` job builds on
  1.96.
- Keep the story's diff under 64 KiB, `Cargo.lock` included (`dist/` shows only as changed
  files). The judge reads the diff and both gates' output within one 128 KiB budget, and
  refuses evidence past it.
- A test binds `127.0.0.1:0`. Only `playwright.config.ts` names a fixed port (8097–8099).
- Run Playwright only through `npm run e2e`. Two stories can be in flight at once, and
  every e2e run binds the host's 8097–8099, so each run waits its turn (plan.md,
  Prerequisite 6).
- A new field of a session read, or of the state file, is left out until it has a value
  (spec.md § Edge Cases).

## Phase 1: User Story 1 — the scope link (Priority: P1)

### Tests first

- [ ] T001 [US1] Write failing in-module tests in a new `crates/scope-link/src/clock.rs`:
  - exchanges A and B give delays 0.040 and 0.120, and offset +0.050 within 1 µs (US1-S3);
  - a 9 s-old least-delay exchange is ignored, and 200.100 maps to 200.050 (US1-S4).
- [ ] T002 [US1] Write failing in-module tests in a new `crates/scope-link/src/auth.rs`.
  `tokens_match` accepts an equal token, and refuses one differing in its last byte and
  one of another length (US1-S7).
- [ ] T003 [US1] Write a failing `crates/scope-link/tests/link.rs`, using a test
  `ScopeNode` behind `TokenInterceptor` over the in-memory transport:
  - no token and `wrong-token` both give UNAUTHENTICATED, with no frame (US1-S1);
  - `test-token` opens the stream and frames arrive (US1-S2).

### Implementation

- [ ] T004 [US1] Create `crates/scope-link/Cargo.toml`:
  - `tonic`, `prost`, `tonic-prost`, `tokio`, `tower`, `hyper-util` and `tokio-stream`
    at `grpc-service`'s versions;
  - `tonic-prost-build` as a build dependency;
  - a `[package.metadata.cargo-machete]` ignore for `prost` and `tonic-prost`, with the
    comment `grpc-service` gives.

  Update `Cargo.lock` (US1-S6).
- [ ] T005 [US1] Write `crates/scope-link/proto/scope_node/v1/scope_node.proto` as
  plan.md § Interfaces gives it, with no `[` in any comment. Add `crates/scope-link/build.rs`
  as `crates/grpc-service/build.rs` does (US1-S5, US1-S6).
- [ ] T006 [US1] In `crates/scope-link/src/lib.rs`, add `pub mod proto` with
  `tonic::include_proto!("scope_node.v1")` and `#[allow(clippy::double_must_use)]`. Add
  `pub mod auth; pub mod clock; pub mod memory;` and crate docs.
- [ ] T007 [US1] Implement `Exchange` and `ClockMap` (8 s window, least delay) in
  `crates/scope-link/src/clock.rs`.
- [ ] T008 [US1] Implement `tokens_match`, `TokenInterceptor` and `BearerToken` in
  `crates/scope-link/src/auth.rs`.
- [ ] T009 [US1] Implement the in-memory transport in `crates/scope-link/src/memory.rs`:
  `tokio::io::duplex`, `TokioIo`, `connect_with_connector` and `serve_with_incoming`.
- [ ] T010 [US1] Run the `lint` and `test` gates (`lint` builds the docs with warnings
  denied). Confirm that every dependency in `crates/scope-link/Cargo.toml` is used or sits
  in its `cargo-machete` ignore, and that `proto/` and `crates/grpc-service` are
  unchanged. Commit the green story (FR-001, FR-002, FR-003; US1-S1, US1-S2, US1-S3,
  US1-S4, US1-S5, US1-S6, US1-S7).

## Phase 2: User Story 2 — the node's camera port, Arducam adapter, detection and config (Priority: P1)

### Tests first

- [ ] T011 [US2] Write failing in-module tests in a new `crates/scope-node/src/camera.rs`.
  The `simulated` synthetic camera refuses an exposure past its range with an error naming
  the range, and takes no frame (US2-S1).
- [ ] T012 [US2] Write a failing `crates/scope-node/tests/arducam.rs` on
  `crates/scope-node/tests/data/`:
  - the listing parses to the sensor model, every mode and the active mode (US2-S2);
  - the padded stream gives the Y plane byte for byte, with the saved timestamps
    (US2-S3);
  - a fake `rpicam-vid` script, written to a temporary folder put first on `PATH`,
    replays the stream and exits. The error and its stderr are reported, and the capture
    loop restarts the camera in the same process (US2-S4).
- [ ] T013 [US2] Write a failing `crates/scope-node/tests/detect.rs`:
  - the saved finder frame, detected by `scope_node::detect` and by `detect_stars` with
    `DetectParams::cedar()`, gives identical lists (US2-S5);
  - `DetectSettings::cedar()` equals `DetectParams::cedar()` field for field (US2-S6).
- [ ] T014 [US2] Write a failing `crates/scope-node/tests/cli.rs` that runs the built
  binary:
  - `--check-config` with `test-token` prints every setting and `****`, never
    `test-token`, and exits 0 (US2-S7);
  - `exposre_ms`, and a gain of `loud`, each exit nonzero naming the key (US2-S8).

### Implementation

- [ ] T015 [US2] Create `crates/scope-node/Cargo.toml`:
  - a library and a `scope-node` binary;
  - `star-detection`, `scope-link`, `tokio`, `serde`, `toml` and `clap`, plus `rustix`
    only if the bring-up notes name `CLOCK_BOOTTIME`;
  - `plate-solver` as a dev-dependency.

  Update `Cargo.lock` (US2-S9).
- [ ] T016 [US2] Implement the camera port in `crates/scope-node/src/camera.rs`: the
  `Camera` trait, `CameraInfo`, `GrayFrame`, `Settings`, `check_range` and the error types.
  Implement the database-free `simulated` camera in `crates/scope-node/src/synthetic.rs`,
  and `NodeClock` in `crates/scope-node/src/clock.rs`. Declare the modules in
  `crates/scope-node/src/lib.rs`.
- [ ] T017 [US2] Implement the Arducam adapter in `crates/scope-node/src/arducam/`:
  - `listing.rs`;
  - `yuv.rs` (stride framing);
  - `metadata.rs` (`SensorTimestamp`, `ExposureTime`);
  - `mod.rs`, which spawns `rpicam-vid` from `PATH` with plan.md's arguments, keeps
    stderr, and reports exit.
- [ ] T018 [US2] Implement `crates/scope-node/src/detect.rs`. It holds
  `DetectSettings::cedar()`, and `detect` calls `star_detection::detect_stars` as
  `finder::solve::solve_frame` does and keeps the brightest 200.
- [ ] T019 [US2] Implement the capture loop with restart-after-error in
  `crates/scope-node/src/capture.rs`. Implement the config (`deny_unknown_fields`, errors
  naming the key, the masked print) in `crates/scope-node/src/config.rs`. Implement
  `--config` and `--check-config` in `crates/scope-node/src/main.rs`.
- [ ] T020 [US2] Add to `crates/scope-node/README.md`, below the operator's bring-up
  notes and without changing them:
  - the static `aarch64-unknown-linux-musl` build, with its target and flags on the
    command line;
  - the systemd unit (`Restart=always`, a dedicated user in the `video` group);
  - the config file at mode 0600 (US2-S9).
- [ ] T021 [US2] Run the `lint` and `test` gates. Confirm four things:
  - every dependency in `crates/scope-node/Cargo.toml` is used;
  - `scope-node`'s `[dependencies]` name no `plate-solver`, `pattern-database` or
    `finder`;
  - there is no `.cargo/config.toml`;
  - the fixtures are unchanged.

  Commit the green story (FR-004, FR-005, FR-006, FR-007, FR-008; US2-S1, US2-S2, US2-S3,
  US2-S4, US2-S5, US2-S6, US2-S7, US2-S8, US2-S9).

## Phase 3: User Story 3 — the simulator's optics, sky and virtual clock (Priority: P1)

### Tests first

- [ ] T022 [US3] Write failing in-module tests in new `crates/simulator/src/` files:
  - in `optics.rs`, the defaults (1456 × 1088, 12°, (700, 420)) and that each can be set
    (US3-S5);
  - in `clock.rs`, a paused-time `VirtualClock` from 2026-01-16T02:00:00Z reads
    1768528810 after 10 s (US3-S6);
  - in `render.rs`, the same inputs render byte-identical frames and the next frame id
    changes the noise (US3-S4);
  - in `sky.rs`, the inverse alt/az against Meeus's example 13.b.
- [ ] T023 [US3] Write a failing `crates/simulator/tests/sky.rs`:
  - `SimCamera` reports model, size and ranges, and yields a gray frame with end − start
    equal to the exposure within 1 µs (US3-S1);
  - alt 50°, az 120° from 40° N, 75° W at 2026-01-16 02:00 UT, detected by
    `scope_node::detect` and solved by `solve_from_centroids` on the bundled database,
    lands within 30″ of the true camera direction (US3-S2);
  - with `point_eyepiece_at` on a star, its centroid is at (700, 420) within 0.3 px
    (US3-S3).

### Implementation

- [ ] T024 [US3] Create `crates/simulator/Cargo.toml` with `scope-node`, `scope-link`,
  `pattern-database`, `math-core` and `tokio`, and as dev-dependencies `plate-solver` and
  `tokio` with `test-util` (which `start_paused` needs). Update `Cargo.lock`. Declare the
  modules in `crates/simulator/src/lib.rs`.
- [ ] T025 [US3] Implement the optics, the true directions and `point_eyepiece_at` in
  `crates/simulator/src/optics.rs`. Implement alt/az to J2000 and the parallactic roll in
  `crates/simulator/src/sky.rs` (US3-S5).
- [ ] T026 [US3] Implement SplitMix64 in `crates/simulator/src/noise.rs`. Implement the
  magnitude limit, Gaussian PSFs (σ 1.2 px), background, Poisson and read noise, seeded by
  frame id, in `crates/simulator/src/render.rs`.
- [ ] T027 [US3] Implement `VirtualClock` in `crates/simulator/src/clock.rs`. Implement
  `SimCamera` behind `scope_node::Camera` in `crates/simulator/src/camera.rs`, with
  settable ranges (default 1–2000 ms, gain 1–16).
- [ ] T028 [US3] Run the `lint` and `test` gates, and confirm that every dependency in
  `crates/simulator/Cargo.toml` is used. Commit the green story (FR-009, FR-010, FR-011;
  US3-S1, US3-S2, US3-S3, US3-S4, US3-S5, US3-S6).

## Phase 4: User Story 4 — the node's frames, previews, settings and auto-exposure (Priority: P1)

### Tests first

- [ ] T029 [US4] Write failing in-module tests in a new `crates/scope-node/src/exposure.rs`
  for the controller:
  - the ×4 clamp;
  - no change in 25–40;
  - gain moving only when exposure is at its cap or fixed;
  - fixed settings never touched.

  Write failing in-module tests in a new `crates/scope-node/src/preview.rs` for the
  scaling to at most 640 px and the 5 s limit.
- [ ] T030 [US4] Write a failing `crates/simulator/tests/node.rs`, in paused time over the
  in-memory transport with `BearerToken`:
  - ten frames of a 20° Milky Way field that first holds more than 200 database stars:
    every field present, ids +1, end − start equal to the exposure within 1 µs, exactly
    200 centroids descending (US4-S1);
  - a dropped message shows as id +2 (US4-S2);
  - 2 frames a second for 20 s give at most 5 JPEG previews, each at most 640 px and at
    least 5 s apart (US4-S3);
  - `RequestPreview` 1 s after a preview puts one on the next frame (US4-S4);
  - cedar's settings by default (US4-S5);
  - 5σ reported from the second frame after (US4-S6);
  - from 5 stars to 25–40 within 6 frames, and staying there (US4-S7);
  - 200 ms and gain 4 held on a 5-star field (US4-S8);
  - 200 ms with gain automatic reaching 25–40 within 6 frames (US4-S9);
  - `Describe` answers the same camera over the in-memory transport and over loopback TCP
    on a random port (US4-S10).

### Implementation

- [ ] T031 [US4] Implement the auto-exposure controller in
  `crates/scope-node/src/exposure.rs`. Implement scaling and JPEG encoding with `image`'s
  `jpeg` feature in `crates/scope-node/src/preview.rs`. Add `image` to
  `crates/scope-node/Cargo.toml` and update `Cargo.lock`. Declare both modules, and
  `service`, in `crates/scope-node/src/lib.rs`.
- [ ] T032 [US4] Implement `NodeService` in `crates/scope-node/src/service.rs`, over the
  capture loop in `crates/scope-node/src/capture.rs`:
  - `Describe`;
  - `Frames` (a broadcast, ids per exposure, the 200 brightest);
  - `Heartbeat` (t1 and t2 from `NodeClock`);
  - `Configure`, which applies from the next exposure and answers the settings in force;
  - `RequestPreview`.
- [ ] T033 [US4] Serve `NodeService` on the configured address behind `TokenInterceptor`
  in `crates/scope-node/src/main.rs`, keeping `--check-config` as it is.
- [ ] T034 [US4] Implement `SimNode` in `crates/simulator/src/node.rs` (the real
  `NodeService` over `SimCamera`, with a settable frame interval, in memory or on loopback
  TCP). Implement the push, stop and start frames, and unresponsive controls in
  `crates/simulator/src/controls.rs`. Declare both in `crates/simulator/src/lib.rs`. Add
  `tonic` to `crates/simulator/Cargo.toml` for the loopback server, and update
  `Cargo.lock`.
- [ ] T035 [US4] Run the `lint` and `test` gates, and confirm that `image` and `tonic` are
  used. Confirm `crates/scope-node/tests/cli.rs` is still green. Commit the green story
  (FR-012, FR-013, FR-014, FR-015, FR-016; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6,
  US4-S7, US4-S8, US4-S9, US4-S10).

## Phase 5: User Story 5 — the link client, and the session's node and camera settings (Priority: P1)

### Tests first

- [ ] T036 [US5] Write a failing `crates/finder/tests/link.rs`, with the simulator's node
  over the in-memory transport in paused time:
  - at least 40 heartbeats in 10 s (US5-S1);
  - 3 s unresponsive gives disconnected at 1 s ±0.1 s after the last heartbeat, and
    reconnected within 5.5 s of the return (US5-S2);
  - a recording interceptor sees only `Describe`, `Frames`, `Heartbeat` and `Configure`
    with `authorization: Bearer` (US5-S3);
  - 200 ms put before connecting is reported from the second frame after (US5-S7);
  - a stored 2000 ms against a 1000 ms range gives automatic from the second frame after,
    and `node.out_of_range` names it (US5-S10);
  - `GET /api/session`'s `node` while connected (US5-S11).
- [ ] T037 [US5] Write a failing `crates/finder/tests/session_node.rs` through the router
  with a `FixedClock`:
  - the address and `token_set: true`, with `test-token` nowhere in the read (US5-S4);
  - `not an address` gives 400 with an `error`, and nothing changes (US5-S6);
  - an exposure past the connected camera's range gives 400 naming the range, and
    nothing changes (US5-S8);
  - an `Origin` from elsewhere on a node edit gives 403 (US5-S12).
- [ ] T038 [US5] Write a failing `crates/finder/tests/session_node_cli.rs` that restarts
  the built binary on one `--state`:
  - the address and token kept, with the file at mode 0600 (US5-S5);
  - 200 ms, gain 4 and 5σ kept exactly (US5-S9).

### Implementation

- [ ] T039 [US5] Add `scope-link`, `tonic`, `tower` and `hyper-util` to
  `crates/finder/Cargo.toml`, and as dev-dependencies `simulator` and `tokio` with
  `test-util` (which `start_paused` needs). Update `Cargo.lock`.
- [ ] T040 [US5] Implement `LinkManager` and `LinkStatus` in a new
  `crates/finder/src/link.rs`:
  - the transport choice (TCP address or connector);
  - `Describe`, `Configure` on connect and on change, `Frames` into a `watch`, and the
    heartbeat into `ClockMap`;
  - disconnected after 1 s, and the 0.5/1/2/4/5 s backoff.

  Add `pub mod link;` to `crates/finder/src/lib.rs`.
- [ ] T041 [US5] Extend `crates/finder/src/session.rs`:
  - the node's address and token (write-only, `token_set`);
  - the camera settings with their validation;
  - the out-of-range rule;
  - 0600 atomic writes;
  - `node` and `camera_settings` left out of the read, and out of the state file, until
    set, so `tests/session_api.rs`'s fresh read and `tests/session_cli.rs`'s three-key
    file stay green unchanged.
- [ ] T042 [US5] Extend `crates/finder/src/api.rs`: the `node` and `camera_settings` read
  fields, the `PUT` fields with `deny_unknown_fields` behind the same-origin check, and
  `AppState::with_link`. Start the link in `crates/finder/src/main.rs`. Document the new
  fields in `crates/finder/README.md`.
- [ ] T043 [US5] Run the `lint` and `test` gates, and confirm that every dependency added
  to `crates/finder/Cargo.toml` is used. Confirm 012's test files and
  `crates/finder/tests/api.rs` are unchanged (US5-S12). Commit the green story (FR-017,
  FR-018, FR-019, FR-020, FR-021, FR-022; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5, US5-S6,
  US5-S7, US5-S8, US5-S9, US5-S10, US5-S11, US5-S12).

## Phase 6: User Story 6 — live solving, the aim point and fresh pointings (Priority: P1)

### Tests first

- [ ] T044 [US6] Write a failing `crates/finder/tests/live.rs`, with the simulator and
  the console in one paused runtime:
  - with the aim point at the eyepiece, 2 frames a second for 10 s are all solved, each
    within 30″ of the true eyepiece direction at mid-exposure (US6-S1);
  - a 1.5 s solver seam always takes the newest frame, never older after newer (US6-S2);
  - 12.04° carried as the estimate, and no estimate after three failures (US6-S3);
  - offset +0.050 and an exposure of 200.000–200.200 give 200.050 within 1 µs (US6-S4);
  - a tap 6 px off the star at the eyepiece gives (700, 420) within 0.5 px, and later
    pointings within 30″ of the true eyepiece direction (US6-S5);
  - a failing solver seam for 30 s of 25+ star frames reports `unsolved_s` at least 30
    (US6-S11);
  - the 12° field reported within 0.1°, against 10–30° (US6-S12).
- [ ] T045 [US6] Write a failing `crates/finder/tests/aim_api.rs` through the router:
  - a tap more than 10 px from every centroid gives 422, says no star is close enough, and
    leaves the aim point unchanged (US6-S6);
  - after `DELETE /api/aim`, pointings are at the frame centre (US6-S7).
- [ ] T046 [US6] Write a failing `crates/finder/tests/freshness_api.rs` with a
  `FixedClock`:
  - connected, 5.1 s old, gives no turn or tilt and `stale: true`, and both return with
    the next solve (US6-S9);
  - the link dropped and past 5 s, still stale (US6-S10).
- [ ] T047 [US6] Write a failing `crates/finder/tests/aim_cli.rs`. (700, 420) survives a
  restart exactly, and the reset leaves none (US6-S8).

### Implementation

- [ ] T048 [US6] Add `solve_centroids` to `crates/finder/src/solve.rs`, keeping
  `solve_frame`'s signature. Add `Frame::unproject` to `crates/finder/src/sky.rs`.
- [ ] T049 [US6] Implement the live task in a new `crates/finder/src/live.rs`:
  - the `Solver` seam with `DbSolver` on `spawn_blocking`;
  - the FOV estimate with its 5% allowance and three-failure reset;
  - the aim-point pointing at mapped mid-exposure;
  - the preview store of 8;
  - the broadcast;
  - `unsolved_s`.

  Add `pub mod live;` to `crates/finder/src/lib.rs`.
- [ ] T050 [US6] Extend `crates/finder/src/session.rs` with the aim point (kept,
  resettable), the pointing's `source` and `frame_id`, and the `field` report, leaving
  `aim` and `field` out of the read, and the aim point out of the state file, until set.
  Extend `crates/finder/src/pushto.rs` with the 5 s stale rule for live pointings.
- [ ] T051 [US6] Extend `crates/finder/src/api.rs` with `POST`/`DELETE /api/aim` behind
  the same-origin check, the `aim` and `field` read fields, and `AppState::with_live`.
  Start the live task in `crates/finder/src/main.rs`. Document the routes in
  `crates/finder/README.md`.
- [ ] T052 [US6] Run the `lint` and `test` gates. Confirm 012's test files are unchanged.
  Commit the green story (FR-023, FR-024, FR-025, FR-026, FR-027, FR-028; US6-S1, US6-S2,
  US6-S3, US6-S4, US6-S5, US6-S6, US6-S7, US6-S8, US6-S9, US6-S10, US6-S11, US6-S12).

## Phase 7: User Story 7 — `finder --simulate`, its controls and the live stream (Priority: P1)

### Tests first

- [ ] T053 [US7] Write a failing `crates/finder/tests/simulate_cli.rs` that runs the
  built binary:
  - `--sim-start 2026-01-16T02:00:00Z` gives the session's time within 2 s, and frames
    arrive (US7-S1);
  - a listening test server at the session's address gets no connection in 5 s (US7-S2);
  - `--sim-start` without `--simulate` exits nonzero naming it, and serves nothing
    (US7-S3);
  - without `--state`, under a temporary `XDG_STATE_HOME`, the default file is unchanged
    byte for byte and the `simulate` folder is written (US7-S4).
- [ ] T054 [US7] Write a failing `crates/finder/tests/sim_api.rs` through the router:
  - a push to alt 40°, az 200° solves within 30″ of it by `sky::alt_az` (US7-S5);
  - 404 on every `/api/sim/*` path without simulate mode (US7-S6);
  - frames stop and start (US7-S7);
  - the true directions are read back (US7-S8);
  - a foreign `Origin` gives 403 and nothing moves (US7-S9).
- [ ] T055 [US7] Write a failing `crates/finder/tests/live_stream.rs`. Two solves give two
  `live` events carrying their reports, and `/api/live/preview.jpg?seq=N` answers the
  JPEG (US7-S10).
- [ ] T056 [US7] Write a failing `crates/finder/tests/e2e_live.rs`. From the first frame,
  through `eyepiece_on_star` and a tap 6 px off, to M78's guidance, the aim point is
  within 0.5 px and the turn and tilt are within 30″ on the sky (US7-S11).

### Implementation

- [ ] T057 [US7] Move `simulator` to `[dependencies]` in `crates/finder/Cargo.toml`, add
  `futures-util` there, and update `Cargo.lock`.
- [ ] T058 [US7] Implement a new `crates/finder/src/simulate.rs`:
  - the `Simulator` at the session's site, or at 40° N, 75° W;
  - `VirtualClock` as `finder`'s `Clock`;
  - `SimNode` on `127.0.0.1:0` with a random token, and the link pointed at it;
  - the `/api/sim/*` routes behind the same-origin check, added only in simulate mode.

  Add `pub mod simulate;` to `crates/finder/src/lib.rs`.
- [ ] T059 [US7] Add `--simulate` and `--sim-start` to `crates/finder/src/main.rs`:
  - the in-crate RFC 3339 `Z` parse;
  - refusal before binding;
  - the `simulate` default state folder.
- [ ] T060 [US7] Add `GET /api/live` (axum `Sse` over the broadcast, keep-alive 15 s) and
  `GET /api/live/preview.jpg` to `crates/finder/src/api.rs`. Document `--simulate`,
  `--sim-start` and the routes in `crates/finder/README.md`.
- [ ] T061 [US7] Run the `lint` and `test` gates, and confirm that `futures-util` and
  `simulator` are used. Commit the green story (FR-029, FR-030, FR-031, FR-032, FR-033;
  US7-S1, US7-S2, US7-S3, US7-S4, US7-S5, US7-S6, US7-S7, US7-S8, US7-S9, US7-S10, US7-S11).

## Phase 8: User Story 8 — the live chart and a stale pointing (Priority: P2)

### Tests first

- [ ] T062 [US8] Write failing Vitest tests in a new `crates/finder/web/src/useLive.test.ts`
  with a fake `EventSource`:
  - no stream with no node or a disconnected one (US8-S1);
  - opened on connect and closed on disconnect (US8-S2);
  - under fake timers, stale within 1 s of passing 5 s old (US8-S5).
- [ ] T063 [US8] Write failing Vitest tests in a new
  `crates/finder/web/src/components/LivePreview.test.tsx`. The preview image comes from
  `/api/live/preview.jpg?seq=N`, inside the `.photo` element (US8-S4).
- [ ] T064 [US8] Write failing Vitest tests in a new
  `crates/finder/web/src/App.live.test.tsx`:
  - a live event written from plan.md § Interfaces (the live stream) draws its pointing
    and stars in the chart (US8-S3);
  - with M78 set and the pointing past 5 s, the guidance block and the chart say stale and
    show no turn or tilt (US8-S5).

### Implementation

- [ ] T065 [US8] Add the live report, `node`, `camera_settings`, `aim`, `field` and
  `stale` types, all optional, to `crates/finder/web/src/api.ts`. A missing field reads as
  `undefined`, never `null`, so `useSession.test.ts`'s empty session stays equal. Add
  `crates/finder/web/src/useLive.ts`.
- [ ] T066 [US8] Add `crates/finder/web/src/components/LivePreview.tsx`. Draw the live
  report in the chart, and put the preview below the chart's legend, in
  `crates/finder/web/src/App.tsx`. With a target on a 390 × 844 phone the legend ends
  8 px above the bottom of the first screen, so nothing new goes above it (plan.md
  § Plan traps, trap 10).
- [ ] T067 [US8] Add the optional stale state to
  `crates/finder/web/src/components/GuidanceBlock.tsx` and
  `crates/finder/web/src/components/ChartHeader.tsx`, keeping their existing props. On a
  phone with a target, the stale state makes neither block taller, so 012's first-screen
  test still passes at 412 × 915 and 390 × 844.
- [ ] T068 [US8] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`, then `npm run build`. Commit the rebuilt
  `crates/finder/web/dist`, and confirm `git status --porcelain -- dist` lists nothing
  after a second `npm run build`, as the `test` gate's build-freshness step does (US8-S6).
  Run `CI=1 npm run e2e`, with `e2e/console.spec.ts` and `e2e/session.spec.ts` unchanged
  and green. Run the `lint` and `test` gates. Confirm `package.json` dependencies and
  every existing Vitest file are unchanged. Commit the green story (FR-034, FR-035;
  US8-S1, US8-S2, US8-S3, US8-S4, US8-S5, US8-S6).

## Phase 9: User Story 9 — Setup's scope node and camera section (Priority: P2)

### Tests first

- [ ] T069 [US9] Write failing Vitest tests in a new
  `crates/finder/web/src/components/ScopeNodeSection.test.tsx`:
  - an empty token field that says a token is set, with no token in any request or answer
    (US9-S1);
  - connected, the model and the star count, updated by live events (US9-S2);
  - a stored 2000 ms reported as outside the range (US9-S6);
  - one `PUT /api/session` with the address, token, exposure, gain and sigma (US9-S8).
- [ ] T070 [US9] Write failing Vitest tests in a new
  `crates/finder/web/src/components/FieldWarning.test.tsx`:
  - 12° against 10–30° with no warning (US9-S3);
  - 10.5° with the warning (US9-S4);
  - 30 s unsolved warns of a field outside 10–30° (US9-S5).
- [ ] T071 [US9] Write failing Vitest tests in a new
  `crates/finder/web/src/components/AimTap.test.tsx`. A tap posts the scaled pixel with
  the frame id, the 422 message is shown, and reset clears the aim point (US9-S7).

### Implementation

- [ ] T072 [US9] Add `crates/finder/web/src/components/ScopeNodeSection.tsx`,
  `FieldWarning.tsx` and `AimTap.tsx`, and their aim and put helpers in
  `crates/finder/web/src/api.ts`. Host them in
  `crates/finder/web/src/components/SetupPage.tsx`, keeping 012's Setup tests green: any
  new `SetupPage` prop is optional, and no new control takes an accessible name those
  tests look up ("Save site", "Use this device's location", "Forget the site",
  "Minimum altitude", "Save minimum altitude", "Back to console").
- [ ] T073 [US9] Record the live chart, the preview, the stale state, the scope node and
  camera section and the aim-point tap in `DESIGN.md` and `.impeccable/design.json`, as
  built (US9-S9).
- [ ] T074 [US9] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`, then `npm run build`. Commit the rebuilt
  `crates/finder/web/dist`, and confirm `git status --porcelain -- dist` lists nothing
  after a second `npm run build` (US9-S10). Run `CI=1 npm run e2e`, with
  `e2e/console.spec.ts` and `e2e/session.spec.ts` unchanged and green; the latter opens
  `/setup` directly and finds no off-red pixel on it. Run the `lint` and `test` gates.
  Confirm no npm package was added and every existing Vitest file is unchanged. Commit
  the green story (FR-036, FR-037, FR-038, FR-039, FR-040; US9-S1, US9-S2, US9-S3,
  US9-S4, US9-S5, US9-S6, US9-S7, US9-S8, US9-S9, US9-S10).

## Phase 10: User Story 10 — end to end, on the simulator (Priority: P3)

### Tests first

- [ ] T075 [US10] Write `crates/finder/web/e2e/simulate.spec.ts`, serial, reading true
  directions from `GET /api/sim/telescope`:
  - a push, then the chart within 30″ in 2 s, and a post-push preview in 5.5 s (US10-S2);
  - with M78 at 40° N, 75° W, frames stopped give the stale state within 6 s of the last
    mid-exposure (US10-S3);
  - Setup connected, with the model and 25–40 stars updated within 2 s (US10-S4);
  - the night pixel check on the live chart with a preview (US10-S6);
  - last, the aim tap, giving (700, 420) within 0.5 px (US10-S5).
- [ ] T076 [US10] Write `crates/finder/web/e2e/live-off.spec.ts` for the default
  project. It asserts no request to `/api/live`, and network idle (US10-S7).

### Implementation

- [ ] T077 [US10] In `crates/finder/web/playwright.config.ts`:
  - add the third server, `finder --listen 127.0.0.1:8099 --simulate --sim-start
    2026-01-16T02:00:00Z`, with a fresh `--state` under `target/` and
    `reuseExistingServer: false`, serving a `simulate` project that matches only
    `e2e/simulate.spec.ts`;
  - make the default project ignore it beside `**/session.spec.ts`; the `session`
    project already matches only its own file;
  - pass the new flags through 012's `server(port, name)` helper as optional extra
    arguments, so the two existing servers' commands stay as they are;
  - leave every earlier server and setting as it is (US10-S1).
- [ ] T078 [US10] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check`, `npm test` and `CI=1 npm run e2e` (as the `test` gate runs it,
  so no server is reused), with `e2e/console.spec.ts` and `e2e/session.spec.ts` unchanged
  and green. Run the `lint` and `test` gates. Commit the green story (FR-041, FR-042;
  US10-S1, US10-S2, US10-S3, US10-S4, US10-S5, US10-S6, US10-S7).

## Execution Order

The stories land in two tracks that join at US10:

```text
US1 → US2 → US3 → US4 → US5 → US6 → US7 ─┐
US8 → US9 ────────────────────────────────┴→ US10
```

Every arrow is a merge edge: the later story starts from a `main` that holds the earlier
one. The factory keeps a story from each track in flight, so US8 starts with US1. It has
one coding agent, so the two never build at the same moment: one builds while the other
is verified or in the merge queue.
- **US1 to US7** each change `Cargo.lock`, and a new crate's `Cargo.toml`, or `finder`'s.
- **US5, US6 and US7** each edit `crates/finder/src/api.rs`, `src/session.rs`, `src/lib.rs`
  and `src/main.rs`.
- **US8 and US9** both rebuild `crates/finder/web/dist` and edit `src/api.ts`.
- **US8 and US9 share no file with US1 to US7.** They write only `crates/finder/web/**`,
  and their tests fake the server from plan.md § Interfaces. `concurrent_with: [US7]` on
  US8 overrides the edge the factory would infer from both naming the route
  `/api/live/preview.jpg`.
- **US10** drives the finished page on `finder --simulate`, so it waits for US7 and US9.
- **US2 → US4 and US5 → US7** are stated as edges too, though the first track implies them: US4
  extends US2's node files, and US7 changes `crates/finder/Cargo.toml` and `Cargo.lock`
  after US5.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `crates/grpc-service`, `proto/`,
  `scripts/ci/`, `.github/`, existing tests, and the bring-up fixtures.
