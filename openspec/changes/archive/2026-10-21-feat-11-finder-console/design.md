## Context

`finder-console` is the first thing in this repository a person uses directly. The solver
crates answer "where does this frame point"; the console answers it for one observer, with
proof, in a form they can read at the eyepiece on a phone and at a desk on a laptop. Its owner
plans to grow it into an electronic finder: a live camera, a SkySafari link (SkySafari connects
to telescopes as an LX200 device), GoTo control through the mount's SynScan controller, and an
AI that picks targets and steers. None of that is in this change, but its shape is: one
process that holds the pointing state and speaks JSON, with the page as one client of it.

The product record is `PRODUCT.md`; the visual system is `DESIGN.md`; the page's direction
contract is `.impeccable/surfaces/crates-finder-web-src-app-tsx.md`.

## Goals / Non-Goals

**Goals:**
- Solve a frame (an upload or a bundled sample) and give cedar's answer on cedar's frames.
- Show where it points first, then the proof: the solution drawn over the frame as a star
  atlas draws a chart (graticule, matched stars, Messier objects at true size and angle).
- Say what is in and just off the field, and what to look at next for the observer's site.
- Work at the telescope: a night mode with nothing but red on black, and no internet needed.
- Specify and test every behaviour: Rust tests on the real database and frames, page tests in
  jsdom, and end-to-end tests in a real browser against the real binary, all in CI.

**Non-Goals:**
- Live camera input, mount control, the SkySafari link and AI steering (later changes).
- Authentication, multiple users, or serving beyond the local network.
- A packaged mobile app (`mobile-runtime` covers on-device embedding).
- Catalogs beyond the Messier list and a short list of named bright stars.

## Decisions

- **One binary, a JSON API and an embedded page.** `finder` loads one pattern database at
  start and serves `/api/*` and the page with axum. The page's production build (`web/dist`)
  is committed and embedded with rust-embed, so `cargo build` and the Rust CI jobs never need
  Node; a `finder-page` CI job rebuilds it and fails when the committed build is stale. The API
  is the seam the later pieces (SkySafari bridge, mount control, an AI's tools) will use.
- **The server places everything; the page only draws.** Star marks, object outlines, the
  graticule and the off-frame directions are computed in Rust from the solver's own rotation,
  camera model and radial distortion, and sent in image pixels. One implementation of the sky
  geometry, tested against the solution itself (each matched star's catalog position must land
  on its detected centroid within the solve's residual), and a page that cannot drift from it.
- **Decode and detect as cedar does.** Color frames become gray with PIL's `convert('L')`
  weights (ITU-R 601-2), not `image`'s Rec. 709 `to_luma8`, and detection levels rows and
  rejects hot pixels, as cedar's goldens were made. With the `solve_image` example's settings
  of the time instead, color frames moved faint stars (orion_belt matched 21 of cedar's 24
  stars; pleiades' field was 16.6″ off). With cedar's, 19 of the 20 solvable samples land within
  0.3″ of cedar and the last within 4.2″. Both live in the solver crates
  (`star_detection::gray`, `DetectParams::cedar()`) rather than in the console, and
  `solve_image` uses them too, so the two tools cannot give different answers for one frame.
  `DetectParams::default()` stays as it was, so no existing caller changes.
- **Bounded work per request.** One decode-and-solve at a time behind a semaphore, so a burst
  queues instead of exhausting memory. A request takes the semaphore before its form is read:
  a queued upload waits unread on the network, and only one is ever held in memory. The cost
  is that a slow client holds everyone else up, so the form must arrive within 30 s of its turn
  or the request is answered 408. Request bodies up to 32 MiB; decoded images up to 20,000 px a
  side and 256 MiB; solve timeouts clamped to 30 s. Samples resolve by listed name only, so no
  request reaches another file.
- **Sky geometry from standard references, tested on their worked examples.** Precession with
  the IAU 1976 angles (Meeus ch. 21), mean sidereal time (Meeus eq. 12.4) and altitude/azimuth
  (Meeus ch. 13), without nutation or refraction: within a few hundredths of a degree, enough
  to say what is up. The constellation comes from the IAU boundaries as tabulated by Roman
  (1987), searched at B1875 after precession.
- **"Something like this" is a rule for now.** Objects of the same family (diffuse nebulae,
  planetaries, open and globular clusters, galaxies; the Crab borrows planetaries and nebulae),
  excluding anything within 1° of the reference (M43 beside M42 is the same view), nearest
  first; with a site, those at least 15° up come first. An AI can replace the ranking later
  behind the same endpoint.
- **The page: React, TypeScript, Vite and Tailwind.** The owner ruled out plain HTML/CSS/JS
  and delegated the framework; React's ecosystem suits the later camera view, mount controls
  and assistant panel. Fonts are self-hosted, so the page loads nothing from the internet.
- **A field-atlas design with a red-torch night edition** (`DESIGN.md`). Night is its own set
  of tokens, every one on the red channel at or below the night ink (`#e00000`); photographs
  pass through a luminance-to-red filter scaled to land just under the ink; a script and an
  inline style set the black ground before the first paint. Tests enforce the tokens, and an
  end-to-end test counts pixels off the red channel in real screenshots.
- **Labels are laid out, not placed.** Each name goes beside its mark where it covers nothing
  drawn (other labels, star rings, object outlines, centroids, the compass, scale bar and
  reticle), past the symbol's true reach in that direction; further out on a leader line when
  the near spots are taken. Objects are always named; a star whose name has no room goes
  without. Measured in the page's own font.
- **The survey view is the one online extra, and only on request.** Aladin Lite (pinned) shows
  a DSS image of the same field with the frame outlined. It downloads only when asked and says
  plainly when there is no connection.

## Risks / Trade-offs

- [No authentication] → the default listen address is loopback; `--listen 0.0.0.0:8080`, for a
  phone on the same Wi-Fi, is an explicit choice documented in the README. The API only reads
  bundled files and solves uploads, within the limits above.
- [Committed build can go stale] → CI rebuilds the page and fails on any difference.
- [API shapes are duplicated in `web/src/api.ts`] → the page's tests run on reports captured
  from the real server, and the end-to-end tests run the page against the real binary.
- [Crowded fields at phone width] → on a frame like the Virgo cluster (fifteen galaxies across
  380 px) every object is still named and no labels overlap, but an object label may cover a
  faint star ring; star names give way instead.
- [The survey view depends on a third party and a pinned beta of Aladin Lite] → it is optional,
  loaded on request, and the rest of the page works without it.
- [Bundled data licences] → the Messier extract keeps OpenNGC's CC BY-SA 4.0; the boundary and
  name tables keep Astropy's BSD 3-Clause; three misspellings in Astropy's names file are
  corrected and recorded in `crates/finder/data/README.md`.
- [The page's build strips its packages' notices] → `THIRD_PARTY_NOTICES.md` lists each
  package, font and data table compiled in, with its full license text, and `finder --licenses`
  prints it. A test fails when a listed package's locked version or license drifts from its
  notice; adding a new package to the page still needs its notice added by hand.
- [A client on a poor connection holds the solve gate] → it is cut off 30 s into its turn, which
  a 32 MiB upload over a slow phone link could exceed; such a frame should be shrunk first.

## Migration Plan

New crate; nothing existing changes. Run with `cargo run --release -p finder`.

## Open Questions

- The live camera: which finder camera, how frames arrive (USB, network), and the frame rate
  the console should solve at.
- The SkySafari link: an LX200 server inside `finder`, or a separate bridge.
- How an AI drives the console: the API as tools, and what it may do without a confirmation.
- Chart detail beyond this change: constellation figure lines and boundaries, and star marks
  graded by magnitude.
