# web-ui Delta Specification

## ADDED Requirements

### Requirement: Health endpoint

The system SHALL expose `GET /healthz` returning HTTP 200 with a JSON body containing
`status: "ok"`, the crate `version`, and the loaded database's properties: `star_catalog`,
`min_fov`, `max_fov` (degrees), and `num_patterns`. (Ref: `ps-web/src/lib.rs`.)

#### Scenario: Health reports loaded database properties
- **WHEN** a client sends `GET /healthz` to a server started with a valid database
- **THEN** the response is HTTP 200 JSON with `status = "ok"` and numeric `min_fov`,
  `max_fov`, and `num_patterns` matching the loaded database

### Requirement: Solve endpoint request contract

The system SHALL expose `POST /api/solve` accepting `multipart/form-data` with a required
`image` field (JPEG or PNG bytes) and a required `fov_estimate` field (degrees, positive
and finite), plus optional fields `fov_max_error` (degrees), `match_radius` (default
`0.01`), `match_threshold` (default `1e-5`), `timeout_ms` (default `30000`, clamped to at
most `60000`), and `distortion` (fixed radial coefficient; omitted → estimated). Unknown
form fields SHALL be ignored. (Ref: `ps-web/src/solve.rs`.)

#### Scenario: Minimal valid request
- **WHEN** a client posts multipart form data with only `image` and `fov_estimate`
- **THEN** the solve runs with the documented defaults for all optional parameters

#### Scenario: Missing required field
- **WHEN** `image` or `fov_estimate` is absent from the form
- **THEN** the response is HTTP 400 with a JSON body `{"error": "..."}` naming the
  missing field

#### Scenario: Invalid field values
- **WHEN** `fov_estimate` is non-positive, non-finite, or unparsable, or any optional
  numeric field is unparsable
- **THEN** the response is HTTP 400 with a JSON `error` naming the offending field

#### Scenario: Timeout clamp
- **WHEN** `timeout_ms` exceeds 60000
- **THEN** the effective solve timeout is 60000 ms

### Requirement: Solve endpoint response contract

A completed solve attempt SHALL return HTTP 200 with a JSON body whose `status` field is
one of `match_found`, `no_match`, `timeout`, `cancelled`, or `too_few`. A `match_found`
response SHALL carry `ra_deg`, `dec_deg`, `roll_deg`, `fov_deg` (degrees), sexagesimal
`ra_hms` (`HHhMMmSS.SSs`) and `dec_dms` (`±DDdMMmSS.SSs`), `rmse`, `p90e`, `maxe`
(pixels), `matches`, `prob`, `distortion`, `t_solve_ms`, and a `matched_stars` array of
`{x, y, ra, dec, mag, cat_id}` where `x`/`y` are pixel coordinates in the uploaded image
and `ra`/`dec` are in degrees. Non-`match_found` responses SHALL carry a human-readable
`hint`. (Ref: `ps-web/src/solve.rs`; reference solve of the medium-FOV example image at
`fov_estimate = 11` yields RA ≈ 230.67°, Dec ≈ 11.04°.)

#### Scenario: Successful solve of the reference image
- **WHEN** the medium-FOV reference image is posted with `fov_estimate = 11`
- **THEN** the response is HTTP 200 with `status = "match_found"`, `ra_deg` ≈ 230.67,
  `dec_deg` ≈ 11.04, and a non-empty `matched_stars` array with per-star degrees-valued
  `ra`/`dec`

#### Scenario: Non-match statuses include a hint
- **WHEN** a solve completes without a match (for example an all-black image yields
  `too_few`)
- **THEN** the response is HTTP 200 with the corresponding `status` and a non-empty
  human-readable `hint`

### Requirement: Solve endpoint error mapping

The system SHALL distinguish request errors by HTTP status: 400 for malformed multipart
bodies or invalid fields, 413 when the request body exceeds the 32 MiB limit or the image
exceeds decode limits, 415 for image bytes that cannot be decoded as JPEG/PNG, and 500 if
the solver panics or the solve gate is unavailable — each with a JSON `{"error": "..."}`
body. (Ref: `ps-web/src/lib.rs` `SOLVE_BODY_LIMIT`; `ps-web/src/solve.rs`.)

#### Scenario: Undecodable image
- **WHEN** the `image` field contains bytes that are not a decodable JPEG/PNG
- **THEN** the response is HTTP 415 with a JSON `error`

#### Scenario: Oversize request body
- **WHEN** the multipart body exceeds 32 MiB
- **THEN** the response is HTTP 413

### Requirement: Decode resource limits

Image decoding SHALL enforce a per-side dimension cap of 20000 px and a total decoder
allocation cap of 256 MiB, rejecting inputs that exceed them with HTTP 413, so that a
small highly-compressible file declaring absurd dimensions (decompression bomb) cannot
exhaust process memory. (Ref: `ps-web/src/solve.rs` `image_decode_limits`.)

#### Scenario: Decompression bomb rejected
- **WHEN** an uploaded file declares decoded dimensions or allocation beyond the caps
- **THEN** decoding aborts and the response is HTTP 413 with a JSON `error`, without the
  full pixel buffer being allocated

### Requirement: Solve serialization and cancellation

The system SHALL serialize heavy work through a single-permit solve gate: at most one
decode+solve runs at a time, acquired before the image is decoded, with decode and solve
executed on a blocking thread off the async executor. If the client disconnects
mid-solve, the system SHALL trip the solve's cancel flag so the in-flight solve exits
early and releases the gate. (Ref: `ps-web/src/solve.rs` `solve_handler`,
`CancelOnDrop`.)

#### Scenario: Concurrent solves are serialized
- **WHEN** a second solve request arrives while one is in flight
- **THEN** it waits for the gate rather than decoding or solving concurrently

#### Scenario: Client disconnect cancels the solve
- **WHEN** the requesting client disconnects while its solve is running
- **THEN** the solve's cancel flag is tripped and the solve loop exits early

### Requirement: Embedded SPA serving

The server binary SHALL embed the built browser UI (`ps-web/frontend/dist`, committed to
git) and serve it without any runtime filesystem or node dependency: `GET /` returns the
SPA shell as `text/html`; hashed asset paths return their embedded bytes with correct
content types (`text/javascript`, `text/css`, …); requests for missing assets or other
dotted paths return 404; extension-less unknown paths fall back to the SPA shell. Plain
`cargo build` SHALL never invoke node. (Ref: `ps-web/src/lib.rs` `FrontendAssets`,
`static_handler`.)

#### Scenario: SPA shell at root
- **WHEN** a client sends `GET /`
- **THEN** the response is HTTP 200 `text/html` containing the SPA root element and a
  module-script reference to a hashed `/assets/` bundle

#### Scenario: Assets referenced by the shell exist and carry correct mime types
- **WHEN** each `/assets/...` path referenced by the served shell is requested
- **THEN** every response is HTTP 200 with a content type matching the file kind
  (guarding against a stale or half-committed `dist`)

#### Scenario: Missing asset is not masked
- **WHEN** a client requests an `/assets/` path (or other dotted path) not in the embed
- **THEN** the response is HTTP 404

#### Scenario: SPA fallback
- **WHEN** a client requests an unknown extension-less path
- **THEN** the response is the SPA shell (HTTP 200 `text/html`)

#### Scenario: Self-contained binary
- **WHEN** the built server binary runs with `frontend/dist` absent from disk
- **THEN** the UI and assets still serve from the embedded bytes

### Requirement: Browser solve workflow

The browser UI SHALL let a user select a JPEG/PNG star-field image (file picker or
drag-and-drop, with preview), enter a FOV estimate (showing the supported range fetched
from `/healthz`), optionally set the five advanced parameters, and submit a solve. On
`match_found` it SHALL display the solution (RA/Dec in degrees and sexagesimal, roll,
solved FOV, RMSE/P90/max error, match count, probability, solve time) and the matched
stars; on other statuses it SHALL display the status title and the server's `hint`; on
transport or non-2xx errors it SHALL display the error message. Empty optional fields
SHALL be omitted from the request rather than sent as empty strings. (Ref:
`ps-web/frontend/src/`.)

#### Scenario: Solve from the browser
- **WHEN** the user provides an image and FOV estimate and submits
- **THEN** the UI indicates solving is in progress, then renders the solution or status
  hint returned by `POST /api/solve`

#### Scenario: FOV range hint
- **WHEN** the page loads and `/healthz` succeeds
- **THEN** the FOV input shows the database's supported `min_fov`–`max_fov` range

### Requirement: Matched-star overlay

On a successful solve the UI SHALL render the uploaded image with a marker at each
matched star's `x`/`y` pixel position, scaled correctly to the image's natural pixel
coordinate system at any display size, working fully offline. Hovering a marker SHALL
reveal that star's catalog ID, magnitude, RA/Dec, and pixel position; hover state SHALL
be cross-linked with the matched-stars table (hovering either highlights both). (Ref:
`ps-web/frontend/src/components/StarOverlay.tsx`.)

#### Scenario: All matched stars marked
- **WHEN** a solve returns N matched stars
- **THEN** the overlay renders N markers positioned at the stars' pixel coordinates on
  the uploaded image

#### Scenario: Hover reveals catalog details
- **WHEN** the user hovers a marker or its table row
- **THEN** the marker highlights, a tooltip shows catalog ID / magnitude / RA / Dec /
  pixel position, and the corresponding table row highlights

### Requirement: Aladin sky view with graceful degradation

The UI SHALL offer an Aladin Lite sky view of the solved position (target = solved
RA/Dec, FOV = 2× solved FOV, ICRS frame, matched stars plotted as a catalog overlay),
loading the Aladin script from its CDN on demand with a bounded timeout. If the script
cannot load or initialize, the UI SHALL degrade to an explanatory message; an external
"Open in Aladin" link to the solved position SHALL be present in all cases. (Ref:
`ps-web/frontend/src/components/AladinView.tsx`.)

#### Scenario: CDN unavailable
- **WHEN** the Aladin script fails to load or times out
- **THEN** the sky view section shows an unavailability message and the external
  "Open in Aladin" link still points at the solved position

### Requirement: Server configuration

The server SHALL accept `--db <path>` (required) loading either a `.npz` tetra3 database
(imported on load) or a native `ps-db` file (loaded directly), and `--listen <addr>`
(default `127.0.0.1:8080`). (Ref: `ps-web/src/main.rs`, `ps-web/README.md`.)

#### Scenario: Default listen address
- **WHEN** the server starts without `--listen`
- **THEN** it serves on `127.0.0.1:8080`
