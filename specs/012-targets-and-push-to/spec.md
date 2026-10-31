---
state: ready
depends_on_landed: []
# Converted 2026-10-28 from the reviewed OpenSpec change feat-12 (#102). This trio is
# now the feature's only specification. It stays draft until the owner has reviewed
# it and given the approvals listed under "Owner approvals". Flipping it to `ready`
# is the act that dispatches it. The owner approved it on 2026-10-28.
# Flipped to ready 2026-10-29, once ergane#896 met the last prerequisite (plan.md
# § Prerequisites, item 4).
---

# Feature Specification: Targets and Push-To

**Feature Branch**: `012-targets-and-push-to`

**Created**: 2026-10-28

**Status**: Draft

**Input**: Today the finder console answers "what am I looking at": it
plate-solves one frame and charts it. At the telescope, the next question is "where do I
want to go, and how do I get there". This feature adds three things:

- a target, chosen from all of OpenNGC;
- whether that target can be pointed at now;
- push-to directions ("turn clockwise 34° · tilt up 12°") from wherever the last solved
  frame points.

It needs no new hardware. It also lays down the shared observing session that live solving
(feat-13), the mount (feat-14, feat-15) and the MCP server (feat-16) all build on.

## Clarifications

### Session 2026-10-28

- Q: What does a story read besides this trio? → A: Nothing about the feature. The trio is
  self-contained: `spec.md` holds every requirement and scenario, and `plan.md` holds
  every design decision, the interfaces, and the alternatives rejected. A story that
  would change a decision escalates rather than re-deciding it.
- Q: How is the change sliced for the factory? → A: Into eight stories, each a vertical
  slice that lands its own tests:
  - the sky ephemeris;
  - the target catalog;
  - the session core;
  - write protection and the latest pointing;
  - push-to;
  - two page slices;
  - the end-to-end suite.

  The server is built before the page, and the page before its end-to-end tests.
- Q: Where does the OpenNGC extract come from? → A: The operator stages it on `main`
  before this spec is ready, outside the factory, in PR #109, with these files:
  - `crates/finder/data/openngc.csv`;
  - its recipe, pinned commit and object count in `crates/finder/data/README.md`;
  - the script that regenerates it;
  - its row in `crates/finder/THIRD_PARTY_NOTICES.md`.

  At 734 KB, the vendored data is about three times the factory's 256 KiB diff ceiling,
  so no story can carry it. US2 builds on the extract and proves it.
- Q: How does Impeccable take part? → A: `DESIGN.md`, `.impeccable/design.json` and the
  surface briefs under `.impeccable/surfaces/` are the design system every page story
  builds to. The Impeccable review itself (shape → craft → finish review) is outside the
  factory, because a "ship verdict" is not evidence the judge can see in a diff. US7
  records the new parts in `DESIGN.md` as built, and the maintainer runs the review after
  US7 lands.
- Q: Who approves the changes to tested behaviour? → A: The owner, by reviewing this spec
  and flipping it to `ready`. The approvals are listed below so that the flip is an
  informed act.

## Owner approvals

**Approved by the owner on 2026-10-28.** The owner approved all four changes below.
This spec is flipped to `ready` once its last prerequisite is met (plan.md §
Prerequisites).

These are the changes to tested behaviour. Each is named in the FR that makes it. No existing test changes, and CI (`ci.yml`) is unchanged.

1. **The site moves from the browser into the session.** A site this browser stored is
   adopted once and then forgotten by the browser (FR-027).
2. **"Up" follows the minimum altitude.** "Up" in suggestions means at or above the
   session's minimum altitude rather than a fixed 15°, and "below 15°" shows the minimum
   altitude (FR-025, FR-028).
3. **The guidance block leads.** With a target set, the guidance block comes first and the
   roster moves below the chart (FR-033).
4. **`crates/finder/web/playwright.config.ts` gains a second server.** It is a second
   `finder` server with its own throwaway `--state`. The existing server also gets a
   throwaway `--state`, and both servers set `reuseExistingServer: false` (FR-035).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Sun and the rising, transit and setting of any altitude (Priority: P1)

As the push-to code (US5), I ask `finder::ephemeris` two kinds of question. The first is
where the Sun is at any time, which drives the Sun guard. The second is when a position
next rises above an altitude, transits, and sets below it, which tells the user when a
target that is down comes up. Both answers come from Meeus's worked methods, and both are
checked against his worked examples.

**Why this priority**: The Sun guard is the one safety rule in push-to, and it is always
on. A Sun that is off by a degree misplaces the guard everywhere. The rise and set times
are the only guidance a target that is down gets.

**Independent Test**: `cargo test -p finder --locked ephemeris` runs the unit tests in
`crates/finder/src/ephemeris.rs`. No server and no clock are needed: every input is an
explicit time, so the module is a pure function of its inputs.

**Acceptance Scenarios**:

1. **Given** Meeus's example 25.a, 1992 October 13.0 TD (JD 2448908.5), **When** a test
   in `ephemeris.rs` computes the Sun, **Then** it asserts an apparent RA of 198.38083°
   and a Dec of −7.78507°, each within 0.01°.
2. **Given** Meeus's example 15.a, Venus at Boston (42.3333° N, 71.0833° W) on 1988
   March 20 at altitude −0.5667°, from the three positions Meeus gives, **When** a test
   computes that date's times, **Then** it asserts a rising at 12:25:26, a transit at
   19:40:31 and a setting at 02:54:40 UT, each within 1 minute. These are Meeus's
   corrected 0.51766, 0.81980 and 0.12130 of a day.
3. **Given** a position at Dec +80° seen from 40° N, **When** a test asks for the times of
   altitude 15°, **Then** the answer says the position stays above 15° all day and still
   gives its transit.
4. **Given** a position at Dec −60° seen from 40° N, **When** a test asks for the times of
   altitude 0°, **Then** the answer says it never rises above 0° and still gives the time
   it is highest.
5. **Given** a fixed position and the site 40° N, 75° W, **When** a test computes its
   rising for altitude 15°, **Then** `sky::alt_az` of that position at that time is 15°
   within 0.05° and climbing.
6. **Given** M8, the site 40° N, 75° W and the time 2026-01-16 02:00 UT, **When** a test
   asks for the next times of altitude 15°, **Then** it asserts four things:
   - 02:00 UT is before the rising;
   - the rising is before the transit, and the transit before the setting;
   - `sky::alt_az` of M8 is 15° within 0.05° at the rising, and climbing;
   - it is 15° within 0.05° at the setting, and sinking.
7. **Given** the story's diff, **When** it is read, **Then** `crates/math-core` is
   untouched, and the existing tests in `crates/finder/src/sky.rs` (among them the Meeus
   13.b test of `alt_az`) are unchanged.

---

### User Story 2 - The target catalog and its search (Priority: P1)

As the observer, I type a Messier number, an NGC or IC id, or a common name, and I find
the object I mean. As every later story, I ask `finder::targets` for an object by id and
get one canonical object, offline, from the OpenNGC extract compiled into the binary.

**Why this priority**: The target is what the session holds, what push-to points at and
what the page searches. Every later story names objects through this catalog.

**Independent Test**: `cargo test -p finder --locked` runs two sets of tests:
- the unit tests in `crates/finder/src/targets.rs`;
- `crates/finder/tests/targets_api.rs`, against `router(AppState::new(..))` as
  `tests/api.rs` does.

`crates/finder/data/openngc.csv` is already on `main` (see Assumptions).

**Acceptance Scenarios**:

1. **Given** the bundled catalog, **When** a test counts its objects, **Then** the count
   equals the object count that `crates/finder/data/README.md` records for the pinned
   OpenNGC commit. That is the rows of `NGC.csv` and `addendum.csv` less the `NonEx` and
   `Dup` rows.
2. **Given** every id that OpenNGC marks `Dup`, **When** a test looks each one up,
   **Then** each finds the object OpenNGC names as the one it duplicates, and none is
   listed as an object of its own.
3. **Given** a test whose working directory is an empty temporary folder, **When** it
   searches, **Then** it gets the same answer as from the repository root. The catalog is
   `include_str!`'d, so no file on disk or network is read.
4. **Given** M1 to M110, **When** a test looks up each one in the catalog, **Then** each
   is found and lies within 1″ of the same object in `catalog::messier()`.
5. **Given** M102, M76 and M45, **When** a test looks them up, **Then** they are NGC 5866,
   NGC 650 and Mel 22, and `M111` is not found.
6. **Given** `m31`, `M 31`, `ngc224`, `NGC 0224` and `ngc0224`, **When** a test looks up
   each, **Then** each finds the one object whose canonical id is `NGC 224`.
7. **Given** `NGC 99999` and `M111`, **When** a test looks them up, **Then** nothing is
   found.
8. **Given** the queries `m31`, `ngc0224` and `andromeda`, **When**
   `GET /api/targets?q=<query>` is requested for each, **Then** each list starts with
   NGC 224 (M31).
9. **Given** the query `qqq`, **When** it is searched, **Then** the answer is 200 with an
   empty list.
10. **Given** `/api/targets` with no `q`, or with `q=` empty, **When** it is requested,
    **Then** the answer is 400 with a JSON `error`.
11. **Given** a query that matches more than 50 objects, **When** it is searched with no
    `limit`, with `limit=0` and with `limit=500`, **Then** the lists hold at most 20,
    exactly 1 and at most 50 objects, respectively.
12. **Given** `crates/finder/THIRD_PARTY_NOTICES.md`, **When** the new test
    `tests/notices.rs` reads it, **Then** it holds an OpenNGC row that names CC BY-SA 4.0
    and the commit that `data/README.md` pins. `licenses_prints_the_notices_and_exits` is
    unchanged.

---

### User Story 3 - One observing session for every device, kept across restarts (Priority: P1)

As a phone and a laptop open on the same console, we read and change one observing
session through `GET` and `PUT /api/session`:
- the site;
- the minimum altitude;
- the target;
- the console's time.

Edits are checked, so a bad value changes nothing. The binary keeps the session in a state
file across restarts. The library keeps it in memory, so tests write nothing. Every
time-dependent answer takes its time from an injectable clock.

**Why this priority**: The session is the shared state that push-to, the page, live
solving (feat-13) and the mount all read. Its clock is what keeps the Sun guard honest
under test.

**Independent Test**: `cargo test -p finder --locked` runs three sets of tests:
- the unit tests in `crates/finder/src/session.rs`;
- `crates/finder/tests/session_api.rs`, which tests the library through the router with a
  fixed clock;
- `crates/finder/tests/session_cli.rs`, which starts the built `finder` binary on a
  temporary `--state` folder, as `tests/cli.rs` runs the binary.

**Acceptance Scenarios**:

1. **Given** client A puts a site of 40° N, 75° W and the target `M42`, **When** client B
   then gets the session, **Then** B reads latitude 40 and longitude −75 exactly, and the
   target NGC 1976 with its Messier number M42.
2. **Given** a server started with no state file, **When** its session is read, **Then**
   it has no site, no target and no pointing, and a minimum altitude of exactly 15°.
3. **Given** a session, **When** a client puts latitude 95, or a minimum altitude of 85°,
   or `{"sun_guard": false}`, **Then** each answer is 400 with a JSON `error` naming the
   field, and a following read of the session is identical to the one before.
4. **Given** a session, **When** a client puts the target `NGC 99999`, **Then** the answer
   is 404 with a JSON `error`, and the session's target is unchanged.
5. **Given** a session holding a site and a target, **When** a client puts only
   `{"min_alt_deg": 30}`, **Then** the minimum altitude is 30° and the site and target are
   exactly as before.
6. **Given** a session holding a site and a target, **When** a client puts
   `{"target": null}` and then `{"site": null}`, **Then** the session has no target and
   then no site, and every other field is as before.
7. **Given** a site, a minimum altitude of 20° and the target M31, set through the binary,
   **When** it is stopped and started again with the same `--state` folder, **Then** the
   session holds the same site, minimum altitude and target exactly, and no pointing.
8. **Given** a state file holding `not json`, **When** the binary starts, **Then** three
   things happen:
   - it serves a fresh session;
   - it says on stderr that the file could not be read;
   - it keeps that file beside the new one, with a `.bad` suffix.
9. **Given** the binary started with no `--state` and with `XDG_STATE_HOME` pointing at a
   temporary folder, **When** the target is set, **Then** the session is written to
   `finder/session.json` under that folder.
10. **Given** the library's state built without a state file, with `HOME` and
    `XDG_STATE_HOME` pointing at an empty temporary folder, **When** the session is
    changed through the API, **Then** that folder is still empty.
11. **Given** the library's clock fixed at 2026-01-16 02:00 UT, **When** the session is
    read twice, **Then** both reads report the time 1768528800 exactly.
12. **Given** the binary serving outside any simulation mode, **When** its session is
    read, **Then** the reported time is within 2 s of the system clock at the moment of
    the read.
13. **Given** the story's diff, **When** it is read, **Then** three things hold:
    - `AppState::new`, `AppState::shared`, `router`, `Observer` and `Frame` keep their
      signatures and fields;
    - `tests/api.rs` is unchanged;
    - the binary has no option that builds a fixed clock.
14. **Given** the crate docs and `crates/finder/README.md`, **When** they are read,
    **Then** they document `GET`/`PUT /api/session` and `--state`, and
    `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps` builds.

---

### User Story 4 - Only the console's own pages change it, and every solve sets the latest pointing (Priority: P1)

As the console, I take two kinds of protection:
- I refuse writes from a page on another site, and from a page on a DNS name rebound to my
  address. A phone on the LAN, or curl and the Rust tests, still reach me.
- Every frame that solves becomes the session's latest pointing: RA and Dec, the frame's
  time, and altitude and azimuth when a site is set. That pointing follows the site as it
  changes.

**Why this priority**: The session will soon drive a motorised mount. A console that any
web page can steer is unsafe before it is useful. The latest pointing is what push-to
directions start from.

**Independent Test**: `cargo test -p finder --locked` runs two sets of tests:
- `crates/finder/tests/session_writes.rs`, for Origin, Host and content type;
- `crates/finder/tests/pointing_api.rs`, which solves the bundled `orion2.jpg` and
  `roof.jpg` samples through `POST /api/solve` under a fixed clock.

`tests/api.rs` passes unchanged.

**Acceptance Scenarios**:

1. **Given** a request to put the target M42 that carries
   `Origin: http://elsewhere.example`, and a frame posted to `/api/solve` with the same
   header, **When** each is sent, **Then** both are answered 403 with a JSON `error`, and
   the session's target and pointing are unchanged.
2. **Given** the same edit with an `Origin` equal to the scheme, host and port it was sent
   to, or with no `Origin` at all, **When** it is sent, **Then** it is applied and
   answered 200 with the session.
3. **Given** an edit sent as `text/plain` whose body would parse as JSON, **When** it is
   sent, **Then** the answer is 415 with a JSON `error`, and nothing changes.
4. **Given** a request to put the target M42 with `Host: evil.example:8097` and
   `Origin: http://evil.example:8097`, **When** it is sent, **Then** the answer is 403
   with a JSON `error`, and the session is unchanged.
5. **Given** the console started with `--allow-host surface.local`, **When** the same
   edit carries `Host: surface.local:8097` and `Origin: http://surface.local:8097`,
   **Then** it is applied and answered 200 with the session.
6. **Given** the console's clock fixed at 2026-01-16 02:00 UT and the site 40° N, 75° W,
   **When** `orion2.jpg` is solved, **Then** three things hold:
   - the pointing is RA 85.190964° and Dec −4.246105°, within 1e-4°;
   - its altitude and azimuth equal `sky::alt_az` of that position for that site and
     time, within 1e-9°;
   - its time is the fixed clock's, exactly.
7. **Given** orion2 has been solved, **When** `roof.jpg` is then solved at the defaults,
   **Then** the session's pointing is still orion2's, field for field.
8. **Given** a session with no site, **When** orion2 is solved, **Then** the pointing has
   RA, Dec and time, and no altitude or azimuth.
9. **Given** orion2 solved with no site under a clock fixed at 2026-01-16 02:00 UT,
   **When** the site 40° N, 75° W is then put, **Then** the pointing's altitude and
   azimuth equal `sky::alt_az` of 85.190964°, −4.246105° for that site at 1768528800,
   within 1e-9°.
10. **Given** that pointing, **When** the site is cleared, **Then** the pointing keeps its
    RA, Dec and time exactly, and has no altitude or azimuth.
11. **Given** a solve that sets the pointing, **When** its report is read, **Then** it
    carries an optional `pointing_seq` equal to the session's pointing `seq`.
    `solve_frame` keeps its signature.
12. **Given** the story's diff, **When** it is read, **Then** `tests/api.rs` is unchanged,
    and the crate docs and `crates/finder/README.md` document `--allow-host` and the
    same-origin rule.

---

### User Story 5 - Whether a target is pointable, and how to push there (Priority: P1)

As the observer with a target, I read the guidance in the session, derived at read time
on the console's clock. It tells me one of three things:
- the target is under the Sun guard, below the horizon or too low, with when it next rises
  to the minimum altitude, transits and sets;
- it is pointable, with the turn and tilt from the latest pointing and the target's place
  on or off the solved frame;
- a site is needed.

Searches carry each result's class, and suggestions rank by the session's minimum
altitude.

**Why this priority**: This is the feature's purpose. Everything before it exists so that
this story can say "turn clockwise 34° · tilt up 12°" correctly, and refuse near the Sun.

**Independent Test**: `cargo test -p finder --locked` runs two sets of tests:
- the unit tests in `crates/finder/src/pushto.rs`, which use explicit alt-az pairs, sites
  and times;
- `crates/finder/tests/pushto_api.rs`, which uses `GET /api/session`, `/api/targets` and
  `/api/similar` with a fixed clock and orion2 solved.

**Acceptance Scenarios**:

1. **Given** the site 40° N, 75° W, the time 2026-01-16 02:00 UT and a minimum altitude
   of 15°, **When** M42, M8 and M75 are each classed, **Then**:
   - M42 (altitude 42.9°) is pointable;
   - M8 (−66.6°) is below the horizon;
   - M75, 3.8° from the Sun, is under the Sun guard, though it is also below the horizon.
2. **Given** a minimum altitude of 50° at the same site and time, **When** M42 is classed,
   **Then** it is too low.
3. **Given** a session with no site at that time, **When** M75 and M42 are classed,
   **Then** M75 is under the Sun guard, and M42 is classed as needing a site.
4. **Given** M8 as the target at 40° N, 75° W at 2026-01-16 02:00 UT with a minimum of
   15°, **When** the guidance is read, **Then** its rising time is later than that time,
   and at the rising time `sky::alt_az` of M8 is 15° within 0.05°, and climbing.
5. **Given** a minimum altitude of 30° (M8 culminates at about 25.6° from 40° N), **When**
   the guidance is read, **Then** it says M8 never reaches the minimum altitude that day,
   and gives its transit.
6. **Given** a pointing at azimuth 350°, altitude 30° and a target at azimuth 24°,
   altitude 42°, **When** the turn and tilt are computed, **Then** they are clockwise 34°
   and up 12°, within 1e-9°.
7. **Given** a pointing at azimuth 20°, altitude 50° and a target at azimuth 350°,
   altitude 41°, **When** the turn and tilt are computed, **Then** they are anticlockwise
   30° and down 9°, within 1e-9°.
8. **Given** a pointing at azimuth 0° and a target at azimuth 180°, **When** the turn is
   computed, **Then** it is +180°, clockwise. The wrap maps exactly −180° to +180°.
9. **Given** M8 below the horizon as the target and a pointing that exists, **When** the
   guidance is read, **Then** no turn or tilt is given.
10. **Given** orion2 solved at 40° N, 75° W at 2026-01-16 02:00 UT with M78 as the
    target, **When** the guidance is read at 03:00 UT by the console's fixed clock (M78
    being pointable then), **Then** the turn and tilt are still given. They run from the
    pointing's altitude and azimuth at 02:00 to M78's at 03:00, within 1e-9°. A pointing
    from an uploaded frame never goes stale.
11. **Given** the same solve, **When** the guidance is read at 02:10 UT, **Then** the turn
    and tilt start from the pointing's altitude and azimuth at 02:00 and end at M78's at
    02:10, each within 1e-9°. So they differ from the 02:00 guidance by M78's own motion.
12. **Given** a clock fixed at 2026-01-16 02:00 UT, the site 40° N, 75° W and a minimum
    altitude of 15°, **When** `m42` and `m75` are searched, **Then** M42's class is
    pointable and M75's is under the Sun guard.
13. **Given** orion2 as the latest pointing and M78 as the target, **When** the guidance's
    placement is read, **Then** it equals the solve report's mark for M78: position, axes
    and angle, within 1e-9.
14. **Given** orion2 as the latest pointing and M79 as the target, **When** the placement
    is read, **Then** the target is off the frame. Its direction and distance are the ones
    the solve report gives M79 among the objects just off the frame, within 1e-9°.
15. **Given** a session minimum altitude of 50°, **When** objects like M42 are asked for
    at 40° N, 75° W at 2026-01-16 02:00 UT, **Then** M78, below 50° at that time, is not
    up and comes after any suggestion at or above 50°.
16. **Given** the story's diff, **When** it is read, **Then** `similar()` keeps its
    signature and calls `similar_above()` with 15°, and `similar()`'s unit tests and
    `similar_objects_rank_for_an_observer` are unchanged.
17. **Given** a session with a target, **When** `GET /api/session` is read, **Then** it
    carries `guidance` with `class` and, where they apply, `rise`, `transit`, `set`,
    `never`, `turn_deg`, `tilt_deg` and `placement`, all derived at read time on the
    console's clock.

---

### User Story 6 - The page shares the session, and the Setup page sets the site and minimum altitude (Priority: P2)

As the observer on a phone and a laptop, the page reads the session. It polls every 5 s
while visible. It adopts a site this browser stored under the earlier console once, then
forgets it. A Setup page at `/setup`, reached from the top bar, sets the site and the
minimum altitude. Suggestions say "below 30°" when the minimum is 30°.

**Why this priority**: The page is how the observer uses the session. The Setup page is
where the site lives now that it has left the browser. It is P2 because it consumes the
server stories' API.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`, which runs Vitest:
- the new tests are `src/useSession.test.ts` and `src/components/SetupPage.test.tsx`;
- `App.test.tsx`, `settings.test.ts`, `SiteDialog.test.tsx` and `NextTargets.test.tsx`
  are unchanged.

Then the `test` gate's build-freshness step passes on the rebuilt `web/dist` (the same
check CI's `finder-page` job runs).

**Acceptance Scenarios**:

1. **Given** `useSession()` under fake timers, **When** the page is visible, **Then** a
   Vitest test asserts three things:
   - it gets `/api/session` on mount and every 5 s, and stops while the page is hidden;
   - `put(edit)` sends `PUT /api/session` with a JSON body;
   - it refetches once after the edit.
2. **Given** a session with no site and a browser that stored 40° N, 75° W under the
   earlier console, **When** the page loads, **Then** three things hold:
   - the session's site becomes 40° N, 75° W;
   - the browser no longer holds its own copy;
   - a later load in that browser leaves the session's site alone.
3. **Given** App.test's fetch mock, which answers `/api/session` with the health body,
   **When** the app renders, **Then** the session reads as empty, and every existing
   App.test case passes unchanged.
4. **Given** the Setup page, **When** the user sets 30° and returns to the console,
   **Then** the session's minimum altitude is 30°, and suggestions that are down read
   "below 30°".
5. **Given** the Setup page, **When** the user enters 90° for the minimum altitude,
   **Then** nothing is saved and the page says that 0–80° is allowed.
6. **Given** the Setup page, **When** the user enters latitude 95, **Then** the site is
   not saved and the page says which ranges are allowed, as SiteDialog does.
7. **Given** NextTargets with a `minAlt` of 30, **When** suggestions load for M42 at
   40° N, 75° W, **Then** every suggestion that is down reads "below 30°". With no
   `minAlt`, the existing NextTargets tests ("below 15°") pass unchanged.
8. **Given** the top bar, **When** Setup is chosen, **Then** the URL is `/setup` through
   `history.pushState`, and the Setup page leads back to the console. No npm package is
   added, so `package.json` dependencies are unchanged.
9. **Given** the rebuilt `web/dist`, **When** the `test` gate runs `npm run build` and then
   `git status --porcelain -- dist`, **Then** it lists nothing: the committed build is the
   build of this source.

---

### User Story 7 - Choosing a target, the guidance block, and the target on the chart (Priority: P2)

As the observer, I open the target sheet from the top bar, type `andro`, and pick
M31. A guidance block then leads the page with one of four messages:
- the turn and tilt, with the pointing's age;
- the rise, transit and set times;
- the Sun-guard refusal;
- a prompt to set a site.

When the charted frame is the latest pointing, the chart shows the target in a reticle,
or an edge arrow toward it.

**Why this priority**: This is the visible half of push-to. It is P2 because it consumes
US5's guidance and US6's session hook.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`:
- the new tests are `src/components/TargetSheet.test.tsx`,
  `src/components/GuidanceBlock.test.tsx`, `src/components/TargetMark.test.tsx` and
  `src/App.target.test.tsx`;
- `Chart.test.tsx`, `App.test.tsx` and every other existing test are unchanged.

Then the `test` gate's build-freshness step passes.

**Acceptance Scenarios**:

1. **Given** the target sheet, **When** the user types `andro`, **Then** NGC 224 (M31),
   the Andromeda Galaxy, is listed first with the class the search answer gives it.
   Picking it puts it as the session's target, closes the sheet, and the guidance block
   names it.
2. **Given** the target sheet, **When** the user types `qqq`, **Then** the sheet lists
   nothing and says that nothing matches.
3. **Given** a target, **When** the user clears it from the sheet, **Then** the session
   has no target, and the page returns to its layout without one.
4. **Given** orion2 as the latest pointing and M78 as the target under a fixed clock,
   **When** the guidance block renders, **Then** it reads the session's turn and tilt,
   rounded to whole degrees, in the form "turn clockwise 34° · tilt up 12°", with the
   pointing's age.
5. **Given** M8 as the target at 40° N, 75° W at 2026-01-16 02:00 UT, **When** the block
   renders, **Then** it says M8 is below the horizon and gives its rising, transit and
   setting times in the device's local time, to the minute, with no turn or tilt.
6. **Given** M75 as the target at that time, **When** the block renders, **Then** it says
   the console will not point within 15° of the Sun, with no directions.
7. **Given** M42 as the target and no site, **When** the block renders, **Then** it says
   a site is needed and links to `/setup`, with no times or directions.
8. **Given** the page showing M78, **When** another client puts the target M31, **Then**
   the guidance block names M31 within 5 s (fake timers).
9. **Given** orion2 charted as the latest pointing with M78 as the target, **When** the
   chart renders, **Then** the target reticle is centred on M78's mark within 0.5 px. The
   legend still has its five chips (Grid, Matched, Unseen, Deep sky, Centroids), with the
   same counts as without a target.
10. **Given** M79 as the target instead, **When** the chart renders, **Then** an arrow at
    the frame's edge points within 1° of the direction the report gives M79, labelled M79
    with its distance in whole degrees.
11. **Given** a report whose `pointing_seq` differs from the session's pointing, or is
    absent, **When** the chart renders, **Then** no target mark is drawn.
12. **Given** M78 as the target, **When** the app renders orion2, **Then** the reading
    order is the guidance block, the title block, the chart and its legend, then what is
    in the field and the rest.
13. **Given** `DESIGN.md` and `.impeccable/design.json`, **When** they are read, **Then**
    they describe four new parts as built: the guidance block, the target reticle, the
    edge arrow and the Setup page.
14. **Given** the rebuilt `web/dist`, **When** the `test` gate runs `npm run build` and then
    `git status --porcelain -- dist`, **Then** it lists nothing: the committed build is the
    build of this source, and no npm package was added.

---

### User Story 8 - End to end, on a server of its own (Priority: P3)

As the maintainer, I run `npm run e2e` with my own console open on 8097, and nothing I
care about is touched. The existing tests run on a server with a throwaway `--state`. The
tests that write to the session run on a second server on 8098, which has its own
throwaway `--state`. Those tests prove four things in Chromium:
- the first screen with a target, at both phone sizes;
- a site set on one device and seen on another;
- the Setup page opened directly;
- the red-only night rule.

**Why this priority**: These tests confirm, in a real browser, what the unit tests prove
piece by piece. They come last because they exercise every story before them.

**Independent Test**: In `crates/finder/web`, `npm run e2e` runs every test in
`e2e/console.spec.ts` unchanged and green, plus the new `e2e/session.spec.ts` in the
`session` project.

**Acceptance Scenarios**:

1. **Given** `playwright.config.ts`, **When** it is read, **Then** five things hold:
   - the existing server starts with `--state` in a fresh folder under `target/`, with
     `reuseExistingServer: false`;
   - a second `finder` server on port 8098 starts with its own fresh `--state`, also with
     `reuseExistingServer: false`, and serves a `session` project;
   - the default project ignores the session tests;
   - with `FINDER_URL` set, the tests run against that server;
   - `e2e/console.spec.ts` is unchanged.
2. **Given** M78 as the target and orion2 charted, **When** a committed Playwright test
   in `e2e/session.spec.ts` lays the page out at 412 × 915 and at 390 × 844 CSS px,
   **Then** it asserts that the bounding boxes of the guidance block, the title block, the
   chart and the legend lie within the viewport, in that order from the top, and that the
   roster's box starts below the legend's.
3. **Given** two browser contexts in a committed Playwright test, **When** the site
   40° N, 75° W is saved in one and the console is opened in the other, **Then** the test
   asserts that the second shows the same site, and that its suggestions are ranked for
   it.
4. **Given** a committed Playwright test, **When** it opens `/setup` directly, **Then** it
   asserts that the Setup page is rendered, from the page's own route fallback.
5. **Given** orion2 charted in night mode with M78 as the target, **When** a committed
   Playwright test runs the night pixel check on its screenshot, **Then** it asserts no
   pixel with any green or blue, and none redder than the ink beyond two levels of
   compositing rounding.
6. **Given** the Setup page in night mode, **When** a committed Playwright test runs the
   night pixel check on its screenshot, **Then** it asserts no off-red pixel, as for the
   console.

---

### Edge Cases

- **Session writes:**
  - A latitude of exactly ±90°, a longitude of exactly ±180°, and minimum altitudes of
    exactly 0° and 80° are accepted. One step beyond any of them is refused with 400.
  - A `PUT` body that is not an object, or that has unknown fields, is refused with 400,
    and nothing changes.
- **Guidance edge cases:**
  - A target at the zenith has no meaningful azimuth for the turn. Push-to still gives the
    tilt, and a turn wrapped into (−180°, 180°].
  - A target and pointing with the same azimuth give a turn of 0°.
- **Startup and storage:**
  - An unreadable state file is never fatal. A missing state folder is created on the
    first write, and a write that fails leaves the previous file intact. Writes are atomic:
    a temporary file in the same folder, synced, then renamed over the old file.
  - A pointing is not persisted. After a restart there is no pointing until a frame solves.
- **The page:**
  - It treats every field of `/api/session` as optional, and missing reads as unset, so
    older or mocked servers render an empty session.
  - The Sun guard has no override anywhere: not in the API, the page or the Setup page.

## Requirements *(mandatory)*

### Functional Requirements

**Sky ephemeris** (US1):

- **FR-001**: The system SHALL compute the Sun's apparent right ascension and declination
  for any time by Meeus's low-accuracy method (ch. 25), to within 0.01° over the years
  1950 to 2100. **Refs**: Meeus, *Astronomical Algorithms* 2nd ed., ch. 25 and example
  25.a.
- **FR-002**: The system SHALL compute, for a position, a site, a UT date and an altitude,
  the times on that date when the position rises above that altitude, crosses the
  meridian and sets below it. These are Meeus's m1, m0 and m2, in no particular order,
  each to within one minute. Alternatively it SHALL say that on that date the position
  stays above that altitude or never reaches it. A position that moves SHALL be
  interpolated over the date as Meeus does. **Refs**: Meeus ch. 15 and example 15.a.
- **FR-003**: The system SHALL compute, for a position, a site, a time and an altitude,
  the next rising: the first at or after that time within the next 24 sidereal hours. It
  SHALL then give the first transit and the first setting after that rising, each to
  within one minute, from the times on that UT date and the next. When the position stays
  above the altitude or never reaches it, the system SHALL say so and give the next
  transit. **Refs**: Meeus ch. 15.

**Target catalog** (US2):

- **FR-004**: The system SHALL bundle every object in OpenNGC's NGC, IC and addendum lists
  except those typed `NonEx`. Each object SHALL carry its id, Messier number, common names,
  type, J2000 position, axes, position angle, magnitude and constellation. A row typed
  `Dup` SHALL NOT be an object of its own: its id SHALL find the object it duplicates. The
  catalog SHALL be compiled into the binary, so it works offline. **Refs**: OpenNGC (Mattia
  Verga, CC BY-SA 4.0) at a pinned commit; GLOSSARY.md (target).
- **FR-005**: The catalog's Messier objects SHALL be the finder console's 110 Messier
  objects. Each Messier number SHALL find one object at the Messier catalog's J2000
  position, with M102 as NGC 5866, M76 as NGC 650 and M45 as Mel 22. **Refs**:
  finder-console "The Messier catalog"; the Messier extract's notes.
- **FR-006**: The system SHALL find an object by an exact id: a Messier number or a
  catalogue designation (NGC, IC or an addendum catalogue). The id SHALL match in any
  case, with or without a space before the number, and with or without leading zeros.
  Each object SHALL have one canonical id: its designation with a space and no leading
  zeros (`NGC 224`), or its Messier number when it has no other. **Refs**: OpenNGC naming
  (`NGC0224`, `IC0434`, `Mel022`).
- **FR-007**: The system SHALL answer `GET /api/targets?q=` with the objects the query
  names:
  - exact ids first;
  - then objects with a common-name word that the query begins (any case), brightest
    first, with objects that have no magnitude last.

  `limit` SHALL default to 20 and be clamped to 1–50. A missing or empty `q` SHALL be
  answered 400 with a JSON `error`. **Refs**: GLOSSARY.md (target); OpenNGC common names.
- **FR-008**: The crate's third-party notices SHALL credit OpenNGC under CC BY-SA 4.0, at
  the commit the extract's README pins. A test SHALL hold the two together. **Refs**:
  OpenNGC licence; `crates/finder/THIRD_PARTY_NOTICES.md`.

**Observing session** (US3):

- **FR-009**: The system SHALL hold one observing session and SHALL answer
  `GET /api/session` with its site, minimum altitude, target, latest pointing and the
  console's time, the same for every client. A fresh session SHALL have no site, no target
  and no pointing, and a minimum altitude of 15°. **Refs**: GLOSSARY.md (observing session,
  site, target, minimum altitude); ADR-0001.
- **FR-010**: The system SHALL accept `PUT /api/session` with any of `site`,
  `min_alt_deg` and `target`. It SHALL keep fields that were not sent, and clear `site` or
  `target` when either is sent as null. It SHALL refuse the following with 400:
  - a latitude outside ±90°;
  - a longitude outside ±180°;
  - a minimum altitude outside 0–80°;
  - an unknown field.

  It SHALL refuse a target the catalog does not hold with 404. Each refusal SHALL carry a
  JSON `error` and change nothing. **Refs**: GLOSSARY.md (site: east positive; minimum
  altitude; Sun guard: no override).
- **FR-011**: The `finder` binary SHALL keep the session's site, minimum altitude and
  target in a state file, and SHALL load it on start. The file SHALL live under
  `$XDG_STATE_HOME/finder/` by default (`~/.local/state/finder/` when that is unset), or
  under the folder `--state` names. It SHALL be written atomically, so that a crash never
  leaves a partial file. The library SHALL keep the session in memory unless it is given a
  state file. **Refs**: XDG Base Directory Specification (`XDG_STATE_HOME`).
- **FR-012**: The session SHALL report the console's clock as its time, and everything the
  system derives from the session SHALL use that clock. A clock other than the system's
  SHALL be settable only by tests and by a simulation mode that never reaches real
  hardware. **Refs**: GLOSSARY.md (Sun guard: always on);
  `docs/research/2026-10-21-scope-node-runtime.md` §7.
- **FR-013**: The crate docs and `crates/finder/README.md` SHALL document
  `GET`/`PUT /api/session` and `--state`. `AppState::new`/`shared`, `router`, `Observer`
  and `Frame` SHALL keep their shapes, so every existing caller and test compiles
  unchanged.

**Write protection and the latest pointing** (US4):

- **FR-014**: Every route that changes the session SHALL refuse, with 403 and a JSON
  `error`, a request whose `Origin` header names an origin other than the one the request
  was sent to. It SHALL allow a request with no `Origin`. The one exception is a route that
  only stops the telescope. Edits through `PUT /api/session` SHALL also need a JSON body,
  and other media types SHALL be refused with 415. **Refs**: RFC 6454 §7; OWASP CSRF
  Prevention Cheat Sheet.
- **FR-015**: Every route that changes the session, except one that only stops the
  telescope, SHALL refuse a request that carries an `Origin` unless its `Host` is an IP
  literal, `localhost`, or a name given with `--allow-host`. The refusal SHALL be a 403
  with a JSON `error`. `--allow-host` SHALL be documented in the crate docs and README.
  **Refs**: OWASP CSRF Prevention Cheat Sheet; RFC 6454 §7.
- **FR-016**: The system SHALL make each frame that solves the session's latest pointing.
  The pointing holds four things:
  - the J2000 RA and Dec at the aim point, which is the frame centre until an aim point is
    set;
  - the frame's time on the console's clock (for an uploaded frame, the time it is
    solved);
  - its altitude and azimuth at that time, when the session has a site;
  - a sequence number, which the solve report carries as `pointing_seq`.

  A frame that does not solve SHALL leave the pointing as it was. **Refs**: GLOSSARY.md
  (aim point, sync).
- **FR-017**: When the session's site is set or changed, the system SHALL recompute the
  latest pointing's altitude and azimuth from its RA, Dec and frame time for the new site.
  When the site is cleared, it SHALL drop them and keep the RA, Dec and time. **Refs**:
  push-to "Turn and tilt".

**Push-to** (US5):

- **FR-018**: The system SHALL class the session's target at the console's time by this
  precedence:
  1. under the Sun guard (centre within 15° of the Sun's);
  2. below the horizon (altitude below 0°);
  3. too low (below the minimum altitude);
  4. otherwise, pointable.

  Without a site it SHALL still apply the Sun guard, and otherwise say that a site is
  needed. The Sun guard SHALL always be on, with no override. **Refs**: GLOSSARY.md
  (pointable, Sun guard, minimum altitude).
- **FR-019**: For a target below the horizon or too low, the system SHALL give when it
  next rises to the minimum altitude, when it next transits, and when it then sets below
  the minimum altitude. Alternatively it SHALL say that the target never reaches the
  minimum altitude on that date. **Refs**: FR-003.
- **FR-020**: For a pointable target and a latest pointing that has an altitude and
  azimuth and has not gone stale, the system SHALL give the turn and the tilt:
  - The turn is the target's azimuth less the pointing's, wrapped to more than −180° and
    at most 180°. It is clockwise (seen from above) when positive.
  - The tilt is the target's altitude less the pointing's. It is up when positive.

  Otherwise it SHALL give neither. **Refs**: GLOSSARY.md (push-to, turn, tilt).
- **FR-021**: A latest pointing SHALL go stale only where another requirement says so. A
  pointing from an uploaded frame SHALL never go stale, whatever its age. **Refs**:
  plan.md § Risks (the guidance trusts the latest pointing).
- **FR-022**: The system SHALL hold the latest pointing's altitude and azimuth at their
  values at its frame's time, since an alt-az telescope left alone stays put. The target's
  altitude and azimuth SHALL follow the console's clock. **Refs**:
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §3.
- **FR-023**: Each result of `GET /api/targets` SHALL carry its class at the console's
  time, classed as FR-018 classes the session's target. **Refs**: FR-007, FR-018.
- **FR-024**: For the latest pointing's frame, the system SHALL place the target as it
  places deep-sky objects when the target's symbol overlaps the frame: position, axes and
  angle in the frame's pixels. Otherwise it SHALL give the target's direction from the
  frame's centre, in degrees clockwise from up, and its distance. **Refs**: finder-console
  "Deep-sky objects on and near the frame".
- **FR-025**: *(MODIFIED, owner approval 2.)* The system SHALL answer
  `GET /api/similar?id=<Messier number>` with objects of the same family, nearest first.
  The Crab, the only supernova remnant, borrows the planetary and diffuse nebulae. The
  object itself and anything within 1° of it are excluded.

  Given `lat` and `lon`, and optionally `time` (Unix seconds, default now), each
  suggestion SHALL carry three things:
  - its altitude;
  - its azimuth;
  - whether it is up, meaning at or above the observing session's minimum altitude (15°
    unless changed).

  Suggestions that are up SHALL come before those that are not. `limit` defaults to 8 and
  is clamped to 1–20. An unknown object SHALL be answered 404. A latitude or longitude out
  of range, only one of them, or no `id` SHALL be answered 400. `similar()` keeps its
  signature and its 15° behaviour, through `similar_above()`. **Refs**: `src/catalog.rs`
  `similar`; tests `similar_objects_rank_for_an_observer` and `similar()`'s unit tests,
  unchanged.
- **FR-026**: `GET /api/session` SHALL carry the target's guidance, derived at read time
  on the console's clock: `class`, and where they apply `rise`, `transit`, `set`, `never`,
  `turn_deg`, `tilt_deg` and `placement`. One poll then gives the page the session and its
  guidance, and the two can never disagree. **Refs**: plan.md § Interfaces and
  § Decisions.

**The page: session and Setup** (US6):

- **FR-027**: *(MODIFIED, owner approval 1.)* The page SHALL let the user set a site,
  forget it, and keep it in the observing session, so every device shows the same site:
  - a latitude (−90 to 90) and a longitude (−180 to 180, east positive), refusing anything
    else with a message;
  - filled from the device's location when the browser shares it, saying why not
    otherwise.

  When the session has no site and this browser stored one before, the page SHALL put
  that site into the session on its first load and then forget it from the browser, so it
  is adopted once. **Refs**: `web/src/components/SiteDialog.tsx`, `web/src/settings.ts`;
  SiteDialog and settings tests, unchanged.
- **FR-028**: *(MODIFIED, owner approval 2.)* The page SHALL suggest objects like the
  reference, giving each one's altitude and compass direction at the observer's site. A
  suggestion that is down SHALL read "below" the session's minimum altitude ("below 15°"
  at the default), with a horizon mark in readable ink. NextTargets SHALL take an optional
  `minAlt` (default 15), so its existing tests stand. **Refs**:
  `web/src/components/NextTargets.tsx`; NextTargets tests, unchanged.
- **FR-029**: The page SHALL have a Setup page at `/setup`. It SHALL be reachable from the
  top bar, set in the field atlas design, and lead back to the console. It SHALL set the
  session's site (typed, or from the device's location) and its minimum altitude (0–80°,
  refusing other values with a message). Routing SHALL use `location.pathname` and
  `history.pushState`, with no new npm package. **Refs**: FR-010; DESIGN.md; the Setup
  page's surface brief.

**The page: target and guidance** (US7):

- **FR-030**: The page SHALL offer a target sheet from the top bar. The sheet SHALL search
  the catalog as the user types, and list each match with its id, Messier number, common
  name, kind, magnitude and whether it is pointable now. Picking a match SHALL make it the
  session's target, and the sheet SHALL also offer to clear the target. **Refs**: FR-007,
  FR-023; DESIGN.md (Sheets, Lists).
- **FR-031**: With a target set, the page SHALL show a guidance block naming the target
  and its class, followed by one of four messages:
  - when pointable, the turn and tilt from the latest pointing to whole degrees ("turn
    clockwise 34° · tilt up 12°") and the pointing's age, or a prompt to solve a frame;
  - when down, when it rises, transits and sets, in the device's local time to the minute;
  - under the Sun guard, that the console will not point there;
  - without a site, that a site is needed, leading to the Setup page.

  The block SHALL refresh at least every 5 s. **Refs**: FR-018 to FR-026.
- **FR-032**: When the charted frame is the latest pointing (the report's `pointing_seq`
  equals the session's), the page SHALL draw the target on the chart. On the frame, it is
  the family's atlas symbol inside a target reticle, labelled. Off the frame, it is an
  arrow at the frame's edge pointing toward the target, labelled with its distance. The
  target SHALL NOT be a legend layer, and SHALL follow night mode's red-only rule.
  **Refs**: FR-024; DESIGN.md (Chart, the Red Torch Rule).
- **FR-033**: *(MODIFIED, owner approval 3.)* With a target set, the page's reading order
  SHALL be:
  1. the guidance block and the title block;
  2. the chart and its legend;
  3. what is in the field, and the rest.

  On a phone of 412 × 915 or 390 × 844 CSS px, the first screen SHALL hold the guidance
  block, the title block, and the whole chart and its legend. Without a target, the page's
  order and first screen are as today. **Refs**: `web/src/App.tsx`; App and ChartHeader
  tests, unchanged.
- **FR-034**: `DESIGN.md` and `.impeccable/design.json` SHALL record the guidance block,
  the target reticle, the edge arrow and the Setup page as built. **Refs**: DESIGN.md;
  `.impeccable/design.json`.

**End to end** (US8):

- **FR-035**: *(Owner approval 4.)* The end-to-end suite SHALL be configured as follows:
  - The existing `finder` server SHALL start with a throwaway `--state` and
    `reuseExistingServer: false`.
  - The tests that write to the session SHALL run in a `session` project, against a second
    server on port 8098 with its own throwaway `--state`, also with
    `reuseExistingServer: false`.
  - The default project SHALL ignore those tests.
  - `FINDER_URL` SHALL keep running the tests against a given server.

  So a developer's own console is never reused or written to. **Refs**: plan.md § User
  Story 8.
- **FR-036**: The end-to-end suite SHALL prove four things in Chromium:
  - FR-033's first screen with a target, at both phone sizes;
  - a site set on one device and seen on another;
  - `/setup` opened directly;
  - the night pixel check with a target set and on the Setup page.

  **Refs**: FR-027, FR-029, FR-032, FR-033.

### Key Entities

- **Observing session**: site (latitude and longitude, east positive), minimum altitude
  (degrees, 0–80, default 15), target (a catalog id), latest pointing, and the console's
  time. It is shared by every client, and persisted except for the pointing.
- **Latest pointing**: `seq`; the solved `Frame`; J2000 RA and Dec at the aim point; the
  frame's time; altitude and azimuth when a site is set.
- **Catalog object**: canonical id, Messier number, common names, type, J2000 RA and Dec,
  axes, position angle, magnitude and constellation. `Dup` ids are aliases to it.
- **Guidance**: `class` (`sun_guard`, `below_horizon`, `too_low`, `pointable` or
  `no_site`); `rise`, `transit` and `set`, or `never`; `turn_deg` and `tilt_deg`; and
  `placement`, which is either on the frame (position, axes and angle) or off it
  (direction and distance).
- **Clock**: `now() -> f64` Unix seconds, in two implementations. `SystemClock` is the
  binary's. `FixedClock` is for tests, and for the simulation mode that feat-13 adds.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it, and
  `cargo test --workspace --locked`, `npm test` and `npm run e2e` pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml` or
  `CODEOWNERS` entry is edited by any story. The four owner approvals are the only changes
  to tested behaviour.
- **SC-003**: Workspace line coverage stays at or above the 80% floor after every story.
- **SC-004**: The Sun-guard classification of M75 at 2026-01-16 02:00 UT from
  40° N, 75° W holds with or without a site, and no API field, page control or option
  turns the guard off.
- **SC-005**: A developer's console on port 8097 and its state folder are untouched by
  `cargo test` and `npm run e2e`.

## Assumptions

- **The OpenNGC extract is staged by the operator.** `crates/finder/data/openngc.csv`,
  about 1 MB, is extracted at a pinned OpenNGC commit. It covers every row of `NGC.csv`
  and `addendum.csv`, with `NonEx` dropped and `Dup` rows as aliases. Its recipe, commit,
  object count and regenerating script are recorded in `crates/finder/data/README.md`,
  and its row is in `THIRD_PARTY_NOTICES.md`. All of this is on `main` before this spec is
  flipped to `ready`.
- **Deferred to later changes:**
  - refraction and apparent altitude (feat-14);
  - the aim point's calibration (feat-13); until then, the aim point is the frame centre;
  - live frames and their 5 s freshness rule (feat-13).
- **Out of scope:**
  - drawing NGC and IC objects on the chart (it keeps drawing Messier objects only);
  - typed RA and Dec;
  - named stars, the Moon and planets as targets;
  - a horizon that varies by direction.
- **Done by the maintainer after US7 lands:** the Impeccable review of the new parts.
- **The factory's gates and CI:**
  - The factory's local gates are `lint` and `test`. `test` runs the cargo tests and
    then, in `crates/finder/web`, the checks CI's required `finder-page` job runs:
    `npm ci`, `npm run typecheck`, `npm run format:check`, `npm test`, `npm run build`
    followed by the build-freshness step (`git status --porcelain -- dist` lists
    nothing), and `npm run e2e`. Every story's gate results carry them, so the page
    stories' Vitest and Playwright scenarios have gate evidence.
  - `crates/finder/web/dist/**` is marked generated in `.gitattributes`, so a rebuilt
    bundle appears in a story's diff as changed files rather than as ~590 KB of
    minified text.
  - Every story's tasks still run these checks before proposing.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006, FR-007, FR-008]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-009, FR-010, FR-011, FR-012, FR-013]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-014, FR-015, FR-016, FR-017]
US5:
  depends_on: []
  depends_on_merged: [US1, US4]
  implements: [FR-018, FR-019, FR-020, FR-021, FR-022, FR-023, FR-024, FR-025, FR-026]
US6:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-027, FR-028, FR-029]
US7:
  depends_on: []
  depends_on_merged: [US5, US6]
  implements: [FR-030, FR-031, FR-032, FR-033, FR-034]
US8:
  depends_on: []
  depends_on_merged: [US7]
  implements: [FR-035, FR-036]
```

Every edge is a merge edge, because each dependent imports what its predecessor creates:

| Story | Builds on |
|---|---|
| US2 | US1: both add a `pub mod` line to `crates/finder/src/lib.rs`, so US2 starts from a `main` that holds US1's |
| US3 | US2: it validates targets against `finder::targets` |
| US4 | US3: it extends `finder::session` and its routes |
| US5 | US1: it reads the Sun and rise/set times from `finder::ephemeris` |
| US5 | US4: it reads the latest pointing |
| US6 | US3: `useSession` and the Setup page read and write `/api/session` |
| US7 | US5: it renders the guidance |
| US7 | US6: it renders through `useSession` and rebuilds the `web/dist` that US6 rebuilt |
| US8 | US7: it drives the finished page |

One branch may run alongside another: **US6 and the US4 → US5 chain**, once US3 has
merged. US6 touches only
  `crates/finder/web/`; US4 and US5 touch only `crates/finder/src/`, `tests/` and the
  README.

Every server story edits `src/api.rs`, so the server stories form a chain. Both page
stories rebuild `web/dist`, so US7 follows US6.
