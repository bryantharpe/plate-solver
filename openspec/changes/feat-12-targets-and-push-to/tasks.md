## 1. Session and API

- [ ] 1.1 Record the owner's approval of the changes to tested behaviour this proposal names (the four MODIFIED `finder-console` requirements and the `playwright.config.ts` servers); verified by the approval on the PR before any code lands
- [ ] 1.2 Add `finder::session` with the `Clock` trait, `SystemClock` and `FixedClock`, and `AppState::with_session`/`with_clock`, keeping `new`, `shared` and `router` as they are; verified by `cargo test -p finder` passing unchanged and a unit test that a fresh session has no site, no target and a 15° minimum
- [ ] 1.3 Add `GET`/`PUT /api/session` with checked, partial edits and unknown fields refused; verified by API tests for "What one device writes, another reads", "Values that are out of range", "A target the catalog does not hold" and "One field at a time"
- [ ] 1.4 Refuse foreign `Origin`s on `PUT /api/session` and `POST /api/solve`, and non-JSON edits; verified by API tests for "A page on another site", "The console's own page" and "A form post", with every existing API test still passing
- [ ] 1.5 Record the latest pointing from each solve (with `pointing_seq` on the report and the solved `Frame` kept); verified by API tests for "orion2 under a fixed clock", "A frame that does not solve" and "No site yet"
- [ ] 1.6 Add the state file (`--state`, the XDG default, atomic write, `.bad` on a bad file); verified by a CLI test that restarts the binary on one `--state` folder, a test with an unreadable file, and a test that the library writes nothing
- [ ] 1.7 Document `/api/session` and `--state` in the crate docs and `crates/finder/README.md`; verified by `cargo doc` with `-D warnings`

## 2. Target catalog

- [ ] 2.1 Extract `data/openngc.csv` from a pinned OpenNGC commit, documenting the recipe, the commit and the object count in `data/README.md` and adding its row to `THIRD_PARTY_NOTICES.md`; verified by "The object count" test and `licenses_prints_the_notices_and_exits`
- [ ] 2.2 Parse the catalog once, with canonical ids, normalised lookup and `Dup` aliases; verified by tests for "Ways to write the same object", "An id that is not there" and "A duplicate's id"
- [ ] 2.3 Check the Messier objects against `messier.csv`; verified by the "All 110" and "The disputed and doubled ones" tests
- [ ] 2.4 Add `GET /api/targets?q=&limit=` ranked as specified; verified by API tests for "Three ways to Andromeda", "Nothing by that name" and "No query"

## 3. Ephemeris

- [ ] 3.1 Add `finder::ephemeris` with the Sun (Meeus ch. 25); verified by the example 25.a test within 0.01°
- [ ] 3.2 Add rising, transit and setting of an altitude (Meeus ch. 15), with interpolation for moving positions; verified by the example 15.a test within 1 minute and the "Always above", "Never that high" and "A crossing is a crossing" tests

## 4. Push-to

- [ ] 4.1 Class targets (Sun guard, below the horizon, too low, pointable, no site); verified by tests for "A January evening", "A higher minimum altitude" and "No site"
- [ ] 4.2 Give rise, transit and set for targets that are down; verified by the "M8 at 15°" and "M8 at 30°" tests
- [ ] 4.3 Give turn and tilt from the held pointing to the moving target; verified by the "Across north", "The other way", "Not pointable" and "Ten minutes later" tests
- [ ] 4.4 Place the target on or off the latest frame; verified by the "A target on the frame" (M78) and "A target off the frame" (M79) tests
- [ ] 4.5 Add `similar_above()` and rank `/api/similar` by the session's minimum altitude; verified by the "Ranked for a higher minimum altitude" API test with `similar_objects_rank_for_an_observer` and the `similar()` unit tests unchanged and passing

## 5. Page

- [ ] 5.1 Add `useSession()` (poll every 5 s while visible, `put`) and adopt a stored browser site once; verified by Vitest tests for both, and App.test, settings.test and SiteDialog.test passing unchanged
- [ ] 5.2 Add the target sheet from the top bar; verified by Vitest tests for "Andromeda by name", "Nothing matches" and "Clearing the target"
- [ ] 5.3 Add the guidance block and the first-screen order with a target; verified by Vitest tests for "Directions to M78", "A target that is down", "Near the Sun" and "Changed on another device"
- [ ] 5.4 Draw the target on the chart (reticle or edge arrow, outside the legend); verified by Chart tests for "M78 on orion2" and "M79 off orion2", with the existing Chart tests passing unchanged
- [ ] 5.5 Add the Setup page at `/setup` (site and minimum altitude), and pass the minimum altitude to NextTargets; verified by Vitest tests for "Raising the minimum altitude", "A minimum altitude out of range" and "The minimum altitude raised", with NextTargets' tests unchanged
- [ ] 5.6 Rebuild `web/dist`; verified by `npm run typecheck`, `npm run format:check`, `npm test` and the build-freshness check

## 6. End to end

- [ ] 6.1 Give the existing Playwright server a fresh `--state`, and add a second server and a `session` project for the tests that write to the session; verified by `npm run e2e` running every existing test unchanged and green
- [ ] 6.2 Add end-to-end tests on the second server for "The first screen with a target", "Set on the phone, seen on the laptop" (two browser contexts), "Opened directly", and the night pixel check with a target set and on the Setup page; verified by `npm run e2e`

## 7. Design

- [ ] 7.1 Run the Impeccable pass (shape → craft → finish review) on the target sheet, guidance block, target mark and Setup page against `.impeccable/surfaces/crates-finder-web-src-setup-tsx.md` and the main surface brief; verified by a ship verdict from the finish review
- [ ] 7.2 Update `DESIGN.md` (and `.impeccable/design.json`) with the guidance block, target reticle, edge arrow and Setup page as built; verified by the finish reviewer's check against the shipped page
