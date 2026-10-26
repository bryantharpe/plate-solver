## Why

The finder console answers "what am I looking at": it plate-solves one frame and charts it. At
the telescope the next question is "where do I want to go, and how do I get there". This change
lets the observer choose a target from OpenNGC, learn whether it can be pointed at now, and get
push-to directions from where the last frame points, with no new hardware. It is the first of
four changes (feat-12 to feat-15) that end in GoTo and tracking, and it lays down the shared
observing session that live solving (feat-13), the mount and later clients will all use.

## What Changes

- An **observing session** held by the server and shared by every device: the site (typed
  once), the minimum altitude (default 15°), the target and the latest pointing, with the
  console's own clock as the time. `GET`/`PUT /api/session`, checked edits, same-origin
  writes, and a state file so it survives a restart.
- A **target catalog**: all of OpenNGC (NGC, IC and the addendum) bundled for offline use,
  searched by Messier number, NGC/IC id and common name (`GET /api/targets?q=`).
- **Sky ephemeris** in the `finder` crate: the Sun's position (Meeus ch. 25) and the rising,
  transit and setting of any altitude (Meeus ch. 15).
- **Push-to**: a target is classed as under the Sun guard, below the horizon, too low, or
  pointable; targets that are down get rise, transit and set times; pointable ones get "turn
  clockwise 34° · tilt up 12°" from the latest pointing; and the target is placed on the
  solved frame, or given as a direction from it.
- The page gains a target search sheet, a guidance block, the target on the chart (or an edge
  arrow toward it), and a Setup page at `/setup` holding the site and the minimum altitude.
- **Changes to tested behaviour, for the owner's approval** (each is a MODIFIED requirement
  below; no existing test changes):
  - the site moves from the browser into the session (a site this browser stored is adopted
    once);
  - "up" in suggestions means at or above the session's minimum altitude rather than a fixed
    15°, and "below 15°" shows the minimum altitude;
  - with a target set, the guidance block comes first and the roster moves below the chart.

## Capabilities

### New Capabilities

- `observing-session`: one night's shared state at the telescope, held by the console: site,
  minimum altitude, target, latest pointing and the console's time; its API, checks,
  persistence and same-origin rule.
- `target-catalog`: the bundled OpenNGC objects and the search that finds a target by Messier
  number, NGC/IC id or common name.
- `sky-ephemeris`: time-dependent sky geometry beyond what the finder console already has: the
  Sun's position, and when a position rises above, transits and sets below a given altitude.
- `push-to`: whether a target is pointable and why not, and the turn and tilt that bring the
  telescope from its latest pointing to the target, including the target's place on the frame.

### Modified Capabilities

- `finder-console`: the observing site lives in the session; "up" follows the minimum
  altitude; the first screen leads with the guidance block when a target is set; and the page
  adds the target search sheet, the guidance block, the target on the chart and the Setup page.

## Impact

- **Code:** `crates/finder` gains session, target-catalog, ephemeris and push-to modules and
  routes (`/api/session`, `/api/targets`), `--state` and `--allow-host` options, and an injectable clock.
  `AppState::new`/`shared`, `router`, `similar()`, `Observer` and `Frame` keep their shapes;
  `similar_above()` is added beside `similar()`. `crates/math-core` is untouched.
- **Data:** `crates/finder/data/openngc.csv`, an extract of OpenNGC at a pinned commit (about
  1 MB, CC BY-SA 4.0), documented in `data/README.md` with a row in `THIRD_PARTY_NOTICES.md`.
  The chart keeps drawing Messier objects only.
- **Page:** new components in `crates/finder/web` (search sheet, guidance block, target mark,
  Setup page) with no new npm packages; `web/dist` rebuilt.
- **Files the binary writes:** the session's state file, by default under
  `$XDG_STATE_HOME/finder/` (`~/.local/state/finder/`), moved with `--state`. The library keeps the session in memory, so
  the Rust tests write nothing.
- **Playwright (named for the owner's approval):** `web/playwright.config.ts` gains a second
  `finder` server on another port with its own throwaway `--state`, for the new tests that
  write to the session; the existing server is given a throwaway `--state` too, and both set
  `reuseExistingServer: false`, so a developer's own console on 8097 is never reused for the
  end-to-end tests (with `FINDER_URL` set, the tests run against that server's session). The
  default project still shares one session, so once a test that seeds a browser site has run,
  later tests in it see that site; no existing assertion depends on there being none. No
  existing test changes. CI (`ci.yml`) is unchanged.
- **Design records:** a surface brief for the Setup page under `.impeccable/surfaces/`, and a
  `DESIGN.md` update after the Impeccable pass.
