# Implementation Plan: GoTo

**Branch**: `014-goto` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The work runs from the node to the page:

1. Give the scope node a mount port, with a simulated mount and a Sky-Watcher adapter
   behind it.
2. Make Stop win on the node, and add a heartbeat watchdog.
3. Put axis positions on star frames, and carry the mount over the scope link.
4. In the console, add apparent altitude.
5. Add a pointing model fitted from syncs, and a closed-loop GoTo that plans through the
   model and the Sun guard and corrects to 1′.
6. Add the one-tap second sync and the model routes.
7. Add GoTo and Stop on the page, and Setup's mount and model section.

Everything is proven on the simulator in paused virtual time, and the adapter against a
byte-level emulator. `crates/math-core` is untouched, and no existing test changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- already present after 013: tokio, tonic and prost in `crates/scope-link`, axum in
  `finder`;
- the page: Vite, Vitest and Playwright.

The one new crate is `serialport` (`default-features = false`) in `crates/scope-node`, or
`rustix` termios in its place (owner approval 1). The model's fit is written in-crate,
with no linear-algebra dependency. No npm package is added.

**Storage**: the session's state file from 012. It gains the night's syncs, zero points
and base tilt, kept until the next local noon, and the fixed terms, kept until reset.

**Testing**:
- `cargo test --workspace --locked` runs:
  - unit tests in-module;
  - integration tests in new files under `crates/scope-node/tests/`,
    `crates/scope-link/tests/`, `crates/simulator/tests/` and `crates/finder/tests/`.
- Simulator tests use `#[tokio::test(start_paused = true)]` with 013's `VirtualClock` and
  in-process `tokio::io::duplex` link.
- Vitest and Playwright (013's `simulate` project) run in `crates/finder/web`.

**Target Platform**:
- the scope node on a Raspberry Pi 3B (`aarch64-unknown-linux-musl`), with a USB–TTL
  cable in the mount's hand-controller jack;
- the console on desktop Linux, WSL and the Pi;
- the page on Android phones and laptops.

**Constraints**:
- **Rust and the page:** `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc,
  and the 80% coverage floor (Rust only). `web/dist` is committed and must be the build of
  its source, and night mode is red only.
- **Safety:** Stop needs no token and wins everywhere, and the Sun guard cannot be
  overridden.

**Scale/Scope**: one mount with two axes, a few hundred syncs a night, and at most
8 model parameters.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The solver and `crates/math-core` are untouched. The simulator still uses math-core's camera model, unchanged. New numerics are checked against Meeus (refraction), synthetic TPoint terms recovered within 1″, and the simulator's known truth. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The proto changes only add. |
| III. Tests first, independent expectations | Every scenario names its oracle: Meeus ch. 16, the motor-controller command set's byte strings, synthetic model terms, or the simulator's true eyepiece direction sampled in virtual time. |
| IV. Critical paths | One dependency change, `serialport` or `rustix`, owner-approved. `scope_node.v1` gains calls only. No `unsafe` is planned: `serialport` hides the termios calls, and if `rustix` is used instead, its safe API is. |
| V. Standalone Rust, honest licensing | `serialport` is MPL-2.0, which is in `deny.toml`'s allow-list. No reference source is copied. |

## Project Structure

```text
crates/scope-node/
├── Cargo.toml                          # US3: serialport (default-features = false) or rustix
├── src/mount/mod.rs                    # US1: the Mount trait, AxisInfo, AxisState
├── src/mount/queue.rs                  # US2: the command queue, epoch counter, Stop bypass
├── src/mount/watchdog.rs               # US2: the 1 s heartbeat watchdog, 50 ms tick
├── src/mount/skywatcher/codec.rs       # US3: 24-bit hex, framing, replies
├── src/mount/skywatcher/transport.rs   # US3: echo skip, 300 ms timeout, 5 tries
├── src/mount/skywatcher/protocol.rs    # US3: connect, rates, goto, reverse, !2, stop
├── src/mount/skywatcher/emulator.rs    # US3: the byte-level emulator of the mount
├── src/mount/skywatcher/serial.rs      # US3: opening the device, the only untested shell
├── src/main.rs                         # US3: --probe-mount DEVICE
├── src/frames.rs (013's frame loop)    # US2: axis positions at exposure start and end
└── tests/{stop_and_watchdog,frame_positions,skywatcher}.rs   # US2, US3
crates/scope-link/
├── proto/scope_node/v1/scope_node.proto   # US2: mount calls (adds only)
├── src/auth.rs (013's interceptor)        # US2: stop all exempt from the token
└── tests/mount_calls.rs                   # US2
crates/simulator/
├── src/mount.rs                        # US1: the simulated mount behind the port
└── tests/mount.rs                      # US1
crates/finder/
├── src/ephemeris.rs                    # US4: apparent altitude (Saemundsson, Bennett)
├── src/model.rs                        # US5 (new): syncs, fit, epochs, fixed terms, inverse
├── src/session.rs                      # US5: the night's model state and the fixed terms
├── src/goto.rs                         # US6 (new): planner and state machine; US7: second sync
├── src/api.rs                          # US6: /api/goto, /api/stop; US7: /api/model/*
├── src/live.rs (013's live loop)       # US5: hand solved star frames to the model as syncs
├── README.md                           # US6, US7: the new routes
└── tests/{model_sim,goto_sim,goto_api,second_sync_sim,model_api,night_sim}.rs
crates/finder/web/
├── src/components/GoToControl.tsx (+ test)   # US8
├── src/components/StopButton.tsx (+ test)    # US8 (in TopBar on every page)
├── src/components/MountSection.tsx (+ test)  # US8 (on SetupPage)
├── src/test/fixtures/session-*.json          # US8: sessions recorded from the simulator
├── dist/                                     # US8: rebuilt
└── e2e/goto.spec.ts                          # US9 (013's simulate project)
DESIGN.md, .impeccable/design.json            # US8
```

File names marked with 013 refer to the modules 013 creates. A story reads them before
editing, and keeps to the names 013 actually landed.

## Interfaces

**The mount port** lives in `crates/scope-node/src/mount/mod.rs`:

```rust
trait Mount {
    fn axes(&self) -> [AxisInfo; 2];                    // counts per revolution, top rate, can_goto
    async fn state(&self, axis: Axis) -> AxisState;     // position (counts), rate, moving
    async fn goto(&self, axis: Axis, position: i32);    // absolute, in counts
    async fn rate(&self, axis: Axis, deg_per_s: f64);   // signed
    async fn stop_all(&self);
}
```

- Positions are in the axis's own counts, and the console converts them with counts per
  revolution.
- Rates are signed degrees per second, and each adapter turns them into its own units.

**`scope_node.v1` gains these calls** (adds only):
- `DescribeMount` → two `AxisInfo`;
- `AxisState(axis)`;
- `GotoAxis(axis, position)`;
- `RateAxis(axis, deg_per_s)`;
- `StopAll`.

Only `StopAll` is exempt from the bearer-token interceptor. `StarFrame` gains optional
`axis_start` and `axis_end` (both axes' counts, on the node's clock) on new field numbers.

**`GET /api/session` gains:**
- `goto: {state, correction, error_arcsec, reason}`, present while a GoTo runs or has just
  ended;
- `model: {syncs, epoch, ia, ie, base_tilt: {deg, toward_deg}?, fixed?, rms_arcsec,
  needs}`, where `needs` is the count of further syncs the fixed terms want;
- `mount: {connected, board, firmware, moving}`.

**The routes:**

| Route | Body | Same-origin check | Answers |
|---|---|---|---|
| `POST /api/goto` | none | 012's | 200 when started; 409 with a JSON `error` naming the reason when the target is not pointable, there is no mount, there is no sync, the Sun is up, or the approach would go below the minimum altitude |
| `POST /api/stop` | none | none; any origin is accepted | 200 always, stopping both axes |
| `POST /api/model/second-sync` | none | 012's | starts the second sync |
| `POST /api/model/reset` | `{"what": "night" \| "fixed"}` | 012's | clears that part of the model |

## Shapes that existing tests pin

These are not changed by any story:
- **012's and 013's surfaces:** every test they landed passes unchanged. That covers
  `AppState::new`, `AppState::shared` and `router`, the same-origin check (with
  `/api/stop` added as its only exempt route), the session's existing fields, and 013's
  link, frame and simulator APIs.
- **Geometry:** `sky::alt_az` stays geometric, as `alt_az_matches_meeus_example_13b`
  pins. Refraction lives only in `finder::ephemeris`.
- **The proto:** `scope_node.v1`'s existing calls, messages and field numbers are kept.
  New ones only add, and proto comments hold no `[...]`, because rustdoc denies warnings.
- **Playwright:** the servers and projects are unchanged. The only possible change is the
  `simulate` project's file match, if it names files (owner approval 3).
- **Layering:** `crates/scope-node` still depends on `star-detection` and `scope-link`
  only, so the Pi never links the solver.

## Decisions and the alternatives rejected

**A port small enough that another mount is an adapter.** Positions are in counts and
rates in signed degrees per second. Nothing Sky-Watcher-specific crosses the port.

**The Sky-Watcher adapter, in three layers.**
- **Codec:** pure functions, tested exhaustively.
  - Values are 24-bit little-endian hex, with the 0x800000 offset on positions.
  - Commands are framed as `:` + command + axis + data + CR.
  - Replies are `=data` or `!code`.
- **Transport:** a byte stream, either the serial port or the in-memory emulator.
  - A command is written, and its echo, if any, is skipped, because a TTL cable on a
    single-wire bus hears itself.
  - The reply is read with a 300 ms timeout. Queries, and other commands safe to repeat,
    get up to 5 tries, as INDI does.
- **Protocol:**
  - **Connect:** probe `:e1` at 9600, then at 115200. Then send `:F3`, read `:a`, `:b`,
    `:g` and `:q` (auxiliary-encoder support), and send `:W` (auxiliary encoders off).
  - **Firmware:** below 3.39 is accepted with a warning that smooth variable-rate
    tracking may not work.
  - **Rates:** set as a step period, `:I` = timer frequency × 360 / (°/s) / counts per
    revolution, times the high-speed ratio in fast mode. Fast mode (`:G` codes 0 and 3) is
    used above 128× sidereal, as INDI does.
  - **Goto:** `:G` (goto mode), `:S` (absolute target), `:J`. If `:S` is refused, the
    adapter falls back to `:H` (relative) and `:M` (brake point).
  - **Reversing:** `:K`, then `:f` polled until stopped, then the new direction.
  - **`!2` (motor not stopped):** the same stop-and-wait, then the command once more.
  - **Stop:** `:K` on both axes, a decelerating stop, gentler than `:L` on a 16-inch tube.
    `:L` is sent only if `:K` is not acknowledged.

**Serial: `serialport` with `default-features = false`.** It needs no libudev, and
MPL-2.0 is allowed. A few lines of termios through `rustix` replace it if it does not
build for `aarch64-unknown-linux-musl`. The adapter's interface is the same either way,
and the maintainer's musl cross-build at bring-up settles the question. Opening the device
is the only untested shell.

**The byte-level emulator.** It implements the command set over an axis model the same as
the simulator's. It can be told to:
- answer only at 115200;
- echo like a single wire;
- inject `!2`;
- refuse `:S`;
- withhold `:K`'s acknowledgement;
- report old firmware.

The adapter's tests, and the oracle byte strings, run against it.

**`scope-node --probe-mount DEVICE`.** It runs the first-test sequence (`:e1`, `:a1`,
`:b1`, `:g1`, `:q1010000`, `:j1`) and prints the results, for bring-up. The sequence is a
function over any byte stream, so it is tested on the emulator.

**Stop wins on the node through an epoch counter, not a priority queue.** Mount commands
go through one queue to the adapter's task. Stop bypasses the queue: it raises an epoch
counter, clears the queue, and commands the stop at once. A command carrying an older
epoch is dropped when it reaches the adapter. That gives "carried out before any command
that arrived with or after it" without a race between two queues.

**The watchdog runs on the node's monotonic clock.**
- A 50 ms tick compares now with the last heartbeat.
- At 1 s, it calls `stop_all` and marks the link lost.
- A move then needs a fresh request after the link returns. Nothing resumes on its own.

**The pointing model: rigorous where the terms are large, TPoint where they are small.**
The encoder's zero points are arbitrary, so IA can be anything. The forward model (axis
positions → apparent alt/az) runs in four steps:
1. axis angles from counts;
2. IA and IE as exact offsets;
3. the base tilt (AN, AW) as an exact rotation of the mount's frame;
4. CA, NPAE and TF as TPoint's small-angle terms (§7.2).

The finder camera's rotation on the tube adds the solve's roll as a third equation per
sync, through the parallactic angle (Meeus ch. 14).

**The fit is in-crate Levenberg–Marquardt, not a linear-algebra crate.** It works on the
residuals (ΔA·cos E, ΔE, Δroll), weighted by each solve's RMS. There are at most 8
parameters, and the normal equations are solved by Cholesky, so no dependency is needed.

**The inverse (target → axis positions)** is Newton's method on the forward model from
the uncorrected guess. It converges in 3–4 steps.

**Epochs and nights.**
- Each night has its own base tilt.
- Each zero-point epoch has its own IA and IE.
- The fixed terms are shared and kept.

The fixed terms are fitted from one night's syncs once that night has at least 20 spread
over 180° × 30°, and are refined on later nights that qualify. Syncs are not archived
across nights.

**Syncs.**
- Only star frames from the scope node carry axis positions. A frame uploaded by hand
  updates the pointing but is never a sync.
- A frame counts if each axis's rate during the exposure is at most 0.2°/s. That rejects
  slews (3.4°/s) and keeps tracking, whose azimuth rate reaches about 0.1°/s at the edge
  of 015's 2° zenith circle and is far lower elsewhere.
- The axis positions are interpolated linearly between the frame's start and end
  readings, to mid-exposure.
- Thinning keeps the newest sync when another lies within 2° and less than 2 minutes
  older.

**Slips: 0.5° after the base tilt is fitted, 2.5° before.** Before the base tilt is
fitted, the prediction is off by up to twice the tilt (2° for 1°). A 0.5° threshold would
make the second sync itself open an epoch, and with its own IA and IE the base tilt could
never be fitted. A mount power-cycled mid-night shows up the same way, at its first sync.

**The night's state ends at local noon.** It is the Sun's next upper transit at the site
after the night's first sync, from 012's rise/transit/set times. It is kept across
restarts until then.

**The cable's wind is the telescope's azimuth, not the counts.** It is unwrapped
continuously from the night's first sync, across syncs and epochs. Raw axis counts would
be reset by a power cycle.

**GoTo is a state machine in `finder::goto`:**
`Plan → Slew → Settle → Solve → (Done | Correct → Slew …) | Failed | Halted`.

**Plan.**
1. Take the target's place at the estimated arrival time, using two iterations of the
   slew-time estimate. Add refraction, and invert the model.
2. Choose the azimuth axis value among the ±360° candidates. It must keep the unwrapped
   azimuth within one revolution of the night's first sync, and be the nearest that does.
   That unwinds the cable when needed.
3. Make each axis's last approach from below: overshoot by 0.5°, more than any backlash a
   mount like this has (the simulator's is 0.3°), then approach upward. When the altitude
   overshoot would go under the minimum altitude, the GoTo is refused with that reason.

**The Sun guard on motion.**
- The path is simulated from the axes' rates and acceleration, at 0.5° steps.
- If any point comes within 15° of the Sun, the planner tries three alternatives in turn:
  altitude first; azimuth first; a dogleg through
  `max(current, target, Sun's altitude + 20°)`.
- If none keeps clear, the slew is refused.
- The Sun's altitude is also checked on a 50 ms tick during motion. Reaching −0.83°
  halts.

**Settle and Solve.** Wait until both axes report stopped, plus 0.5 s. The first frame
whose exposure starts after that is solved and taken as a sync.

**Correct.** The new sync refines IA and IE. If the error is 1′ or more, re-plan a small
move, still approaching from below. After 5 corrections the GoTo fails with the last
error.

**Halted.** Stop, a lost link (the console's own 1 s rule, matching the node's watchdog)
or the Sun guard. Each sends `stop_all` and records the reason.

**The one-tap second sync.** It picks the pointable spot nearest the current pointing
whose azimuth is at least 90° from every sync of the night and that lies outside the Sun
guard. It plans and slews to it as a GoTo with no corrections, then solves and takes the
sync.

**The page.**
- **GoTo:** the guidance block gains it, as a primary button when the target is pointable,
  with its progress and result.
- **Stop:** it sits in the top bar of every page, the console and Setup alike, whenever
  the session says the mount may be moving. It is never disabled, never under a sheet, and
  at least 44 px tall.
- **Setup:** it gains the mount and model section.

**The console never moves the mount unasked.** Only GoTo and the second sync move it here
(015 adds Track), so someone at the eyepiece is never surprised. Stop is always on screen
while the mount moves.

**Proof in virtual time, not on hardware.** Every GoTo, sync and model behaviour is a
Rust integration test that runs the simulator and the console in one paused-time tokio
runtime, over the in-process link. A night of slews takes milliseconds of CPU, and the
coverage job counts it. Playwright's `simulate` project adds one GoTo-and-Stop flow.

**Rejected alternatives:**
- the hand controller's protocol and SynScan App;
- the mount's own star alignment and GoTo (ADR-0001);
- `:L` as the normal stop, which is abrupt on a 16-inch tube;
- a linear-algebra crate for an 8-parameter fit;
- archiving syncs across nights;
- measuring the cable's wind in raw counts.

## User Story 1 — The mount port and the simulated mount

- **The port:** `crates/scope-node/src/mount/mod.rs` holds the `Mount` trait and its
  types, with unit tests on a fake axis.
- **The simulated mount:** `crates/simulator/src/mount.rs` implements the trait over two
  simulated axes in virtual time:
  - a trapezoidal velocity profile (3°/s², 3.4°/s top rate);
  - backlash as a 0.3° dead band between the axis position and the true angle;
  - a base tilt (1° by default) as a rotation of the mount's frame;
  - zero-point offsets.
- **Test hooks:** push by hand (true direction only, counts unchanged), slip per move,
  power-cycle (counts reset to the power-on zero), and the true eyepiece direction.
- **013's simulated telescope** follows the mount's true direction while a mount is
  connected, and 013's push by hand (`POST /api/sim/telescope`) goes through the hook.

## User Story 2 — Stop, the watchdog, frames and the link

- **`mount/queue.rs`:** the single queue to the adapter's task, with the epoch counter
  and the Stop bypass.
- **`mount/watchdog.rs`:** the 50 ms tick on the node's monotonic clock, which calls
  `stop_all` at 1 s and marks the link lost.
- **The frame loop:** 013's loop reads both axes' positions at exposure start and end,
  onto `StarFrame`.
- **`scope_node.v1`:** gains the mount calls, with 013's token interceptor exempting
  `StopAll` only.
- **The console's link client:** gains the matching calls.

## User Story 3 — The Sky-Watcher adapter

- **The adapter:** codec, transport, protocol and emulator under
  `crates/scope-node/src/mount/skywatcher/`, as above.
- **The emulator:** implements every command the adapter sends, over an axis model.
- **`serial.rs`:** opens the device with `serialport` (or termios), and is the only
  untested shell.
- **`--probe-mount`:** added in `src/main.rs`.
- **The dependency:** `serialport` (`default-features = false`) goes in
  `crates/scope-node/Cargo.toml`, with `Cargo.lock` updated. `cargo-deny` and the MSRV
  build check it in the queue.

## User Story 4 — Apparent altitude

`finder::ephemeris` gains the two directions of refraction for the standard atmosphere
(1010 hPa, 10 °C):
- `apparent_from_true(h)`, by Saemundsson (Meeus 16.4);
- `true_from_apparent(h0)`, by Bennett (Meeus 16.3).

`sky::alt_az` is not touched.

## User Story 5 — The pointing model

**`finder::model`:**
- sync intake (the rate rule, interpolation to mid-exposure, thinning);
- the forward model (four steps);
- the in-crate Levenberg–Marquardt fit with Cholesky;
- the Newton inverse;
- epochs and slips;
- the fixed terms and their spread rule;
- `needs`, the count of further syncs the fixed terms want.

**The live loop** hands each solved star frame with axis positions to the model.
Refraction is removed first.

**`finder::session`** keeps the night's model (syncs, epochs and base tilt) until the next
local noon after the first sync, across restarts, and keeps the fixed terms until reset.

## User Story 6 — GoTo

**`finder::goto`:**
- the planner: arrival-time iteration, refraction, the model inverse, the
  one-revolution cable rule, the approach from below, the minimum altitude, the Sun-up
  refusal, and the path simulation with its three re-plans;
- the state machine, with settle, solve, at most 5 corrections, and the halts.

**`src/api.rs`:**
- `POST /api/goto`, behind 012's same-origin check, with its 409 reasons;
- `POST /api/stop`, which skips the check and needs no body;
- `goto` progress in `GET /api/session`.

**Documentation:** the routes are documented in the crate docs and README.

## User Story 7 — The second sync and the model routes

**The second sync:**
- the spot choice (pointable, at least 90° in azimuth from every sync of the night,
  outside the Sun guard);
- a GoTo with no corrections;
- then the sync.

**The model routes:** `POST /api/model/second-sync` and `POST /api/model/reset`, behind
012's same-origin check, documented in the crate docs and README.

## User Story 8 — The page

**New components:**
- `GoToControl` in the guidance block, enabled only when a GoTo would be accepted,
  otherwise showing the reason, with progress and result;
- `StopButton` in `TopBar` on every page whenever `mount.moving` or a GoTo or second
  sync is under way: never disabled, never inside a sheet, at least 44 px;
- `MountSection` on `SetupPage`: link and board, "base tilt X° toward Y°", RMS, sync
  count, the second sync and the two resets.

**Fixtures:** sessions recorded from the simulator (a GoTo to M42, two syncs) go in
`src/test/fixtures/`.

**Design records:** `DESIGN.md` and `.impeccable/design.json` gain the new parts as
built.

**Last step:** rebuild `web/dist`.

## User Story 9 — End to end

**`crates/finder/tests/night_sim.rs`** plays the night in paused time, in this order:
1. the first sync, then the second sync;
2. five GoTos;
3. a hand push, then a GoTo;
4. a Stop mid-slew;
5. a dropped link mid-slew.

**`e2e/goto.spec.ts`**, in 013's `simulate` project, covers:
- a GoTo from the page;
- Stop from the page;
- the second sync from Setup;
- Stop's size and that nothing covers it;
- the night pixel check with GoTo running.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check` and
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked` and
    `cargo test --workspace --locked --doc`.
- **Rust stories** (US1–US7) also run
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`.
- **The page story** (US8), in `crates/finder/web`: `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`. Then `npm run build`, and confirm that
  `git status` shows `dist/` matching the rebuild (CI's freshness check).
- **US9** also runs `npm run e2e`.
- **CI in the merge queue** runs all of the above and more: coverage at 80%, the cedar
  e2e eval and canary, msrv, docs, deny, machete and the secret scan. A story is not done
  until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **The owner approvals are given.** They are listed in spec § Owner approvals, and
   flipping to `ready` records them.
2. **The repository is wired for the Ergane merge queue.** Run `ergane init --wire`, and
   `ergane init --check` must be all green.

No story adds a file large enough to need staging by the operator. The session fixtures
and test vectors are small. `depends_on_landed` holds dispatch until 012 and 013 have
landed.

## Risks

| Risk | Mitigation |
|---|---|
| The cable into the mount's jack. Sky-Watcher's own USB adapter may enumerate as HID rather than serial, and RS-232 levels damage the jack. | Use a plain USB–TTL serial cable (FTDI, CP210x, CH340 or PL2303, all in the Pi's kernel) at TTL levels. Confirm with `--probe-mount` before any move (maintainer, at bring-up). |
| Motor firmware below 3.39 may track unevenly (015). | Warn at connect, and record the version at bring-up. |
| 1′ needs the aim point accurate to well under 1′. At 30″ a pixel, 1′ is 2 px. | Centre the calibration star at high power. A wider field on the real finder camera makes 1′ harder; bring-up records the scale, and the tolerance is revisited then if needed. |
| The mechanical limits: no published altitude range, the rocker and ground board limit low altitudes, and there is no azimuth hard stop, so a power cable to the ground can wrap. | GoTo targets are pointable (above the minimum altitude). The one-revolution rule unwinds the cable. The adapter reports any range limits it learns at bring-up. |
| Someone at the eyepiece when the tube moves. | The console never moves the mount unasked: only GoTo and the second sync move it here (015 adds Track). Stop is always on screen while it moves. |
| Freedom Find off means a push by hand is not counted. | Slips are detected at the next sync, and the GoTo still converges (US6 "pushed by hand"). |
| `serialport` may not build for musl. | Termios through `rustix` behind the same interface. The maintainer's cross-build at bring-up settles it. |
