# finder-console Specification

## Purpose
The finder console: the `finder` binary and its page, a consumer of the solver that
plate-solves a frame and charts it, showing where the telescope points, what is in and near the
field, and what to look at next, at the telescope in a red-on-black night mode and offline.

## Requirements

### Requirement: Serve the console from one binary

The system SHALL provide a `finder` binary that loads one pattern database (`--db`, default cedar's
`default_database.npz`) and a folder of sample frames (`--samples`, default cedar's `medium_fov`
examples), and serves the JSON API under `/api/` and the page on one address (`--listen`, default
`127.0.0.1:8080`). (Ref: `src/main.rs`, `src/api.rs` `router`, `page`; tests:
`the_page_is_served_and_routes_fall_back_to_it`, `page_types`,
`samples_are_listed_served_and_previewed`.)

#### Scenario: The page is built in
- **WHEN** the crate is built
- **THEN** the page's production build SHALL be embedded in the binary, so building the crate never needs Node

#### Scenario: The committed build is the build of the source
- **WHEN** CI rebuilds the page from source
- **THEN** the result is identical to the committed `web/dist`, or the `finder-page` job fails

### Requirement: Page routes and caching

Any path without a file extension SHALL serve the page's `index.html`; fingerprinted files under
`assets/` SHALL be cacheable for a year and everything else revalidated; a missing page file SHALL
answer 404, and an unknown `/api/` route SHALL answer 404 with a JSON `{"error": "..."}` body.
(Ref: `src/api.rs` `router`, `page`; tests: `the_page_is_served_and_routes_fall_back_to_it`,
`page_types`.)

#### Scenario: The page and its routes
- **WHEN** a browser requests `/`, `/index.html` or `/some/route`
- **THEN** each answers 200 with the page's HTML, and `/assets/missing.js` answers 404

#### Scenario: Unknown API route
- **WHEN** a client requests `/api/nothing-here`
- **THEN** the answer is 404 with a JSON `error`

### Requirement: Report the loaded database

The system SHALL answer `GET /api/health` with `status: "ok"`, the crate `version`, the
database's `catalog`, `stars`, `patterns`, `min_fov` and `max_fov` (degrees), and the number of
sample frames. (Ref: `src/api.rs` `health`; test: `health_reports_the_database`.)

#### Scenario: Health of the default setup
- **WHEN** the server runs with cedar's default database and samples
- **THEN** health reports 42,212 stars, a 10–30° field range and 21 samples

### Requirement: Third-party notices

The system SHALL carry, in `THIRD_PARTY_NOTICES.md`, a notice for each third-party work compiled
into the binary (the page's packages and fonts, and the bundled sky data), with its source, version,
license and the license's full text. The page's packages SHALL be listed at the versions and
licenses `web/package-lock.json` pins. (Ref: `THIRD_PARTY_NOTICES.md`, `src/main.rs`
`THIRD_PARTY_NOTICES`; tests: `licenses_prints_the_notices_and_exits`,
`the_notices_name_the_page_packages_as_locked`.)

#### Scenario: The licenses flag
- **WHEN** `finder --licenses` is run
- **THEN** it SHALL print the file and exit without loading a database or serving

#### Scenario: Printing the notices
- **WHEN** `finder --licenses --db no-such-database.npz` runs
- **THEN** it prints `THIRD_PARTY_NOTICES.md` exactly and exits successfully

#### Scenario: A package upgrade
- **WHEN** React, React DOM, scheduler, Tailwind CSS, Vite or the Archivo font is locked at a
  version or license other than the one its notice names
- **THEN** the test fails until the notice is updated

### Requirement: Offer sample frames safely

The system SHALL list the image files (`jpg`, `jpeg`, `png`, `bmp`, `pgm`, `ppm`, `pnm`, any case)
directly inside the samples folder, sorted by name, each with a readable label. Only names from the
listing SHALL resolve: any other name, including one that climbs out of the folder, SHALL answer
404. (Ref: `src/samples.rs`, `src/api.rs`; tests:
`samples_are_listed_served_and_previewed`,
`scan_lists_images_only_and_paths_resolve_only_for_listed_names`, `names_labels_and_types`.)

#### Scenario: Sample labels
- **WHEN** a sample is listed
- **THEN** its readable label SHALL be the file stem with underscores as spaces, first letter capitalised, a leading capture timestamp dropped

#### Scenario: Listing skips what is not a frame
- **WHEN** the samples folder also holds a subfolder and non-image files
- **THEN** the listing holds only the 21 image files, labelled (`orion_belt.jpg` → "Orion belt")

#### Scenario: A path outside the folder
- **WHEN** a client requests `/api/samples/..%2F..%2FCargo.toml`
- **THEN** the answer is 404 and no file outside the listing is read

### Requirement: Sample files and previews

The system SHALL serve a sample's file and a JPEG preview at most 320 px on its longer side, shrunk
by keeping each block's brightest pixel and stretched from the sky background to the brightest
stars, so faint stars survive. (Ref: `src/solve.rs` `thumbnail_jpeg`, `src/api.rs`; tests:
`samples_are_listed_served_and_previewed`, `thumbnails_fit_the_box`, `previews_keep_faint_stars`,
`stretch_takes_the_sky_to_black_and_the_stars_to_white`.)

#### Scenario: A preview keeps a faint star
- **WHEN** a 1600 × 1200 frame with one faint star is previewed at 160 px
- **THEN** the preview is 160 × 120 and the star is still near white

### Requirement: Solve request

The system SHALL accept `POST /api/solve` as `multipart/form-data` with a frame, either an
`image` file or a `sample` name, and optional fields: `fov` (horizontal degrees, empty for
unknown, otherwise more than 0 and less than 180), `fov_max_error` (degrees, more than 0,
default 10), `distortion` (radial `k`, magnitude less than 1, default 0), `bin` (1, 2, 4 or 8,
default 2), `sigma` (more than 0, default 8), `stars` (4 to 200, default 30) and `timeout_ms`
(more than 0, default 5000, clamped to 30,000). The defaults SHALL be the `solve_image`
example's. Unknown fields SHALL be ignored. (Ref: `src/api.rs` `SolveForm::read`, `check`,
`src/solve.rs` `SolveOptions`; tests: `options_are_checked`,
`defaults_match_the_solve_image_example`, `bad_requests_are_refused_with_reasons`,
`a_sample_solves_and_its_overlay_lands_on_its_stars`, `an_upload_solves_and_finds_what_is_in_it`.)

#### Scenario: A sample by name with the defaults
- **WHEN** the form holds only `sample=orion2.jpg`
- **THEN** the frame solves with the default options and the report names `orion2.jpg`

#### Scenario: An upload with a field of view hint
- **WHEN** the Pleiades frame is uploaded as `mine.jpg` with `fov=10` and `fov_max_error=2`
- **THEN** it solves in Taurus, M45 is in the field and Alcyone is named

#### Scenario: Invalid options
- **WHEN** the form has no frame, or `bin=3`, `sigma=lots` or `fov=200`
- **THEN** the answer is 400 with a JSON `error` saying what is wrong

### Requirement: Solve errors and limits

The system SHALL refuse a request body over 32 MiB with 413, an image over 20,000 px on a side or
over the decoder's 256 MiB budget with 413, bytes that are not a readable PNG, JPEG, BMP or PNM
image with 415, and an unknown sample name with 404, each with a JSON `error`. (Ref: `src/api.rs`
`SOLVE_BODY_LIMIT`, `From<DecodeError>`, `src/solve.rs` `decode`; tests: `oversized_uploads_are_refused`,
`decodes_and_refuses`, `decode_errors_map_to_statuses`, `bad_requests_are_refused_with_reasons`.)

#### Scenario: Too large to read
- **WHEN** an upload is one byte over 32 MiB
- **THEN** the answer is 413 with a JSON `error`

#### Scenario: Not an image
- **WHEN** an upload holds `not a jpeg`
- **THEN** the answer is 415 with a JSON `error`

### Requirement: One solve at a time

The system SHALL run one decode-and-solve at a time, so that frames posted together queue rather
than run the machine out of memory, and each is answered with its own solution. (Ref: `src/api.rs`
`solve_gate`; tests: `frames_posted_together_are_each_answered`.)

#### Scenario: Two frames at once
- **WHEN** the Orion and Pleiades samples are posted together
- **THEN** both answer 200, one in Orion and one in Taurus

### Requirement: A queued upload waits unread

A request SHALL take its turn before its form is read, so a queued upload stays unread on the
network rather than in memory; a request whose form has not fully arrived 30 s after its turn began
SHALL be answered 408 with a JSON `error`, and the next in the queue SHALL go on. (Ref: `src/api.rs` `solve_gate`,
`FORM_READ_TIMEOUT`, `AppState::with_form_timeout`; tests:
`a_stalled_upload_times_out_and_the_queue_moves_on`.)

#### Scenario: An upload that stops
- **WHEN** a client sends half its form and stops while it has the turn, and another frame is
  posted behind it (the test shortens the form timeout to 1 s)
- **THEN** the waiting frame's form is not read while the stopped one has the turn; the stopped
  one is answered 408 with a JSON `error` when the timeout runs out, and the waiting frame then
  solves (in Orion)

### Requirement: Give cedar's answer

The system SHALL decode and detect a frame as cedar's reference pipeline does: color converted
to 8-bit gray with PIL's `convert('L')` weights (ITU-R 601-2 luma in 16-bit fixed point, rounded;
`star_detection::gray`), rows levelled and hot pixels rejected before detection
(`DetectParams::cedar()`), and the brightest `stars` centroids passed to the solver. Both come
from the solver crates (this change's `star-detection` and `plate-solver` requirements), so the
console and the `solve_image` example read a frame the same way. On cedar's 21 sample frames it
SHALL give cedar's status, and for the 20 that cedar solves a pointing within 10″ of cedar's, a
field within 15″, and at least 90% of cedar's matched stars matched again (the image tier's
tolerances). (Ref: `src/solve.rs` `to_gray`, `solve_frame`, `SolveOptions`;
`crates/star-detection/src/gray.rs` `from_rgb8`; `crates/plate-solver/src/solve.rs`
`DetectParams::cedar`; goldens: `crates/plate-solver/tests/data/cedar_parity.json`; tests:
`every_sample_gives_cedars_answer`, `color_turns_gray_as_cedar_turns_it`,
`a_sample_solves_and_its_overlay_lands_on_its_stars`.)

#### Scenario: Every sample
- **WHEN** each of the 21 samples is solved with the defaults
- **THEN** `roof.jpg` does not solve, as in cedar, and the other 20 match cedar within those
  tolerances (measured: 19 within 0.3″ with all of cedar's stars, the last within 4.2″)

#### Scenario: A color pixel turns gray as in cedar
- **WHEN** a color pixel (10, 200, 30) is decoded
- **THEN** its gray level is 124, PIL's, not `image`'s Rec. 709 value of 147

### Requirement: The solve report

The system SHALL answer a completed solve with 200 and a report holding: the frame's `name`,
`width` and `height`; a `status` of `match_found`, `no_match`, `timeout`, `cancelled` or
`too_few`, with a `hint` saying what to try for every status but a match; timings for decode,
detection and solve; up to 500 detected centroids `[x, y]`, brightest first, and how many went
to the solver; and, when it solved, the `pointing`: J2000 RA and Dec (degrees, and as
`05h 40m 46s` and `−04° 14′ 46″` with a true minus sign), the constellation at the centre,
roll, where north points in the image, horizontal and vertical field, scale at the centre in
arcseconds per pixel, fitted distortion, the number of matched stars, the RMS, 90th-percentile
and largest residuals in arcseconds, and the false-alarm probability. A frame that does not
solve SHALL report no pointing, stars, objects or grid. (Ref: `src/solve.rs` `SolveReport`,
`Pointing`, `Status::hint`, `src/sky.rs` `format_ra`, `format_dec`; tests:
`a_sample_solves_and_its_overlay_lands_on_its_stars`, `a_frame_that_does_not_solve_says_what_to_try`,
`every_failure_has_a_hint`, `compass_and_formatting`.)

#### Scenario: Where orion2 points
- **WHEN** `orion2.jpg` is solved
- **THEN** the pointing is RA 85.190964°, Dec −4.246105° and a 10.181431° field (within 1e-4°),
  shown as `05h 40m 46s`, `−04° 14′ 46″`, in Orion, with 30 stars matched

#### Scenario: A frame that does not solve
- **WHEN** `roof.jpg` is solved at the defaults
- **THEN** the status is not a match, the hint says what to try, and there is no pointing

### Requirement: Place the solution on the frame

The system SHALL place every mark in the frame's own pixels using the solver's rotation, camera
model and radial distortion. Matched catalog stars SHALL be placed where they were detected,
each with how far the solution puts its catalog position from there; the brightest unmatched
catalog stars that the solution puts inside the frame (up to 25) SHALL be placed where the
solution puts them. Stars SHALL carry their catalog designation (`HIP 26727`) and, for the
bundled list of bright stars, their proper name, checked against the database's positions.
(Ref: `src/solve.rs` `star_marks`, `src/sky.rs` `Frame`, `src/stars.rs`; tests:
`a_sample_solves_and_its_overlay_lands_on_its_stars`, `named_stars_match_the_database`,
`sorted_unique_and_found`, `rotation_inverts_extract_radec_roll`,
`zero_roll_puts_north_up_and_east_left`, `roll_turns_north_clockwise_from_up`,
`center_projects_to_image_center_and_behind_is_none`, `field_geometry`.)

#### Scenario: The overlay is the solution
- **WHEN** `orion2.jpg` is solved
- **THEN** every matched star's catalog position projects onto its detected centroid within the
  solve's largest residual plus half a pixel, every unmatched star placed lies inside the frame,
  and Alnitak, Alnilam and Mintaka are named

#### Scenario: North and east
- **WHEN** a frame has zero roll
- **THEN** north is up and east is to the left, and a positive roll turns north counterclockwise

### Requirement: Deep-sky objects on and near the frame

The system SHALL report each Messier object whose symbol overlaps the frame, with its image
position, half its major and minor axes in pixels, the image direction of its major axis (from its
position angle and the local direction of north), its distance from the frame centre, and whether
its centre is inside the frame. (Ref: `src/solve.rs` `object_marks`; tests:
`a_sample_solves_and_its_overlay_lands_on_its_stars`, `an_upload_solves_and_finds_what_is_in_it`.)

#### Scenario: Order of objects on the frame
- **WHEN** objects on the frame are reported
- **THEN** those centred inside come first, then by magnitude

#### Scenario: Orion's nebulae at true size
- **WHEN** `orion2.jpg` is solved
- **THEN** the objects are M42, M78 and M43 in that order, M42 is inside the frame with a radius
  of 36–39 px (45′ at 72″ a pixel), and M79 is listed just off the frame about 20° away

### Requirement: Deep-sky objects just off the frame

The system SHALL report up to eight Messier objects off the frame within 15° of its corners,
nearest first, each with its distance and its direction in the image (degrees clockwise from up).
(Ref: `src/solve.rs` `object_marks`; tests: `objects_just_off_the_frame_are_listed_nearest_first`.)

#### Scenario: Objects off the frame
- **WHEN** a frame solves with Messier objects off it within 15° of its corners
- **THEN** up to eight of them SHALL be reported, nearest first, each with its distance and its direction in the image (degrees clockwise from up)

### Requirement: The RA/Dec grid

The system SHALL compute an RA/Dec graticule for a solved frame: parallels at a round spacing giving
about four across the frame's width (0.25° to 45°), hour circles at a round spacing that keeps cells
roughly square on the sky, both carried a little past the frame and labelled where they cross its
edges (`5h 40m` or `6h`; `−4°` or `+2° 30′`). (Ref: `src/grid.rs`; tests:
`on_the_equator_north_up_the_grid_is_square_to_the_frame`, `near_the_pole_hour_circles_radiate`,
`labels`.)

#### Scenario: Hour circles at a pole
- **WHEN** the frame is near a pole
- **THEN** the hour circles SHALL radiate from it all the way round

#### Scenario: Near the pole
- **WHEN** the grid is drawn for a frame centred at Dec 88°
- **THEN** at least six hour circles cross it, every one drawn with finite points

#### Scenario: A 10° field on the equator, north up
- **WHEN** the grid is drawn for a 10° field centred at RA 6h, Dec 0°, roll 0
- **THEN** the equator is horizontal through the centre and meets the right edge then the left,
  the 6h hour circle is vertical, parallels are 2° apart and hour circles 10 minutes apart

### Requirement: Constellation lookup

The system SHALL name the constellation containing a J2000 position from the IAU boundaries as
tabulated by Roman (1987), after precessing the position to their B1875 equinox, with the
IAU's spelling of all 88 names. (Ref: `src/constellation.rs`, `data/README.md`; tests:
`every_band_names_a_constellation_and_all_88_appear`, `bright_stars_land_in_their_constellations`,
`names_are_spelled_as_the_iau_spells_them`, `computed_constellations_agree_with_openngc`.)

#### Scenario: Positions near boundaries and poles
- **WHEN** Betelgeuse, Polaris, Acrux, the vernal equinox, the south pole and M16 are looked up
- **THEN** they fall in Orion, Ursa Minor, Crux, Pisces, Octans and Serpens

#### Scenario: All 110 Messier objects
- **WHEN** each Messier object's constellation is computed
- **THEN** it agrees with OpenNGC's for all 110

### Requirement: Time and place on the sky

The system SHALL compute, for an observer's latitude, longitude and time, the altitude and azimuth
(from north through east) of a J2000 position: precessed to the date with the IAU 1976 angles, with
mean sidereal time, without nutation or refraction. (Ref: `src/sky.rs` `precess_from_j2000`,
`gmst_deg`, `alt_az`, `compass_point`; tests: `precession_matches_meeus_example_21b`,
`sidereal_time_matches_meeus_example_12a`, `alt_az_matches_meeus_example_13b`,
`compass_and_formatting`.)

#### Scenario: Agreement with Meeus
- **WHEN** the result is compared with Meeus's worked examples
- **THEN** it SHALL agree to within 0.01°

#### Scenario: Venus from Washington
- **WHEN** Meeus's example 13.b is computed (1987 April 10, 19:21 UT, US Naval Observatory)
- **THEN** the altitude is 15.1249° and the azimuth 248.0337°, each within 0.01°

### Requirement: The Messier catalog

The system SHALL bundle the 110 Messier objects from OpenNGC with their designation (`NGC 1976`,
or none for Messier-only objects), common name, type and its plain-words kind, family, J2000
position, axes, position angle and magnitude, and find one by its number in any case.
(Ref: `src/catalog.rs`, `data/messier.csv`, `data/README.md`; tests:
`all_110_parse_in_order_with_known_fields`, `families_and_kinds_cover_every_code_in_the_list`.)

#### Scenario: M42
- **WHEN** `m42` is looked up
- **THEN** it is NGC 1976, the Great Orion Nebula, magnitude 4.0, and `M111` is not found

### Requirement: Objects like this one

The system SHALL answer `GET /api/similar?id=<Messier number>` with objects of the same family
(the Crab, the only supernova remnant, borrows the planetary and diffuse nebulae), excluding the
object itself and anything within 1° of it, nearest first. Given `lat` and `lon` (and optionally
`time`, Unix seconds, default now), each suggestion SHALL carry its altitude and azimuth and
whether it is at least 15° up, and those up SHALL come before those that are not. `limit`
defaults to 8 and is clamped to 1–20. An unknown object SHALL answer 404; a latitude or longitude
out of range, only one of them, or no `id` SHALL answer 400. (Ref: `src/catalog.rs` `similar`,
`src/api.rs` `similar`; tests: `like_m42_without_an_observer_is_the_nearest_other_nebula`,
`like_m42_on_a_january_evening_puts_summer_nebulae_last`, `the_crab_borrows_planetaries_and_nebulae`,
`similar_objects_rank_for_an_observer`.)

#### Scenario: Like M42
- **WHEN** objects like M42 are asked for without a site
- **THEN** they are diffuse nebulae, M78 first, and M43 (beside M42) is not among them

#### Scenario: Like M42 on a January evening
- **WHEN** they are asked for at 40° N, 75° W at 9 pm on 2026-01-15, limit 3
- **THEN** M78 comes first, more than 40° up, and the summer nebulae below the horizon come last

### Requirement: Where it points comes first

The page SHALL open a solved frame with where it points: a title block naming the
constellation, RA and Dec at display size with their unit marks, the field, roll and scale, and
the solve time, matched-of-detected stars and RMS residual, with these facts wrapping only
between clauses. In reading order it SHALL then give what is in the field, the chart, the full
list of what is in the field, what to look at next and the survey view. On a phone of 412 × 915
or 390 × 844 CSS px, the first screen SHALL hold the title block, a one-or-two-line roster of
the field's objects and brightest named stars, the whole chart and its legend. On a screen at
least 64rem wide, the chart SHALL sit to the right of the facts and stay in view while they
scroll, with the page centred, sized to the chart, and gutters of at least 40 px. (Ref:
`web/src/App.tsx`, `web/src/components/ChartHeader.tsx`, `FieldList.tsx` `FieldRoster`; tests:
ChartHeader "says where the frame points…", "sets its facts as clauses…"; App "comes back to the
last frame and reads, in order…"; FieldList "rosters the objects…", "points to the full list…";
e2e "the first screen shows where it points…" at both sizes, "the chart sits beside the facts…".)

#### Scenario: The first screen on the user's phone
- **WHEN** orion2 is charted on a 412 × 915 screen
- **THEN** the title block, the roster (M42, M78, M43; Alnilam, Alnitak, Mintaka), the chart and
  the legend are all fully on screen, in that order from the top

#### Scenario: A residual never splits from its unit
- **WHEN** the fit line wraps on a narrow screen
- **THEN** it breaks between "30 of 67 stars matched" and "12″ RMS", never inside either

### Requirement: Choosing a frame

The page SHALL, on a first visit, offer the Orion sample, the device's own frame, and every sample
with its preview. A frame picker reachable from the top bar SHALL offer the same; picking a frame
SHALL chart it. (Ref: `web/src/App.tsx` `FirstRun`, `web/src/components/FramePicker.tsx`; tests: App "opens on a
first-run card…"; FramePicker "offers the server's samples…", "opens a frame from the device",
"takes a field of view hint…"; e2e "a frame from the device solves and is charted".)

#### Scenario: Hints in the frame picker
- **WHEN** the frame picker is open
- **THEN** it SHALL also offer a field-of-view hint (empty for unknown) and a detection threshold

#### Scenario: A frame from the phone's gallery
- **WHEN** the user opens their own frame and chooses the Pleiades image
- **THEN** the page charts it in Taurus with M45 on the chart

### Requirement: Remember the last frame

The page SHALL remember the last sample charted and chart it again on the next visit. (Ref:
`web/src/settings.ts`; tests: App "comes back to the last frame…"; settings "remembers the last
sample charted".)

#### Scenario: The next visit
- **WHEN** a sample has been charted and the page is opened again
- **THEN** the page SHALL chart that sample again

### Requirement: A frame that does not solve

The page SHALL show, for a frame that does not solve, "No match", the server's hint, how many stars
were found at which threshold, the centroids on the frame, and a button to try again at a lower
threshold. (Ref: `web/src/components/ChartHeader.tsx`, `Chart.tsx`; tests: ChartHeader "says what to
try when a frame does not solve…"; Chart "shows only the detected stars, fixed on…"; e2e "a frame
that does not solve says what to try, and solves at the lower threshold it offers".)

#### Scenario: The lower threshold offered
- **WHEN** the button to try again at a lower threshold is shown
- **THEN** the lower threshold it offers SHALL be 5σ from above 5σ, then 3σ, then none

#### Scenario: The roof frame
- **WHEN** `roof.jpg` does not solve at 8σ and the user taps "Try again at 5σ"
- **THEN** the page solves it again with `sigma=5`, and it solves

### Requirement: The chart

The page SHALL draw the frame as a star-atlas chart: the RA/Dec grid clipped to the frame with
its labels in a paper margin at the edges; matched stars as solid rings and unseen catalog stars
as smaller dashed rings; the centroids on request; each Messier object in its family's atlas
symbol (square for a diffuse nebula, dotted circle for an open cluster, crossed circle for a
globular, rayed circle for a planetary, an ellipse at true size and angle for a galaxy), never
smaller than a mark; a north and east compass, a scale bar and a centre reticle. A legend SHALL
key every kind of mark with its count, each a toggle: Grid, Matched, Unseen, Deep sky (keyed with
the symbols actually on the chart) and Centroids (off by default). When a frame does not solve,
the centroids SHALL be all it shows, fixed on. Tapping a star or object SHALL show its facts
under the chart. (Ref: `web/src/components/Chart.tsx`, `Symbol.tsx`; tests: Chart "keys every
kind of mark…", "draws the deep-sky key…", "draws matched and unseen catalog stars and the
centroids as separate layers…", "draws each Messier object with a label…", "draws the RA/Dec
grid…".)

#### Scenario: Layers
- **WHEN** orion2 is charted and the user turns off Matched, then Unseen, then turns on Centroids
- **THEN** the 30 solid rings go, then the 25 dashed rings, then the 67 centroids appear

### Requirement: Chart labels never collide

The page SHALL name every Messier object on the chart and the named stars where there is room,
without any label covering another label, the compass, scale bar or reticle, or crossing its own
symbol; labels SHALL stay inside the frame. On the Orion frame at phone and laptop widths no label
SHALL cover another star's ring. (Ref: `web/src/labels.ts`; tests: labels.test.ts, every case, on
the real Orion and Virgo reports at 380, 800 and 960 px and on constructed fields.)

#### Scenario: Where a label goes
- **WHEN** a label is placed
- **THEN** it goes beside its mark past the symbol's reach in that direction, or further out on a leader line from the symbol's edge when the near spots are taken
- **AND** a star name with no clear spot is left out
- **AND** an object whose centre lies off the frame is named inside it

#### Scenario: Orion on a phone
- **WHEN** orion2 is laid out 380 px wide
- **THEN** M42, M78, M43, Alnitak, Alnilam, Mintaka and Saiph are all named, none over another
  star's ring, label or the chart's furniture

#### Scenario: Fifteen galaxies
- **WHEN** the Virgo frame is laid out 380 px wide
- **THEN** all fifteen galaxies are named and no two labels overlap

### Requirement: What is in the field, and what to look at next

The page SHALL list the objects in the field, brightest first, with name or designation, kind,
magnitude and whether they sit at the edge, and the named stars brightest first, the same way the
roster names them; selecting an object makes it the reference for suggestions. It SHALL suggest
objects like the reference from `/api/similar`, giving each one's altitude and compass direction
at the observer's site, or "below 15°" with a horizon mark in readable ink for those that are
down, and the angular distance when no site is set, with a prompt to set one. It SHALL list
objects just off the frame with an arrow pointing the way to turn and their distance. (Ref:
`web/src/components/FieldList.tsx`, `NextTargets.tsx`; tests: FieldList "lists every object…";
NextTargets, every case.)

#### Scenario: Like M42 from the user's site
- **WHEN** suggestions load for M42 at 40° N, 75° W
- **THEN** M78 shows its altitude and direction, and M8 shows "below 15°" in readable ink with a
  faint symbol

### Requirement: The observing site

The page SHALL let the user set a latitude (−90 to 90) and longitude (−180 to 180, east
positive), refusing anything else with a message, fill them from the device's location when the
browser shares it (and say why not otherwise), forget the site, and remember it on the device.
(Ref: `web/src/components/SiteDialog.tsx`, `web/src/settings.ts`; tests: SiteDialog, every case;
settings "remembers the observing site…", "accepts only real latitudes and longitudes".)

#### Scenario: A site that is not on Earth
- **WHEN** the user enters latitude 95
- **THEN** the site is not saved and the dialog says which ranges are allowed

### Requirement: Night mode is red on black

In night mode every colour SHALL be on the red channel alone, none brighter than the night ink
(`#e00000`). (Ref: `web/src/index.css` `[data-theme="night"]`, `web/index.html` `#red-only` and the
pre-paint script; tests: theme "redefines every colour…", "puts every colour on the red channel
alone…", "filters photographs into the red channel by luminance"; e2e "night mode puts nothing on
screen but red, none brighter than the ink…".)

#### Scenario: Photographs at night
- **WHEN** the page is in night mode
- **THEN** photographs, previews and the survey view SHALL pass through a filter that maps
  luminance to red, scaled so white lands just under the night ink

#### Scenario: Before the first paint
- **WHEN** the page loads
- **THEN** it SHALL be black before its first paint

#### Scenario: Every pixel at night
- **WHEN** orion2 and virgo are charted in night mode and the frame picker is opened
- **THEN** screenshots of the whole page and the picker have no pixel with any green or blue, and
    none redder than the ink beyond two levels of compositing rounding, while the same check on the
  day edition fails

### Requirement: Switching night mode

The page SHALL have a night mode, switched in the top bar, that follows the device's dark setting
until the user chooses and then remembers the choice. Settings SHALL still work when the browser
blocks storage. (Ref: `web/src/settings.ts` `useNight`; tests: settings "follows the device until
chosen…", "follows a dark device setting…", "still works when storage is blocked".)

#### Scenario: Following the device until chosen
- **WHEN** the user has not chosen a mode
- **THEN** night mode SHALL follow the device's dark setting
- **AND** once the user chooses, the page SHALL remember the choice

#### Scenario: Storage blocked
- **WHEN** the browser blocks storage
- **THEN** settings SHALL still work

### Requirement: Day mode is legible

The page's day edition SHALL set body and secondary text at WCAG AA contrast (4.5:1) on both of
its paper tones, and SHALL NOT set readable text in its faint ink. (Ref: `web/src/index.css`
`@theme`, `web/src/components/NextTargets.tsx`; tests: theme "sets body text and secondary text at
WCAG AA…"; NextTargets "keeps a row for an object that is down readable…".)

#### Scenario: Secondary text
- **WHEN** the contrast of `ink-2` on `paper` and `paper-2` is computed from the stylesheet
- **THEN** both are at least 4.5:1

### Requirement: Works offline

The page SHALL load everything it needs from `finder` itself, its fonts included, and SHALL
request nothing from another host until the user asks for the survey view. (Ref: `web/src`,
`web/index.html`; tests: offline "references no other host, except the survey view's Aladin
Lite", "serves its own fonts"; SurveyView "loads nothing from the internet until asked"; e2e
"works offline: nothing comes from another host until the survey is asked for".)

#### Scenario: A night at a dark site
- **WHEN** the page charts a frame with no internet connection
- **THEN** every request it makes goes to the `finder` server

### Requirement: The same field in a sky survey

The page SHALL, on request, show the solved field in Aladin Lite (a pinned version, from CDS) on
the DSS colour survey, with the frame's outline from its field and roll, and SHALL say, when
there is no connection, that the survey needs the internet and the chart does not. (Ref:
`web/src/components/SurveyView.tsx`; tests: SurveyView, every case; e2e "works offline…".)

#### Scenario: Asked for with no connection
- **WHEN** the user asks for the survey and CDS cannot be reached
- **THEN** the page says there is no connection to the survey and that the chart does not need it

#### Scenario: The outline follows the frame
- **WHEN** a 10° × 6° frame is outlined with north up, then with north to the right
- **THEN** the outline spans 10° of RA and 6° of Dec, then 10° of Dec
