# Implementation Plan: Tracking

**Branch**: `015-tracking` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Make the console follow the target with the mount's motors:
- feed-forward axis rates from the pointing model, sent every second;
- a proportional-integral correction from the finder camera's solves;
- backlash measured at the first reversal and taken up before later ones;
- a wait at the exit of the 2° zenith circle;
- a safe ending on Stop, a lost link, sunrise or a target too low.

Add the alt-az rates, the field rotation and an imaging camera's longest exposure to
`finder::ephemeris`. Give the simulator a rate error, a backlash setting and a truth hook.
Expose Track and the imaging camera through the session and on the page.

Everything is proven on 014's simulated mount in paused virtual time. `crates/math-core`
is untouched. No existing test changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- already in the workspace: tokio (paused time), axum and serde;
- the simulator (`crates/simulator`, from 013 and 014);
- the page: Vite, Vitest and Playwright.

No new crate and no new npm package is expected. The PI controller, the forecast and the
backlash logic are a few hundred lines of plain Rust. A new crate would be a dependency
change on a critical path: it must build on 1.96, pass `cargo-deny` and `cargo-machete`,
and be justified in its story.

**Storage**: in the session's state file, held by the binary only:
- the measured backlash per axis, beside 014's fixed terms;
- the imaging camera.

Tracking itself is never persisted: it is off at every start.

**Testing**:
- Rust integration tests in new files under `crates/finder/tests/` and
  `crates/simulator/tests/`. The console and the simulator run in one paused-time tokio
  runtime over the in-process link, as 014's GoTo tests do.
- Vitest and Playwright in `crates/finder/web`.

**Target Platform**: the `finder` binary on desktop Linux, WSL and Raspberry Pi, driving
014's scope node and mount. The page runs on Android phones and laptops.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the 80% coverage
  floor, which counts Rust only.
- Long runs must stay cheap: an hour of tracking in under 30 s of wall time.
- `web/dist` is committed and must be the build of its source.
- The night mode is red only.

**Scale/Scope**: one mount, one target at a time, rate updates at 1 Hz, and solves at
0.2–2 Hz.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing here touches the solver or `crates/math-core`. The rates are checked against hand-computed values at φ 40°, δ 20° (Meeus ch. 13 and 14), and tracking against the simulator's own truth. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The 1% rate-error scenario and the Playwright file are owner approvals (spec § Owner approvals). |
| III. Tests first, independent expectations | Every scenario names its oracle: a worked value at φ 40°, δ 20°, the simulator's true eyepiece direction, its configured backlash and zero points, or a fixed virtual clock. |
| IV. Critical paths | `src/api.rs` gains one route on the HTTP surface, not `proto/`. No `unsafe`, no database-format change, and no dependency change is planned. The scope node and the link are untouched. |
| V. Standalone Rust, honest licensing | No new data or third-party code. |

## Project Structure

```text
crates/finder/
├── src/
│   ├── ephemeris.rs          # US1: alt-az rates, field rotation, longest exposure
│   ├── lib.rs                # US3: `pub mod tracking;`
│   ├── tracking.rs           # US3 (new): feed-forward, ending; US4: PI;
│   │                         # US5: backlash measure and take-up; US6: keyhole
│   ├── model.rs              # US2: test hook to load known terms;
│   │                         # US5: syncs against the approach direction
│   ├── session.rs            # US5: backlash kept with the fixed terms, cleared on reset;
│   │                         # US7: Track state, imaging camera, readout
│   └── api.rs                # US7: POST /api/track; imaging_camera in PUT /api/session
├── tests/
│   ├── rates.rs              # US1 (new)
│   ├── sim_truth.rs          # US2 (new)
│   ├── tracking_sim.rs       # US3 (new)
│   ├── tracking_loop.rs      # US4 (new)
│   ├── backlash_sim.rs       # US5 (new)
│   ├── backlash_cli.rs       # US5 (new)
│   ├── keyhole_sim.rs        # US6 (new)
│   ├── track_api.rs          # US7 (new)
│   ├── track_cli.rs          # US7 (new)
│   └── tracking_night.rs     # US9 (new)
└── web/
    ├── src/components/TrackControl.tsx (+ test)          # US8 (in the guidance block)
    ├── src/components/ImagingCameraSection.tsx (+ test)  # US8 (on SetupPage)
    ├── src/components/GuidanceBlock.tsx, SetupPage.tsx   # US8: mount the two parts
    ├── dist/                                             # US8: rebuilt
    ├── e2e/tracking.spec.ts                              # US9 (new, `simulate` project)
    └── playwright.config.ts                              # US9: at most one `simulate` entry
crates/simulator/
├── src/mount.rs              # US2: rate error and backlash settings, truth accessor
│                             # (or the module where 014 placed the simulated mount)
└── tests/rate_and_backlash.rs  # US2 (new)
DESIGN.md, .impeccable/design.json                         # US8
```

The simulated mount's module and the pointing model's module are 014's. Each story
edits the file where 014 put that code, and names it in its PR.

## Interfaces

**`finder::ephemeris`** (US1) adds:

```text
altaz_rates(lat_deg, dec_deg, hour_angle_deg) -> AltAzRates {
    alt_arcsec_per_s, az_arcsec_per_s, field_rotation_deg_per_h }
longest_exposure(sensor_w_mm, sensor_h_mm, pitch_um,
                 field_rotation_rad_per_s, smear_px) -> Exposure { Seconds(f64) | NoLimit }
```

A form taking altitude and azimuth is used for the "Due east" case. The corner radius is
public for its test.

**`finder::tracking`** (US3) holds a `Tracker` driven by the console's clock:
- `start(target)`, `stop(reason)` and `state()`;
- a pointing entry point that takes the error between the target and the aim point. A
  solve feeds it after the solver, and a test feeds it truth plus noise.

`state()` reports:
- the state: `off`, `armed`, `tracking` or `keyhole_wait`;
- `ended_reason`: `stopped`, `link_lost`, `sun_up`, `too_low`, or the GoTo's reason;
- the error samples for the RMS;
- the tracking offset per axis.

**`POST /api/track`** (US7) takes `{"on": true | false}` and answers 200 with the session.
- It answers 409 with `{"error": <reason>}` when there is no mount connected, no sync
  tonight, the Sun is up, or the target is not pointable.
- It is subject to the same-origin rule.

**The session** gains three things:
- `track: {wanted, state, target?, ended_reason?, readout?: {rms_arcsec,
  field_rotation_deg_per_min, longest_exposure_s | "no limit"}}`;
- `imaging_camera: {width_mm, height_mm, pitch_um} | null`, settable through
  `PUT /api/session` with the same checked-edit rules (each value more than 0, at most
  100 mm or 50 µm);
- `backlash: {alt_deg | null, az_deg | null}`, beside the fixed terms.

**The simulator** (US2) gains `rate_error` (a fraction, default 0) and a per-axis
`backlash_deg` (default 0.3). It also gains a truth accessor (base tilt and zero points)
that a test can hand to the pointing model's load hook.

## Shapes that existing tests pin

These are not changed by any story:
- **012–014 tests:** every Rust and Vitest test, and every Playwright test in the default,
  `session` and `simulate` projects.
- **The simulator's defaults:** a rate error of 0 and backlash of 0.3° per axis are the
  defaults, so 014's simulator, GoTo and model tests stand unchanged.
- **014's mount path:** GoTo's approach-from-below, the slip thresholds (0.5° once the
  base tilt is fitted, 2.5° before) and the rule that every solve is a sync.
- **Geometry:** `sky::alt_az` stays geometric (the Meeus 13.b test). Tracking's ending
  judges geometric altitude, as "pointable" does.
- **Stop:** `POST /api/stop` takes no body and skips the same-origin check. It wins over
  tracking.

## Decisions and the alternatives rejected

**The rates.** All are checked by hand at φ 40°, δ 20°:
- dh/dt = −ω cos φ cos δ sin H / cos h;
- dA/dt = ω (sin φ − sin δ sin h) / cos² h;
- dq/dt = −ω cos φ cos A / cos h, with A measured from north through east.

Meeus's own formula uses azimuth from the south, hence the sign. dq/dt is +33.69°/h at
φ 40°, δ 20°, transit. These rates serve the readout, the keyhole forecast and the tests'
oracles, not the feed-forward.

**Feed-forward from the model, sent every second.** Each second, the tracker:
1. takes the target's apparent place now and 1 s ahead (014's refraction);
2. inverts the model for both, to axis positions;
3. sends each axis the rate that covers the difference in that second, plus the
   correction.

That carries every model term (base tilt, CA, NPAE, TF) into the rates.
- *Rejected:* plain dh/dt and dA/dt, which miss the model's terms.

**The closed loop: PI on the error, resolved into the axes.** Following Wallace (2002):
1. rotate the error by the parallactic angle into cross-elevation and elevation;
2. divide the azimuth part by cos E.

The control constants:
- a proportional gain of 0.6 on each axis, applied as a rate over the next interval;
- a slow integral, time constant about 60 s, held as a tracking offset per axis and
  cleared when tracking ends;
- errors under 10″ ignored. Inside the deadband the integral keeps applying and is not
  updated.
- the integral clamped to 5% of the sidereal rate. That is well above the 1% error the
  loop is designed for, and low enough that a fault cannot drive the axes away.

The integral is kept apart from IA and IE because every tracking solve is also a sync, and
the zero-point refit would undo anything folded into them. The gains are fixed constants
in code, tuned on the simulator, not settings. The on-sky checklist confirms or retunes
them.
- *Rejected:* a model refit only, with no PI. Refits need spread syncs, and tracking gives
  syncs along one track.

**Backlash at reversals.** The altitude axis reverses at every transit. The azimuth axis
reverses at greatest elongation for any target with δ > φ, where sin H = sin φ / sin δ;
at φ 40° that includes M81, M82, M101 and Capella.
- **Measuring.** At an axis's first reversal in the mount's life, the closed loop sees
  the error jump along that axis. The size of the jump, settled over the next solves, is
  that axis's backlash. Later reversals refine it.
- **Keeping.** The measurement belongs to the mount, so it is kept with the fixed terms
  and cleared when they are reset.
- **Syncs.** A GoTo's last approach is from below, so its syncs carry the backlash on the
  upward side. A sync taken while an axis last moved the other way, such as tracking down
  after transit, has that axis's measured backlash applied before fitting. It is held
  back until a backlash is measured. Otherwise up to 0.3° would bias IE without reaching
  the 0.5° slip threshold.
- **Taking it up.** Before a planned reversal of either axis, the tracker:
  1. stops the axis;
  2. drives it through the measured backlash at 0.5°/s;
  3. resumes the feed-forward rate.

  A reversal is planned when the axis's feed-forward rate changes sign within the
  30-minute forecast. Until a backlash is measured, the closed loop recovers within about
  15 s, relying on 014's stop-before-reverse.

**The zenith keyhole: wait at the exit.** The tracker forecasts the target's path for the
next 30 minutes. When the path enters the 2° circle round the zenith, the tracker:
1. stops at the entry;
2. slews ahead, through 014's planner with the Sun guard included, to where the path
   leaves the circle;
3. waits;
4. once the target's forecast position is outside the circle, solves, applies the error
   and resumes.

A target 1° from the zenith spends about 18 minutes in the 2° circle. The azimuth rate
there is only about 0.1°/s at 2°, but it changes fast, and the plan's rule keeps the loop
out of that region.
- *Rejected:* chasing the target through the circle.

**Ending.** The tracker checks four conditions on the same 50 ms tick that 014 uses for the
Sun during a GoTo, so the scenarios' 1 s bounds have margin:
- Stop;
- a lost link (013's 1 s rule; the node's watchdog also stops the axes);
- the Sun's centre above −0.83°;
- the target's geometric altitude below the minimum altitude.

Any of them stops both axes and records why. The session reports the reason, and the page
shows it.

**Track and GoTo together.** `POST /api/track {"on": true}` arms tracking for the
session's target:
- if a GoTo is running, tracking starts when it completes;
- if the target is more than 1′ from the aim point and no GoTo is running, a GoTo starts
  first;
- `{"on": false}` stops tracking and the axes, leaving the telescope where it is.

**The readout.**
- The RMS error is taken over the last 60 s.
- The field rotation at the target comes from dq/dt, in °/min.
- With an imaging camera set, the longest exposure is t = 1 / (r · |dq/dt|), for 1 px of
  smear at the sensor's corner. Here r is the half-diagonal in pixels and dq/dt is in
  radians per second.
- "No limit" applies when |dq/dt| is below 1e-9 rad/s.

**The rate-error scenario (owner approval 1).** The earlier plan's "0.5% drifts more than
1′ in 10 min" is false: 0.5% of the 8,985″ that M42 moves in 10 minutes is about 45″. The
scenario uses 1% (about 90″).

**Tests: truth plus noise for long runs.** Rendering and solving a frame every 2 s for an
hour of virtual time would dominate the coverage job. Long tests instead feed the tracker
the simulator's true eyepiece direction plus Gaussian noise (σ 3″, seeded) as its
pointings, through the same entry point a solve uses after the solver. One 2-minute run
on rendered, solved frames proves that the two paths agree. The oracles are the
simulator's truth.
- *Rejected:* rendering every frame of the long runs.

**Out of scope.** Field derotation (the readout states the limit instead), guiding from
the imaging camera, and objects that move against the stars.

## User Story 1 — Alt-az rates and the longest exposure

Add `altaz_rates` and `longest_exposure`, with its corner radius, to
`crates/finder/src/ephemeris.rs`. Both take explicit inputs. The tests go in the new
`crates/finder/tests/rates.rs`, using the hand-checked values at φ 40°, δ 20°.

## User Story 2 — The simulator's settings and truth

In the simulated mount, which 014 placed in `crates/simulator`:
- add `rate_error`, applied to every commanded rate;
- add the per-axis `backlash_deg`, replacing the fixed 0.3° with a setting of the same
  default;
- add a truth accessor for the base tilt and zero points.

In the pointing model (`crates/finder/src/model.rs`), add a public hook that loads known
base-tilt and zero-point terms, documented as being for tests and the simulator.

## User Story 3 — Feed-forward and ending

Add `crates/finder/src/tracking.rs`, with the `Tracker`, the 1 Hz feed-forward through the
model's inverse, and the 50 ms ending checks. Each end stops both axes through the mount
port and records the reason. A lost link uses 013's 1 s rule.

## User Story 4 — The closed loop

In `crates/finder/src/tracking.rs`, add the PI correction:
- the parallactic rotation and the division by cos E;
- the gain of 0.6, the 60 s integral as the tracking offset, the 10″ deadband and the 5%
  clamp;
- the offset cleared on every end.

Add the truth-plus-noise pointing feed (seeded σ 3″) in `crates/finder/tests/tracking_loop.rs`,
and the 2-minute rendered-frame run beside it.

## User Story 5 — Backlash

In `crates/finder/src/tracking.rs`:
- add the 30-minute reversal forecast from the feed-forward rate's sign;
- measure the backlash from the closed loop's settled jump at the first reversal;
- take it up at 0.5°/s before later reversals.

In `crates/finder/src/model.rs`, apply or hold back syncs taken against the approach
direction. In `crates/finder/src/session.rs`, keep the backlash with the fixed terms, and
clear it on 014's fixed-terms reset.

## User Story 6 — The zenith keyhole

In `crates/finder/src/tracking.rs`, add the 30-minute path forecast against the 2° circle.
At the circle's entry, stop, slew to the exit through 014's planner, wait, then solve and
resume. A Sun-guard refusal from the planner ends tracking with that reason.

## User Story 7 — Track and the imaging camera in the session

In `crates/finder/src/api.rs`:
- add `POST /api/track`, with its 409 reasons and the same-origin rule;
- add `imaging_camera` to `PUT /api/session`'s checked edits.

In `crates/finder/src/session.rs`:
- hold the Track state, and arm the tracker after a running GoTo or start a GoTo first;
- compute the readout from the tracker's samples and US1's functions;
- persist the imaging camera, and never persist tracking.

## User Story 8 — The page

**`TrackControl`.** It goes in the guidance block. It shows a Switch per DESIGN.md, and
only with a mount connected. It reads out the error, field rotation and longest exposure,
rounded to 1″, 0.01°/min and 0.1 s or "no limit", or why tracking ended.

**`ImagingCameraSection`.** It goes on the Setup page, using DESIGN.md's inputs. It puts
`imaging_camera`, and refuses values out of range with the allowed range.

Record both in `DESIGN.md` and `.impeccable/design.json`. Rebuild `web/dist`.

## User Story 9 — End to end

**`crates/finder/tests/tracking_night.rs`.** An hour of paused time on the simulator:
- a 1% rate error and 0.3° of backlash;
- a target chosen to transit, pass within 2° of the zenith and then set through the
  minimum altitude within the hour;
- the test asserts its own `Instant::elapsed` is under 30 s.

**`crates/finder/web/e2e/tracking.spec.ts`.** It goes in the `simulate` project and covers
two things:
- "Tracking M42", with the readout matching the session as rounded;
- the night pixel check with tracking on.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check` and
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked` and
    `cargo test --workspace --locked --doc`.
- **Rust stories** (US1–US7, US9) also run
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`.
- **The page story** (US8), in `crates/finder/web`: `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`. Then `npm run build`, and confirm that `dist/`
  matches the rebuild (CI's freshness check).
- **US9** also runs `npm run e2e`.
- **CI in the merge queue** runs all of the above and more: coverage at 80%, the cedar
  e2e eval and canary, msrv, docs, deny, machete and the secret scan. A story is not done
  until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **012-targets-and-push-to, 013-live-finder and 014-goto have landed** (frontmatter
   `depends_on_landed`).
2. **The owner approvals are given** (spec § Owner approvals). Flipping to `ready` records
   them.
3. **The repository is wired for the Ergane merge queue.** `ergane init --check` must be
   all green.

No operator-staged data is needed.

## Risks

| Risk | Mitigation |
|---|---|
| Motor firmware below 3.39 reportedly tracks unevenly at variable rates. | 014 warns at connect. The closed loop absorbs small unevenness, and bring-up records the firmware. |
| Rate updates every second go over a 9600-baud single-wire link. | Each update is a few short commands per axis, well under the link's capacity. It is measured at bring-up, outside the factory. |
| The altitude axis reverses at transit. | Stop-before-reverse costs a fraction of a second, and backlash far more. The first reversal measures the backlash and later ones take it up ahead of time, so the error stays within the 30″ the scenario allows. A real mount's backlash may vary along the axis; the closed loop absorbs what the measurement misses. |
| Imaging at 1800 mm on an alt-az mount needs short subs and derotation when stacking. | The readout states the limit. Derotation is out of scope. |
| A rate error far beyond 1% (a slipping clutch, a wrong counts-per-turn). | 1% is the closed loop's design case, and the integral is clamped at 5%. Larger errors point to a fault that the model and the slip rule catch at the next sync. |
| The simulator's PI gains do not suit the real mount. | They are the starting point. The on-sky checklist confirms or retunes them as constants in code. |
