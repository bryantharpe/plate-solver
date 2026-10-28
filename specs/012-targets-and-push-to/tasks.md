---
description: "Task list for 012-targets-and-push-to: Targets and Push-To"
---

# Tasks: Targets and Push-To

**Prerequisites** (plan.md § Prerequisites):
- `crates/finder/data/openngc.csv`, its `data/README.md` recipe and its
  `THIRD_PARTY_NOTICES.md` row are on `main`;
- the owner approvals in spec.md are given;
- the repository is wired for the Ergane merge queue.

**Rules for every story:**
- Never edit an existing test, golden, `scripts/ci/`, `.github/` or `CODEOWNERS`.
- New Rust tests go in new files under `crates/finder/tests/`, or in-module.
- `crates/math-core` is never touched.

## Phase 1: User Story 1 — the Sun and rising, transit and setting (Priority: P1)

### Tests first

- [ ] T001 [US1] In a new `crates/finder/src/ephemeris.rs`, write failing in-module tests:
  - Meeus 25.a (US1-S1);
  - Meeus 15.a Venus at Boston (US1-S2);
  - always above at Dec +80° (US1-S3);
  - never rises at Dec −60° (US1-S4);
  - a rising that `sky::alt_az` confirms at 15° ±0.05°, climbing (US1-S5);
  - M8's next rising, transit and setting from 2026-01-16 02:00 UT at 40° N, 75° W
    (US1-S6).

### Implementation

- [ ] T002 [US1] Add `pub mod ephemeris;` to `crates/finder/src/lib.rs`.
- [ ] T003 [US1] Implement the Sun's apparent RA and Dec (Meeus ch. 25 low-accuracy
  series) in `crates/finder/src/ephemeris.rs`.
- [ ] T004 [US1] Implement the date-bound rising, transit and setting of an altitude
  (Meeus ch. 15) in `crates/finder/src/ephemeris.rs`:
  - m0, m1 and m2 with three corrections each;
  - mean sidereal time from `sky::gmst_deg`;
  - interpolation over three daily positions;
  - `always_above` and `never_rises` with the transit still given.
- [ ] T005 [US1] Implement the next rising, then the transit and setting after it, over
  today's and tomorrow's times within 24 sidereal hours, in
  `crates/finder/src/ephemeris.rs`. Document the public items so that `cargo doc` with
  `-D warnings` builds.
- [ ] T006 [US1] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm `crates/math-core` and `sky.rs`'s existing tests are untouched (US1-S7). Commit
  the green story (FR-001–FR-003; US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6, US1-S7).

## Phase 2: User Story 2 — the target catalog and its search (Priority: P1)

### Tests first

- [ ] T007 [US2] Write failing in-module tests in a new `crates/finder/src/targets.rs`:
  - the object count equals the count recorded in `crates/finder/data/README.md`
    (US2-S1);
  - every `Dup` id finds the object it duplicates (US2-S2);
  - all 110 Messier objects agree with `catalog::messier()` within 1″ (US2-S4);
  - M102, M76 and M45, and `M111` absent (US2-S5);
  - five ways to write NGC 224 (US2-S6);
  - ids that are not there (US2-S7).
- [ ] T008 [US2] Write failing tests in a new `crates/finder/tests/targets_api.rs` for
  `GET /api/targets` through `router(AppState::new(..))`:
  - three ways to Andromeda (US2-S8);
  - `qqq` answers an empty list (US2-S9);
  - a missing or empty `q` answers 400 (US2-S10);
  - the default and clamped `limit` (US2-S11);
  - a search from an empty working directory (US2-S3).
- [ ] T009 [US2] Write a failing test in a new `crates/finder/tests/notices.rs`: the
  OpenNGC row in `crates/finder/THIRD_PARTY_NOTICES.md` names CC BY-SA 4.0 and the commit
  that `crates/finder/data/README.md` pins (US2-S12).

### Implementation

- [ ] T010 [US2] Add `pub mod targets;` to `crates/finder/src/lib.rs`.
- [ ] T011 [US2] In `crates/finder/src/targets.rs`:
  - parse `include_str!("../data/openngc.csv")` once into a `OnceLock`;
  - give each object its canonical id and make `Dup` rows aliases;
  - normalise lookups for case, a space before the number, and leading zeros.
- [ ] T012 [US2] Implement the search ranking in `crates/finder/src/targets.rs`: exact ids
  first, then common-name word prefixes, brightest first, with no magnitude last.
- [ ] T013 [US2] Add the `GET /api/targets?q=&limit=` route in `crates/finder/src/api.rs`,
  with the console's `{"error": ...}` body for a 400.
- [ ] T014 [US2] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Commit the green story (FR-004–FR-008; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6,
  US2-S7, US2-S8, US2-S9, US2-S10, US2-S11, US2-S12).

## Phase 3: User Story 3 — one observing session, kept across restarts (Priority: P1)

### Tests first

- [ ] T015 [US3] Write failing in-module tests in a new `crates/finder/src/session.rs`:
  - a fresh session has no site, no target and a 15° minimum (US3-S2);
  - a `FixedClock` reads 1768528800 twice (US3-S11).
- [ ] T016 [US3] Write failing tests in a new `crates/finder/tests/session_api.rs`
  through the router:
  - one device writes and another reads (US3-S1);
  - out-of-range values and unknown fields answer 400 (US3-S3);
  - an unknown target answers 404 (US3-S4);
  - one field at a time (US3-S5);
  - clearing a field with `null` (US3-S6);
  - the library writes nothing under an empty `HOME` and `XDG_STATE_HOME` (US3-S10).
- [ ] T017 [US3] Write failing tests in a new `crates/finder/tests/session_cli.rs`,
  running the built `finder` binary:
  - a restart on one `--state` folder (US3-S7);
  - a `not json` state file becomes `.bad`, with a line on stderr (US3-S8);
  - the XDG default location (US3-S9);
  - the binary's time is within 2 s of the system clock (US3-S12).

### Implementation

- [ ] T018 [US3] Add `pub mod session;` to `crates/finder/src/lib.rs`. In
  `crates/finder/src/session.rs`, implement:
  - `Session`: site, minimum altitude and target behind a mutex, with an optional state
    file;
  - the `Clock` trait, `SystemClock` and `FixedClock`;
  - checked partial edits.
- [ ] T019 [US3] In `crates/finder/src/api.rs`, add three things:
  - `AppState::with_session` and `AppState::with_clock`, keeping `new`, `shared` and
    `router` as they are;
  - `GET /api/session`;
  - `PUT /api/session` with `deny_unknown_fields`, target validation against
    `finder::targets`, and the console's error body.
- [ ] T020 [US3] In `crates/finder/src/session.rs`, implement the state file:
  - an atomic write (`session.json.tmp`, synced, then renamed);
  - loading on start;
  - `.bad` handling with a line on stderr.

  Add `--state` with the XDG default to `crates/finder/src/main.rs`. Add no option that
  builds a fixed clock.
- [ ] T021 [US3] Document `GET`/`PUT /api/session` and `--state` in the crate docs and in
  `crates/finder/README.md` (US3-S14).
- [ ] T022 [US3] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm `tests/api.rs` is unchanged and the shapes are kept (US3-S13). Commit the green
  story (FR-009–FR-013; US3-S1 to US3-S14).

## Phase 4: User Story 4 — same-origin writes and the latest pointing (Priority: P1)

### Tests first

- [ ] T023 [US4] Write failing tests in a new `crates/finder/tests/session_writes.rs`:
  - a foreign `Origin` on `PUT /api/session` and on `POST /api/solve` answers 403
    (US4-S1);
  - the console's own `Origin`, or none, is applied (US4-S2);
  - `text/plain` answers 415 (US4-S3);
  - a rebound `Host` answers 403 (US4-S4);
  - an `--allow-host` name is applied (US4-S5).
- [ ] T024 [US4] Write failing tests in a new `crates/finder/tests/pointing_api.rs`,
  solving `orion2.jpg` and `roof.jpg` under a `FixedClock`:
  - orion2's pointing and its alt/az against `sky::alt_az` (US4-S6);
  - a frame that does not solve leaves the pointing (US4-S7);
  - no site gives no alt/az (US4-S8);
  - a site set after the solve (US4-S9);
  - a site cleared (US4-S10);
  - `pointing_seq` on the report (US4-S11).

### Implementation

- [ ] T025 [US4] In `crates/finder/src/api.rs`, add the Origin and Host check to
  `PUT /api/session` and `POST /api/solve`:
  - a foreign `Origin` is refused 403;
  - an absent `Origin` is allowed;
  - a present `Origin` needs an IP-literal, `localhost` or `--allow-host` `Host`.

  Add `--allow-host` (repeatable) to `crates/finder/src/main.rs`.
- [ ] T026 [US4] In `crates/finder/src/solve.rs`, add a sibling of `solve_frame` that
  also returns the solved `Frame`; `solve_frame` keeps its signature and calls it. Add the
  optional `pointing_seq` to the report.
- [ ] T027 [US4] Record the latest pointing after each frame that solves, in
  `crates/finder/src/session.rs` and `crates/finder/src/api.rs`:
  - `seq`;
  - the `Frame`;
  - the J2000 centre;
  - the clock's time;
  - alt/az when a site is set.

  Recompute alt/az when the site changes, and drop them when it is cleared.
- [ ] T028 [US4] Document `--allow-host` and the same-origin rule in the crate docs and in
  `crates/finder/README.md`.
- [ ] T029 [US4] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm `tests/api.rs` is unchanged (US4-S12). Commit the green story
  (FR-014–FR-017; US4-S1 to US4-S12).

## Phase 5: User Story 5 — pointable, and how to push there (Priority: P1)

### Tests first

- [ ] T030 [US5] Write failing in-module tests in a new `crates/finder/src/pushto.rs`:
  - the January-evening classes of M42, M8 and M75 (US5-S1);
  - a 50° minimum (US5-S2);
  - no site (US5-S3);
  - M8 at a 15° minimum (US5-S4);
  - M8 at a 30° minimum (US5-S5);
  - across north (US5-S6);
  - the other way (US5-S7);
  - an exact half-turn (US5-S8);
  - not pointable (US5-S9).
- [ ] T031 [US5] Write failing tests in a new `crates/finder/tests/pushto_api.rs`, with a
  `FixedClock` and orion2 solved:
  - an hour later (US5-S10);
  - ten minutes later (US5-S11);
  - classes in a search (US5-S12);
  - M78 placed on the frame (US5-S13);
  - M79 off the frame (US5-S14);
  - `/api/similar` ranked for a 50° minimum (US5-S15);
  - `guidance` in `GET /api/session` (US5-S17).

### Implementation

- [ ] T032 [US5] Add `pub mod pushto;` to `crates/finder/src/lib.rs`. In
  `crates/finder/src/pushto.rs`, implement two things:
  - the class, by precedence: the Sun guard via `ephemeris` and
    `sky::precess_from_j2000`, then below the horizon, then too low, then pointable, with
    `no_site`;
  - for a target that is down, the next rising, transit and setting at the minimum
    altitude.
- [ ] T033 [US5] Implement turn and tilt in `crates/finder/src/pushto.rs`. The wrap is
  `((Δaz + 180) mod 360) − 180`, with −180 mapped to +180. They run from the pointing's
  held alt/az to the target's at the console's time.
- [ ] T034 [US5] Implement placement in `crates/finder/src/pushto.rs`:
  - on the frame, through `object_marks`' code path in `crates/finder/src/solve.rs`;
  - off the frame, as direction and distance from the frame's centre.
- [ ] T035 [US5] Add the following in `crates/finder/src/session.rs` and
  `crates/finder/src/api.rs`:
  - `guidance` derived on each read of `GET /api/session`;
  - `class` on each `/api/targets` result, in `crates/finder/src/targets.rs`.
- [ ] T036 [US5] Add `similar_above()` in `crates/finder/src/catalog.rs`, with `similar()`
  calling it with 15°. Pass the session's minimum altitude from `/api/similar` in
  `crates/finder/src/api.rs` (US5-S16).
- [ ] T037 [US5] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm `similar()`'s unit tests and `similar_objects_rank_for_an_observer` are
  unchanged. Commit the green story (FR-018–FR-026; US5-S1 to US5-S17).

## Phase 6: User Story 6 — the page shares the session; the Setup page (Priority: P2)

### Tests first

- [ ] T038 [US6] Write failing Vitest tests in a new
  `crates/finder/web/src/useSession.test.ts`:
  - polling every 5 s while visible, and stopping while hidden (US6-S1);
  - `put` sends JSON and refetches (US6-S1);
  - a stored browser site is adopted once (US6-S2);
  - App.test's mock reads as an empty session (US6-S3).
- [ ] T039 [US6] Write failing Vitest tests in a new
  `crates/finder/web/src/components/SetupPage.test.tsx`:
  - raising the minimum altitude to 30° (US6-S4);
  - 90° refused with "0–80°" (US6-S5);
  - latitude 95 refused (US6-S6);
  - the top-bar route to `/setup` and back (US6-S8).

### Implementation

- [ ] T040 [US6] Add `crates/finder/web/src/useSession.ts`, with polling while visible,
  `put(edit)`, and missing fields read as unset. Add the once-only adoption of a
  `useSite()` site in `crates/finder/web/src/App.tsx`.
- [ ] T041 [US6] Add `crates/finder/web/src/components/SetupPage.tsx` (site and minimum
  altitude, field atlas design), routing on `location.pathname` and `history.pushState` in
  `crates/finder/web/src/App.tsx`, and a Setup link in
  `crates/finder/web/src/components/TopBar.tsx`.
- [ ] T042 [US6] Add the optional `minAlt` prop (default 15) to
  `crates/finder/web/src/components/NextTargets.tsx`, passed from the session in `App.tsx`
  (US6-S7).
- [ ] T043 [US6] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`, then `npm run build`. Commit the rebuilt
  `crates/finder/web/dist` (US6-S9), and run the `lint` and `test` gates. Confirm
  `package.json` dependencies and every existing Vitest file are unchanged. Commit the
  green story (FR-027–FR-029; US6-S1 to US6-S9).

## Phase 7: User Story 7 — target sheet, guidance block, target on the chart (Priority: P2)

### Tests first

- [ ] T044 [US7] Write failing Vitest tests in a new
  `crates/finder/web/src/components/TargetSheet.test.tsx`:
  - Andromeda by name, with its class, picked into the session (US7-S1);
  - nothing matches (US7-S2);
  - clearing the target (US7-S3).
- [ ] T045 [US7] Write failing Vitest tests in a new
  `crates/finder/web/src/components/GuidanceBlock.test.tsx`:
  - directions to M78 (US7-S4);
  - M8 down, with local times to the minute (US7-S5);
  - near the Sun (US7-S6);
  - no site, linking to `/setup` (US7-S7);
  - changed on another device within 5 s (US7-S8).
- [ ] T046 [US7] Write failing Vitest tests in a new
  `crates/finder/web/src/components/TargetMark.test.tsx`:
  - a reticle on M78 within 0.5 px, with five unchanged legend chips (US7-S9);
  - an edge arrow toward M79 within 1° (US7-S10);
  - no mark when `pointing_seq` differs (US7-S11).

  Write one more in a new `crates/finder/web/src/App.target.test.tsx`: the reading order
  with a target (US7-S12).

### Implementation

- [ ] T047 [US7] Add `crates/finder/web/src/components/TargetSheet.tsx` on FramePicker's
  dialog pattern, opened from `crates/finder/web/src/components/TopBar.tsx`.
- [ ] T048 [US7] Add `crates/finder/web/src/components/GuidanceBlock.tsx` (ink only,
  device-local times to the minute). Put it, with the title block, first in
  `crates/finder/web/src/App.tsx` when a target is set.
- [ ] T049 [US7] Add `crates/finder/web/src/components/TargetMark.tsx` as a Chart layer
  outside the legend: a reticle round the family symbol, or an edge arrow reusing
  `DirectionArrow`, drawn only when `pointing_seq` matches. Wire it in
  `crates/finder/web/src/components/Chart.tsx` without changing existing layers.
- [ ] T050 [US7] Record the guidance block, target reticle, edge arrow and Setup page as
  built in `DESIGN.md` and `.impeccable/design.json` (US7-S13).
- [ ] T051 [US7] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`, then `npm run build`. Commit the rebuilt
  `crates/finder/web/dist` (US7-S14), and run the `lint` and `test` gates. Confirm
  `Chart.test.tsx`, `App.test.tsx` and `package.json` dependencies are unchanged. Commit
  the green story (FR-030–FR-034; US7-S1 to US7-S14).

## Phase 8: User Story 8 — end to end, on a server of its own (Priority: P3)

### Tests first

- [ ] T052 [US8] Write `crates/finder/web/e2e/session.spec.ts`:
  - the first screen with a target at 412 × 915 and 390 × 844 (US8-S2);
  - set on the phone, seen on the laptop, in two contexts (US8-S3);
  - `/setup` opened directly (US8-S4);
  - the night pixel check with a target (US8-S5) and on the Setup page (US8-S6).

### Implementation

- [ ] T053 [US8] In `crates/finder/web/playwright.config.ts`:
  - give the existing server a fresh `--state` under `target/` and
    `reuseExistingServer: false`;
  - add a second server on 8098 with its own fresh `--state` and
    `reuseExistingServer: false`, serving a `session` project;
  - make the default project ignore `e2e/session.spec.ts`;
  - keep `FINDER_URL` behaviour (US8-S1).
- [ ] T054 [US8] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check`, `npm test` and `npm run e2e`, with every test in
  `e2e/console.spec.ts` unchanged and green. Run the `lint` and `test` gates. Commit the
  green story (FR-035–FR-036; US8-S1 to US8-S6).

## Execution Order

The stories land in this order:

```text
US1 → US2 → US3 ─┬→ US4 → US5 ─┐
                 └→ US6 ───────┴→ US7 → US8
```

Every arrow is a merge edge: the later story starts from a `main` that holds the earlier
one.
- **US2** follows US1 because both add a `pub mod` line to `lib.rs`.
- **US6** may run beside US4 and US5, because it touches only `crates/finder/web/`.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `proto/`, `scripts/ci/`,
  `.github/`, existing tests, and the Messier chart data.
