## 1. Server

- [x] 1.1 Create the `finder` crate: binary and library over `math-core`, `star-detection`, `pattern-database` and `plate-solver`; load one database and a samples folder; serve on `--listen`
- [x] 1.2 Health, samples (listing, files, previews) and the page's routes, with samples resolved by listed name only
- [x] 1.3 Solve endpoint: the multipart form, option checks and defaults, the status codes and limits, one solve at a time
- [x] 1.4 Decode and detect as cedar does (PIL gray, rows levelled, hot pixels rejected) and check every sample against cedar's goldens

## 2. Sky

- [x] 2.1 Project catalog positions into the frame with the solver's attitude, camera and distortion; north, east, field and scale
- [x] 2.2 The RA/Dec graticule with edge labels, including near the poles
- [x] 2.3 Precession, sidereal time and altitude/azimuth, tested on Meeus's worked examples
- [x] 2.4 Constellation lookup from Roman's (1987) boundaries at B1875, checked for all 88 constellations and the 110 Messier objects

## 3. Catalog

- [x] 3.1 Bundle the Messier objects (OpenNGC extract), the constellation tables and the named bright stars, with sources and licences in `data/README.md`
- [x] 3.2 Place objects on and near the frame at true size and angle; list those just off it with their direction
- [x] 3.3 `/api/similar`: same-family objects, nearest first, those up at the observer's site first

## 4. Page

- [x] 4.1 React, TypeScript, Vite and Tailwind in `web/`, built into `web/dist` and embedded in the binary
- [x] 4.2 Where it points first; the roster, chart and legend in a phone's first screen; two columns on a wide screen
- [x] 4.3 The chart: graticule, star rings, centroids, atlas symbols, compass, scale bar, legend toggles, tap for facts
- [x] 4.4 Label layout that never collides, with leader lines
- [x] 4.5 Frame picker, first run, no-match retry, field list, next targets, site dialog, survey view
- [x] 4.6 Night mode red on black, none brighter than the ink; day text at WCAG AA; offline except the survey

## 5. Design

- [x] 5.1 Product record (`PRODUCT.md`) and the surface brief (`.impeccable/surfaces/`)
- [x] 5.2 Impeccable finish review to a ship verdict (eight fixes and three follow-ups, two rounds)
- [x] 5.3 Record the design system as built (`DESIGN.md`, `.impeccable/design.json`)

## 6. Tests and CI

- [x] 6.1 Rust unit tests and API tests on the real database and sample frames
- [x] 6.2 Page unit and component tests (Vitest, jsdom) on reports captured from the server
- [x] 6.3 End-to-end tests in Chromium against the release binary (Playwright)
- [ ] 6.4 A `finder-page` CI job: typecheck, format, tests, build freshness and the end-to-end tests
