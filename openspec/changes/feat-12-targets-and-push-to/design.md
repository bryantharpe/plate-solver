## Context

See proposal.md for why. The finder console (`crates/finder`) is an axum server over one
pattern database, with a React page embedded from `web/dist`. It already has the sky geometry
this change needs: `sky.rs` (`alt_az`, `precess_from_j2000`, `gmst_deg`, `julian_date`,
`Frame::project`, `project_pinhole`, `direction_deg`, `north_at`), the Messier catalog and
`similar()` in `catalog.rs` (`Family`, `kind_of`, `designation`), and the `object_marks`
placement in `solve.rs`. On the page it has the FramePicker sheet, NextTargets'
`DirectionArrow`, SiteDialog, `settings.ts`' `usePersistent`, the Chart's layers and
`labels.ts`. All of these are reused.

Existing tests pin some shapes, and nothing here changes them:
- the signatures of `AppState::new`/`shared`, `router` and `similar()`;
- the fields of `Observer` and `Frame`, which tests build as struct literals;
- the legend's five chips; `useSite` and SiteDialog's props; NextTargets' `getSimilar` URL;
- the page's fixtures (any new report field is optional) and App.test's fetch mock, which
  answers every unknown route with the health body;
- `alt_az` stays geometric (the Meeus 13.b test).

`crates/math-core` is the parity path and is untouched.

## Goals / Non-Goals

**Goals:**
- One shared observing session, with every time-dependent answer on an injectable clock.
- Push-to from the latest solved frame, with the Sun guard always on.
- Keep every existing Rust, Vitest and Playwright test valid as written.

**Non-Goals:**
- Refraction and apparent altitude (feat-14); the aim point's calibration (feat-13). Until
  then the aim point is the frame centre.
- Drawing NGC/IC objects on the chart. It keeps drawing Messier objects only, because adding
  others would change what the orion2 and Virgo tests see.
- Typed RA/Dec, named stars, the Moon and planets as targets; a horizon that varies by
  direction.

## Decisions

### The session lives in `finder::session`, behind an injectable clock

`Session` holds the site, minimum altitude, target id and latest pointing behind a mutex, with
an optional state file. `Clock` is a small trait (`now() -> f64` Unix seconds) with
`SystemClock` and `FixedClock`. `AppState` gains builder methods in the style of
`with_form_timeout`: `with_session(Session)` and `with_clock(Arc<dyn Clock>)`. `new` and
`shared` keep their signatures and default to an in-memory session on the system clock, so
`tests/api.rs` and every existing caller compile and write nothing.

`FixedClock` is public so integration tests can use it, but the binary has no option that
builds one. The virtual clock that feat-13 adds is reachable only through `--simulate`.

*Alternative:* a global clock or `#[cfg(test)]` clock. Rejected: integration tests in
`tests/` can't see `cfg(test)` items, and a global makes parallel tests interfere.

### One poll carries the session and its guidance

`GET /api/session` answers `{site, min_alt_deg, target, pointing, time, guidance}`.
- `target` is the catalog object.
- `pointing` is the latest pointing: `{seq, frame, ra_deg, dec_deg, alt_deg?, az_deg?, time}`.
- `guidance` is derived at read time on the console's clock: `{class, rise?, transit?, set?,
  never?, turn_deg?, tilt_deg?, placement?}`.

So the page polls one route every 5 s (and once after each edit).

`PUT /api/session` takes `{site?, min_alt_deg?, target?}` with serde's `deny_unknown_fields`;
`null` clears site or target. It answers the new session. The route extracts axum's `Json`,
which already answers 415 for a missing or wrong content type. Its errors are mapped to the
console's `{"error": ...}` body.

*Alternative:* a separate `/api/guidance` route. Rejected: two polls, and they can disagree
for a moment.

### The solve sets the pointing, and keeps its frame for placement

The `/api/solve` handler, after a frame solves, records the pointing:
- the J2000 centre (the aim point until feat-13);
- alt/az from `alt_az` at the clock's now, when a site is set;
- the clock's time;
- a sequence number.

It also keeps the solved `Frame`, so the target can be placed without solving again.
`solve_frame` keeps its signature. A sibling returns the report together with the `Frame`,
and `solve_frame` calls it.

The report gains an optional `pointing_seq`, so the page knows whether the frame it charts is
the latest pointing. Old fixtures lack the field and read as "not the latest", which is right.

### Pointable, rise/set, turn/tilt and placement in `finder::pushto`

**Class.** Precedence is Sun guard, then below the horizon, then too low, then pointable.
- The Sun guard compares the target, precessed to the date, with the Sun's apparent place from
  `finder::ephemeris`. 20″ of aberration and nutation is nothing against 15°.
- Without a site, only the Sun guard can be decided, so the class is `sun_guard` or `no_site`.

**Turn.** `((Δaz + 180) mod 360) − 180`, with exactly −180 mapped to +180, so a half-turn goes
clockwise.

**Placement.** It reuses `object_marks`' code path for one object, so a Messier target's
placement equals its report mark by construction. Off the frame it reuses the nearby-object
computation (`project_pinhole`, `direction_deg`, angular distance).

### `finder::ephemeris`: the Sun and rise/transit/set, in `finder`, not math-core

**The Sun** uses Meeus ch. 25's low-accuracy series, with the apparent correction for
longitude.

**Rise, transit and set** use Meeus ch. 15:
- a first estimate from the hour angle of the altitude;
- then three corrections each for transit, rising and setting;
- mean sidereal time, as `alt_az` uses, so a computed rising is a crossing by `alt_az`'s own
  measure;
- a position may be a fixed point or three daily positions to interpolate (Meeus's Venus
  test, and the Sun later).

"Next" rising means at or after now, searched over the next 24 sidereal hours. The answer
names `always_above` or `never_rises` when cos H₀ falls outside ±1, and still gives the
transit.

*Alternative:* step `alt_az` through the day and bisect. Rejected: slower, and the worked
example would no longer check the method.

### The target catalog: `data/openngc.csv`, beside `messier.csv`

The extract follows `data/README.md`'s recipe for `messier.csv`, at a pinned OpenNGC commit:
- **Rows:** every row of `NGC.csv` and `addendum.csv`; `NonEx` dropped; `Dup` rows kept only as
  aliases, pointing at the object OpenNGC names.
- **Columns:** name, type, RA and Dec in decimal degrees, constellation, axes, position angle,
  magnitude (V, falling back to B), Messier number, and the common names.

That is about 1 MB, `include_str!`'d and parsed once into a `OnceLock`, as `messier()` is.

**Ids.**
- The canonical id is `designation()`'s form (`NGC 224`, `Mel 22`), or the Messier number for
  the Messier-only addendum rows.
- Lookup normalises case, a space before the number, and leading zeros.
- Search ranks exact ids first, then common-name word prefixes by magnitude (none last).

The Messier catalog (`messier.csv`, `messier()`) stays as it is. The chart and `/api/similar`
keep using it, and a test checks the two agree within 1″.

*Alternative:* derive the Messier list from the new extract. Rejected: it would change the
chart's data source under the orion2 and Virgo tests for no gain.

### `similar_above()` and the minimum altitude

`similar_above(reference, observer, limit, min_alt_deg)` holds today's body, with
`MIN_ALTITUDE_DEG` replaced by the argument. `similar()` calls it with 15°, so its unit tests
stand. `/api/similar` passes the session's minimum altitude, which is 15° on a fresh server,
so `similar_objects_rank_for_an_observer` stands too. NextTargets' URL stays the same; the
server knows the minimum.

### Persistence: one small JSON file, written atomically

**Where:**
- `--state DIR`, defaulting to `$XDG_STATE_HOME/finder`, else `~/.local/state/finder`;
- the file is `session.json`, holding site, minimum altitude and target only; a pointing is
  stale after a restart, so it isn't kept.

**How:**
- each edit writes `session.json.tmp` in the same folder, syncs it and renames it over the old
  file;
- an unreadable file is renamed to `session.json.bad`, with a line on stderr, and the session
  starts fresh.

### Same-origin writes

A small check runs on every route that changes the session: `PUT /api/session` and
`POST /api/solve`, which sets the pointing.
- It compares the `Origin` header's host and port with the request's `Host`.
- It answers 403 on a mismatch and lets an absent `Origin` through, so curl and the Rust tests
  are unaffected.

A phone through `netsh portproxy` or mirrored networking keeps its own `Host`, so it passes.
A route that only stops the telescope (feat-14's `POST /api/stop`) is exempt, because a stop
must never be refused.
The JSON content type is enforced by the extractor (above). That stops the "simple" requests
(form posts and `text/plain` fetches) that a browser sends cross-site without a preflight. The
Origin check covers `/api/solve`, whose multipart body is itself one of those.

### The page

**Data.**
- `useSession()` polls `GET /api/session` every 5 s while the page is visible, and exposes
  `put(edit)`.
- Missing fields read as unset, so App.test's mock (which answers `/api/session` with the
  health body) is an empty session.
- Polling is short GETs, so the offline end-to-end test still reaches `networkidle`.

**Adopting the browser's site.**
- On first load, if the session has no site and `useSite()` holds one, the page puts it and
  then clears the browser's copy through `useSite`'s own setter. That is why it happens only
  once.
- `useSite` and SiteDialog keep their shapes. SiteDialog's `site`/`onSave` now carry the
  session's site.

**Components.**
- TargetSheet, built on FramePicker's dialog pattern.
- GuidanceBlock, which uses ink only, since object colours are for objects.
- TargetMark, a Chart layer outside the legend, drawn only when the report's `pointing_seq`
  equals the session's. It is a target reticle round the family symbol, or an edge arrow
  reusing `DirectionArrow`.
- SetupPage.
- NextTargets gains an optional `minAlt` prop (default 15), so its tests stand.

**Routing.** No new package. `App` reads `location.pathname` and uses `history.pushState`;
`/setup` already falls back to `index.html` on the server.

**Times.** Shown in the device's local time zone, to the minute.

### End-to-end tests

The existing Playwright server writes nothing a developer cares about only if it has its own
state, so:
- `playwright.config.ts` starts it with `--state` in a fresh folder under `target/`;
- a second `finder` server on port 8098, with its own fresh `--state`, serves a new `session`
  project for the tests that set targets;
- the default project ignores those tests, so the existing ones still see a session with no
  target.

The proposal names this config change for the owner's approval.

## Risks / Trade-offs

- **The whole of OpenNGC enlarges the binary by about 1 MB.** Acceptable for a desktop and Pi
  binary; the database dwarfs it.
- **Last write wins between devices.** One observer, a handful of edits a night. The mutex keeps
  each edit whole, and the 5 s poll shows the result everywhere.
- **The guidance trusts the latest pointing whatever its age** (frames are uploaded by hand in
  this change). The block shows the age; feat-13 adds the 5 s freshness rule for live frames.
- **Local time on the device may differ from the site's zone** when the console is used
  remotely. The times are labelled with the device's zone.
- **An unreadable state file loses the night's site and target.** It is kept as `.bad` for
  inspection, and both take seconds to re-enter.
- **The Sun guard depends on the console's clock.** WSL follows Windows' clock; a wrong clock
  would misplace the Sun by 1° per 4 minutes of error, inside the 15° margin for any plausible
  error.

## Migration Plan

Nothing to migrate on the server. A browser that stored a site before this change hands it to
the session once, then forgets its own copy. Rolling back means deleting the state file;
older pages ignore the new routes.

## Open Questions

- Which OpenNGC commit to pin. The newest when the extract is made; the extract's README
  records it and the object count.
- The target reticle's exact drawing (corner ticks or a broken circle). The Impeccable pass
  settles it within DESIGN.md's atlas language.
