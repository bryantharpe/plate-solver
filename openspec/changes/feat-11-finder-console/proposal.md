## Why

The solver has parity tests and benchmarks, but nothing that lets its owner *see* it work: give
it a frame, see where it points, and see the proof on the frame itself. The owner is an amateur
astronomer with a 16-inch GoTo Dobsonian and a finder camera; the long-term aim is an electronic
finder that drives the mount, talks to SkySafari and lets an AI choose and steer to targets ("show
me something like M42"). This change specifies the low-barrier first step toward that: a local
console that plate-solves one frame and charts it, usable on a phone at the telescope (in a
red-on-black night mode) and on a laptop indoors. It is a consumer of the solver, not part of
the PRD's seven solver capabilities.

## What Changes

- Introduce the `finder-console` capability: the `finder` binary, which loads one pattern
  database and serves a JSON API and an embedded web page on the local network.
- The API solves a frame (an upload or a bundled sample) with cedar's decode and detection,
  and reports where it points (RA/Dec, constellation, roll, field, scale, residuals, timing),
  the matched and expected catalog stars, the Messier objects on and near the frame, and an
  RA/Dec graticule, all placed in the frame's pixels through the solver's own attitude and lens
  model. It also suggests objects like a given one, ranked for an observer's site and time.
- The page shows where the frame points first, then a field-atlas chart of the frame with the
  solution drawn over it, what is in the field, what to look at next, and (online only) the
  same field in a sky survey. It has a night mode in which every pixel is red on black.
- Tests for all of it: Rust unit and API tests on the real database and frames, page unit and
  component tests (Vitest), and end-to-end tests in Chromium against the release binary
  (Playwright), run in CI by a new `finder-page` job.

## Capabilities

### New Capabilities

- `finder-console`: a local console that plate-solves a frame and charts it: the solve API,
  sky geometry for the overlay and for "what is up", the Messier catalog and similar-object
  ranking, and the page, with its night mode and offline use.

### Modified Capabilities

(none.)

## Impact

- New crate `crates/finder` (binary and library) over `math-core`, `star-detection`,
  `pattern-database` and `plate-solver`, with `axum`, `tokio`, `image`, `rust-embed` and
  `clap`. It does not touch the solver crates.
- Bundled data: the 110 Messier objects (an OpenNGC extract, CC BY-SA 4.0), the IAU
  constellation boundaries and names (Roman 1987 via Astropy, BSD 3-Clause), and a list of
  named bright stars by HIP number (`crates/finder/data/README.md`).
- A Node toolchain for the page (React 19, TypeScript, Vite, Tailwind 4) in
  `crates/finder/web`. Its production build (`dist/`) is committed and embedded, so building
  the crate never needs Node; CI checks that the committed build matches the source.
- CI: a `finder-page` job (typecheck, format, unit and component tests, build freshness,
  end-to-end tests).
- Design records: `PRODUCT.md`, `DESIGN.md` and the Impeccable surface brief under
  `.impeccable/surfaces/`.
