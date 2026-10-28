# Implementation Plan: Targets and Push-To

**Branch**: `012-targets-and-push-to` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Add an observing session, an OpenNGC target catalog, a sky ephemeris and push-to guidance
to the `finder` crate. Then add the page parts that use them: a target sheet, a guidance
block, a target mark on the chart and a Setup page. Then add end-to-end tests on a server
of their own.

Everything time-dependent runs on an injectable clock. `crates/math-core` is untouched.
No existing test changes.

## Technical Context

**Language/Version**: Rust, MSRV 1.96 (`rust-version`), stable in CI. TypeScript with
React 19 for the page.

**Primary Dependencies**:
- already in `crates/finder`: axum, serde and clap;
- the page: Vite, Vitest and Playwright.

No new crate and no new npm package is expected. A new crate would be a dependency change
on a critical path: it must build on 1.96, pass `cargo-deny` and `cargo-machete`, and be
justified in its story.

**Storage**: one JSON state file, `session.json`, held by the binary only. The library
holds the session in memory.

**Testing**:
- `cargo test -p finder --locked`: unit tests in-module, and integration tests in new
  files under `crates/finder/tests/`.
- Vitest and Playwright in `crates/finder/web`.

**Target Platform**: the `finder` binary on desktop Linux, WSL and Raspberry Pi, and the
page on Android phones and laptops.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, and the 80% coverage
  floor.
- `web/dist` is committed and must be the build of its source.
- The night mode is red only.

**Scale/Scope**: about 13,000 catalog objects, one observer and a handful of devices.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing here touches the solver or `crates/math-core`. Positions come from the existing `sky.rs` functions, and new numerics are checked against Meeus's worked examples. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The four behaviour changes are owner-approved (spec § Owner approvals). |
| III. Tests first, independent expectations | Every scenario names its oracle: a Meeus example, OpenNGC, `messier()`, `sky::alt_az`, the solve report's own marks, or a fixed clock. |
| IV. Critical paths | `src/api.rs` changes the HTTP surface, not `proto/`. No `unsafe`, no database-format change, and no dependency change is planned. |
| V. Standalone Rust, honest licensing | OpenNGC is data under CC BY-SA 4.0. It is credited in `THIRD_PARTY_NOTICES.md` and held there by a test (FR-008). |

## Project Structure

```text
crates/finder/
├── data/openngc.csv          # PREREQUISITE (maintainer): the OpenNGC extract
├── data/README.md            # PREREQUISITE: recipe, pinned commit, object count
├── THIRD_PARTY_NOTICES.md    # PREREQUISITE: OpenNGC row (CC BY-SA 4.0, commit)
├── README.md                 # US3, US4: /api/session, --state, --allow-host
├── src/
│   ├── lib.rs                # US1, US2, US3, US5: `pub mod` lines only
│   ├── ephemeris.rs          # US1 (new)
│   ├── targets.rs            # US2 (new); US5 adds the class to search results
│   ├── session.rs            # US3 (new); US4 adds the pointing; US5 adds guidance
│   ├── pushto.rs             # US5 (new)
│   ├── api.rs                # US2 (/api/targets); US3 (/api/session, builders);
│   │                         # US4 (origin/host check, pointing on solve); US5 (similar)
│   ├── solve.rs              # US4: a sibling of solve_frame that also returns Frame
│   ├── catalog.rs            # US5: similar_above()
│   └── main.rs               # US3 (--state); US4 (--allow-host)
├── tests/
│   ├── targets_api.rs        # US2 (new)
│   ├── notices.rs            # US2 (new)
│   ├── session_api.rs        # US3 (new)
│   ├── session_cli.rs        # US3 (new)
│   ├── session_writes.rs     # US4 (new)
│   ├── pointing_api.rs       # US4 (new)
│   └── pushto_api.rs         # US5 (new)
└── web/
    ├── src/useSession.ts (+ .test.ts)            # US6
    ├── src/components/SetupPage.tsx (+ test)     # US6
    ├── src/App.tsx, TopBar.tsx, NextTargets.tsx  # US6 (routing, Setup link, minAlt)
    ├── src/components/TargetSheet.tsx (+ test)   # US7
    ├── src/components/GuidanceBlock.tsx (+ test) # US7
    ├── src/components/TargetMark.tsx (+ test)    # US7 (a Chart layer, outside the legend)
    ├── src/App.target.test.tsx                   # US7
    ├── dist/                                     # US6, US7: rebuilt
    ├── playwright.config.ts                      # US8 (owner approval 4)
    └── e2e/session.spec.ts                       # US8 (new)
DESIGN.md, .impeccable/design.json                # US7
```

## Interfaces

**`GET /api/session`** answers:

```text
{ site: {lat_deg, lon_deg} | null, min_alt_deg, target: <catalog object> | null,
  pointing: {seq, frame, ra_deg, dec_deg, alt_deg?, az_deg?, time} | null,
  time, guidance?: {class, rise?, transit?, set?, never?, turn_deg?, tilt_deg?, placement?} }
```

- `pointing` arrives with US4 and `guidance` with US5.
- `class` is one of `sun_guard`, `below_horizon`, `too_low`, `pointable` or `no_site`.
- `placement` is either `{on_frame: true, x, y, axes, angle}` or
  `{on_frame: false, direction_deg, distance_deg}`.

**`PUT /api/session`** takes `{site?, min_alt_deg?, target?}` with `deny_unknown_fields`,
where `null` clears `site` or `target`. It answers the new session.

**`GET /api/targets?q=&limit=`** answers a list of catalog objects, each carrying a
`class` from US5 on.

**The solve report** gains an optional `pointing_seq`. Old page fixtures lack it and read
as "not the latest pointing", which is correct.

## Shapes that existing tests pin

These are not changed by any story:
- **Rust signatures:** `AppState::new`, `AppState::shared`, `router` and `similar()`.
- **Rust structs:** the fields of `Observer` and `Frame`, which tests build as struct
  literals.
- **Page components:** the legend's five chips; `useSite` and SiteDialog's props;
  NextTargets' `getSimilar` URL.
- **Page test data:** the page's fixtures, so every new report field is optional; and
  App.test's fetch mock, which answers every unknown route with the health body.
- **Geometry:** `sky::alt_az` stays geometric, as the Meeus 13.b test pins. Refraction
  arrives with feat-14.

## Decisions and the alternatives rejected

**An injectable clock, not a global or `#[cfg(test)]` clock.** Integration tests in
`tests/` cannot see `cfg(test)` items, and a global clock makes parallel tests interfere.
`FixedClock` is public, but no binary option builds one. Only feat-13's `--simulate`
reaches a virtual clock.

**One poll for the session and its guidance, not a separate `/api/guidance`.** Two polls
cost twice as much and can disagree for a moment.

**Meeus's method for rise and set, not stepping `alt_az` and bisecting.** Stepping is
slower, and Meeus's worked example would no longer check the method.

**A separate OpenNGC catalog, with the Messier list left alone.** Deriving the Messier
list from OpenNGC would change the chart's data source under the orion2 and Virgo tests
for no gain. The chart keeps drawing Messier objects only.

**The Sun guard at mean precision.** About 20″ of aberration and nutation is nothing
against the guard's 15°, so the guard compares the target precessed to the date with the
Sun's apparent place.

**The same-origin rule keeps the network setups that matter working.** A phone reaching
the console by LAN IP passes, and so does `netsh portproxy` or WSL mirrored networking,
because each keeps its own `Host`. The JSON content type that axum's `Json` extractor
enforces stops the "simple" cross-site requests a browser sends without a preflight. The
Origin check also covers `/api/solve`, whose multipart body is one of those. A route that
only stops the telescope (feat-14's `POST /api/stop`) will be exempt, because a stop must
never be refused.

## User Story 1 — Sky ephemeris

The module is `finder::ephemeris`, which is pure. It lives in `finder`, not
`crates/math-core`, because math-core is the solver's parity path.

**The Sun.** Meeus ch. 25's low-accuracy series, with the apparent correction for
longitude.

**Rise, transit and set.** Meeus ch. 15:
- a first estimate from the hour angle of the altitude;
- three corrections each for transit, rising and setting;
- mean sidereal time via `sky::gmst_deg`, so a computed rising is a crossing by
  `alt_az`'s own measure.

The input is either a fixed position or three daily positions to interpolate. The
date-bound result is m0, m1 and m2, or `always_above` / `never_rises` when cos H₀ falls
outside ±1. The transit is given either way.

**"Next" times.** The rising is searched over today's and tomorrow's date-bound times
within 24 sidereal hours. The transit and setting are the first after that rising.

**Rejected alternative:** stepping `alt_az` and bisecting. It is slower, and Meeus's
worked example would no longer check the method.

**Tests.** All in-module. Scenario 7 is a property of the diff.

## User Story 2 — Target catalog

The module is `finder::targets`. The extract it reads, `crates/finder/data/openngc.csv`,
is already on `main` (see § Prerequisites). It is semicolon-separated with a header row:

```text
name;type;ra_deg;dec_deg;const;major_arcmin;minor_arcmin;pa_deg;mag;messier;common_names;dup_of
```

- `name` is OpenNGC's spelling (`NGC0224`, `IC0434`, `Mel022`).
- `messier` is `M31` or empty.
- `common_names` is comma-separated.
- A `Dup` row has only `name`, `type` (`Dup`) and `dup_of`, naming the object it
  duplicates.

`data/README.md` records the object count (13372 objects and 651 aliases) that US2-S1
asserts.

**Loading.** `include_str!("../data/openngc.csv")` is parsed once into a `OnceLock`, as
`catalog::messier()` is.

**Ids.**
- The canonical id is `catalog::designation()`'s form, or the Messier number for
  Messier-only addendum rows.
- `Dup` rows become aliases pointing at the object OpenNGC names.
- Lookup normalises case, a space before the number, and leading zeros.

**Search.** `GET /api/targets?q=&limit=` ranks exact ids first, then common-name word
prefixes, brightest first, with no magnitude last. `limit` defaults to 20 and is clamped
to 1–50. An empty or missing `q` answers 400 with the console's `{"error": ...}` body.

**Not affected.** `messier.csv` and `messier()` stay as they are, because the chart and
`/api/similar` keep using them. US2's test holds the two catalogs together within 1″.

## User Story 3 — Observing session

The module is `finder::session`.

**The session and its clock.**
- `Session` holds the site, minimum altitude and target behind a mutex, with an optional
  state-file path.
- `Clock` is a trait with `now() -> f64` (Unix seconds), implemented by `SystemClock` and
  `FixedClock`. `FixedClock` is public for integration tests, but no CLI option builds
  one.

**`AppState`.** It gains `with_session(Session)` and `with_clock(Arc<dyn Clock>)`, in the
style of `with_form_timeout`. `new` and `shared` keep their signatures and default to an
in-memory session on the system clock.

**Routes.**
- `GET /api/session` answers the session.
- `PUT /api/session` takes `{site?, min_alt_deg?, target?}` with `deny_unknown_fields`;
  `null` clears a field. It answers the new session. Errors use the console's
  `{"error": ...}` body.
- The route extracts axum's `Json`, which already answers 415 for a wrong content type.
  US4's tests assert that.

**The state file.**
- `--state DIR`, defaulting to `$XDG_STATE_HOME/finder`, else `~/.local/state/finder`.
- The file is `session.json`, holding the site, minimum altitude and target only.
- Writes are atomic: `session.json.tmp`, synced, then renamed.
- A bad file is renamed to `session.json.bad`, with a line on stderr.

**Tests.**
- CLI tests start the built binary, as `tests/cli.rs` does, on a temporary `--state`.
- The XDG scenario sets `XDG_STATE_HOME` for the child process only.

## User Story 4 — Write protection and the latest pointing

**The origin and host check.** One check guards `PUT /api/session` and `POST /api/solve`:
- an `Origin` whose host and port differ from `Host` is refused 403;
- an absent `Origin` is allowed, which keeps curl and the Rust tests working;
- when `Origin` is present, `Host` must be an IP literal, `localhost`, or an
  `--allow-host` name.

feat-14's `POST /api/stop` will be exempt; nothing in this spec adds it.

**The pointing on solve.** `solve_frame` keeps its signature. A sibling returns the
report together with the solved `Frame`, and `solve_frame` calls it. After a frame solves,
the handler records the pointing:
- `seq`;
- the `Frame`;
- the J2000 centre;
- the clock's time;
- alt/az from `sky::alt_az` when a site is set.

The report gains an optional `pointing_seq`.

**The pointing follows the site.** The site setter in `Session` recomputes alt/az when the
site changes, and drops them when it is cleared.

## User Story 5 — Push-to

The module is `finder::pushto`.

**Class.** The precedence is Sun guard, then below the horizon, then too low, then
pointable.
- The Sun guard compares the target, precessed to the date (`sky::precess_from_j2000`),
  with `ephemeris::sun` at the console's time.
- With no site, the class is `sun_guard` or `no_site`.

**Times for a target that is down.** `ephemeris` next-times at the minimum altitude.

**Turn and tilt.**
- turn = `((Δaz + 180) mod 360) − 180`, with exactly −180 mapped to +180;
- tilt = Δalt.

They are computed from the pointing's alt/az held at its frame time to the target's
alt/az now.

**Placement.** It reuses `object_marks`' code path for one object, so a Messier target's
placement equals its report mark by construction. Off the frame, it reuses the
nearby-object computation (`project_pinhole`, `direction_deg` and the angular distance).

**Guidance on read.** `GET /api/session` adds `guidance`, derived on each read.
`/api/targets` results gain `class`.

**Similar objects.** `catalog::similar_above(reference, observer, limit, min_alt_deg)`
holds `similar()`'s body, and `similar()` calls it with 15°. `/api/similar` passes the
session's minimum altitude.

## User Story 6 — The page shares the session; the Setup page

**`useSession()`.**
- It polls `GET /api/session` every 5 s while `document.visibilityState === "visible"`,
  and once after each `put`.
- Missing fields read as unset, so App.test's mock, which answers `/api/session` with the
  health body, is an empty session.

**Adopting the browser's site.** On first load, with no session site and a `useSite()`
site present, the page puts that site, then clears the browser's copy through `useSite`'s
own setter. `useSite` and SiteDialog keep their shapes.

**The Setup page.**
- `SetupPage` sets the site (typed, or from the device's location) and the minimum
  altitude (0–80).
- `App` routes on `location.pathname` with `history.pushState`, and the server already
  falls `/setup` back to `index.html`.
- `TopBar` gains a Setup link.
- `NextTargets` gains an optional `minAlt` (default 15).

**Last step.** Rebuild `web/dist`.

## User Story 7 — Target sheet, guidance block, target mark

**Components.**
- `TargetSheet` follows FramePicker's dialog pattern.
- `GuidanceBlock` uses ink only, because object colours are for objects. Times are shown
  in the device's zone, to the minute.
- `TargetMark` is a Chart layer outside the legend, drawn only when the report's
  `pointing_seq` equals the session's. It is a reticle round the family symbol, or an edge
  arrow reusing `DirectionArrow`.

**Layout.** `App` puts the guidance block and the title block first when a target is set.

**Design records.** `DESIGN.md` and `.impeccable/design.json` gain the four new parts as
built. The reticle's exact drawing (corner ticks or a broken circle) is chosen within
DESIGN.md's atlas language; the maintainer's Impeccable pass may refine it after landing.

**Last step.** Rebuild `web/dist`.

## User Story 8 — End to end

**`playwright.config.ts`.**
- The existing server starts with `--state` in a fresh folder under `target/`, and with
  `reuseExistingServer: false`.
- A second server on 8098 has its own fresh `--state` and serves a `session` project.
- The default project ignores `e2e/session.spec.ts`.
- With `FINDER_URL` set, the tests run against that server.

**`e2e/session.spec.ts`.** It covers the first screen with a target at 412 × 915 and
390 × 844, two contexts sharing a site, `/setup` opened directly, and the night pixel
check with a target and on the Setup page.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check` and
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked` and
    `cargo test --workspace --locked --doc`.
- **Rust stories** (US1–US5) also run
  `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps --all-features`.
- **Page stories** (US6, US7), in `crates/finder/web`: `npm ci`, `npm run typecheck`,
  `npm run format:check` and `npm test`. Then `npm run build`, and confirm that
  `git status` shows `dist/` matching the rebuild (CI's freshness check).
- **US8** also runs `npm run e2e`, which builds the release binary.
- **CI in the merge queue** runs all of the above and more: coverage at 80%, the cedar
  e2e eval and canary, msrv, docs, deny, machete and the secret scan. A story is not done
  until it would pass them.

## Prerequisites before this spec is flipped to `ready`

1. **The OpenNGC extract is on `main`.** Done: landed in #109 (10f3e6d).
2. **The owner approvals are given.** Done: approved on 2026-10-28 (spec § Owner
   approvals).
3. **The repository is wired for the Ergane merge queue.** Done: `ergane init --check`
   passes all 15 checks.
4. **The coding agent can run cargo.** Ergane spec 261 (an agent can build what its gates
   build) has landed, and the worker has been redeployed at a build that includes it.

## Risks

| Risk | Mitigation |
|---|---|
| A page story passes the cargo gates but fails `finder-page` in the queue. | Each page story's tasks run the npm checks before proposing. The queue failure is reported back to the node's ladder. |
| The OpenNGC extract grows the binary by about 1 MB. | Accepted: the pattern database dwarfs it. |
| Last write wins between devices. | One observer and a few edits a night. The mutex keeps each edit whole, and the 5 s poll shows the result. |
| The Sun guard depends on the console's clock. | A wrong clock misplaces the Sun by 1° per 4 minutes of error, inside the 15° margin for any plausible error. |
| An unreadable state file loses the night's site and target. | It is kept as `.bad`, and both take seconds to re-enter. |
