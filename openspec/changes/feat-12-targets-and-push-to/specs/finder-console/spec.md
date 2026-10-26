## MODIFIED Requirements

### Requirement: The observing site

The page SHALL let the user set a latitude (−90 to 90) and longitude (−180 to 180, east
positive), refusing anything else with a message, fill them from the device's location when the
browser shares it (and say why not otherwise), forget the site, and keep it in the observing
session, so every device shows the same site. When the session has no site and this browser
stored one before, the page SHALL put that site into the session on its first load and then
forget it from the browser, so it is adopted once. (Ref: `web/src/components/SiteDialog.tsx`,
`web/src/settings.ts`; tests: SiteDialog, every case; settings "remembers the observing site…",
"accepts only real latitudes and longitudes".)

#### Scenario: A site that is not on Earth
- **WHEN** the user enters latitude 95
- **THEN** the site is not saved and the dialog says which ranges are allowed

#### Scenario: Set on the phone, seen on the laptop
- **WHEN** the site 40° N, 75° W is saved on one device and the console is opened on another
- **THEN** the second device shows the same site, and its suggestions are ranked for it

#### Scenario: A site this browser stored before
- **WHEN** the session has no site and the page loads in a browser that stored 40° N, 75° W
  under the earlier console
- **THEN** the session's site becomes 40° N, 75° W, the browser no longer holds its own copy,
  and a later load in that browser leaves the session's site alone

### Requirement: Objects like this one

The system SHALL answer `GET /api/similar?id=<Messier number>` with objects of the same family
(the Crab, the only supernova remnant, borrows the planetary and diffuse nebulae), excluding the
object itself and anything within 1° of it, nearest first. Given `lat` and `lon` (and optionally
`time`, Unix seconds, default now), each suggestion SHALL carry its altitude and azimuth and
whether it is up, at or above the observing session's minimum altitude (15° unless changed), and
those up SHALL come before those that are not. `limit` defaults to 8 and is clamped to 1–20. An
unknown object SHALL answer 404; a latitude or longitude out of range, only one of them, or no
`id` SHALL answer 400. (Ref: `src/catalog.rs` `similar`, `similar_above`, `src/api.rs`
`similar`; tests: `like_m42_without_an_observer_is_the_nearest_other_nebula`,
`like_m42_on_a_january_evening_puts_summer_nebulae_last`, `the_crab_borrows_planetaries_and_nebulae`,
`similar_objects_rank_for_an_observer`.)

#### Scenario: Like M42
- **WHEN** objects like M42 are asked for without a site
- **THEN** they are diffuse nebulae, M78 first, and M43 (beside M42) is not among them

#### Scenario: Like M42 on a January evening
- **WHEN** they are asked for at 40° N, 75° W at 9 pm on 2026-01-15, limit 3
- **THEN** M78 comes first, more than 40° up, and the summer nebulae below the horizon come last

#### Scenario: Ranked for a higher minimum altitude
- **WHEN** the session's minimum altitude is 50° and objects like M42 are asked for at 40° N,
  75° W at 2026-01-16 02:00 UT
- **THEN** M78, below 50° at that time, is not up and comes after any suggestion at or above 50°

### Requirement: What is in the field, and what to look at next

The page SHALL list the objects in the field, brightest first, with name or designation, kind,
magnitude and whether they sit at the edge, and the named stars brightest first, the same way the
roster names them; selecting an object makes it the reference for suggestions. It SHALL suggest
objects like the reference from `/api/similar`, giving each one's altitude and compass direction
at the observer's site, or "below" the session's minimum altitude ("below 15°" at the default)
with a horizon mark in readable ink for those that are down, and the angular distance when no
site is set, with a prompt to set one. It SHALL list objects just off the frame with an arrow
pointing the way to turn and their distance. (Ref: `web/src/components/FieldList.tsx`,
`NextTargets.tsx`; tests: FieldList "lists every object…"; NextTargets, every case.)

#### Scenario: Like M42 from the user's site
- **WHEN** suggestions load for M42 at 40° N, 75° W
- **THEN** M78 shows its altitude and direction, and M8 shows "below 15°" in readable ink with a
  faint symbol

#### Scenario: The minimum altitude raised
- **WHEN** the session's minimum altitude is 30° and suggestions load for M42 at 40° N, 75° W
- **THEN** every suggestion that is down reads "below 30°"

### Requirement: Where it points comes first

The page SHALL open a solved frame with where it points: a title block naming the
constellation, RA and Dec at display size with their unit marks, the field, roll and scale, and
the solve time, matched-of-detected stars and RMS residual, with these facts wrapping only
between clauses. In reading order it SHALL then give what is in the field, the chart, the full
list of what is in the field, what to look at next and the survey view; with a target set, the
guidance block and the title block come first, then the chart and its legend, then what is in
the field and the rest. On a phone of 412 × 915 or 390 × 844 CSS px, the first screen SHALL
hold the title block, a one-or-two-line roster of the field's objects and brightest named stars,
the whole chart and its legend with no target set, and the guidance block, the title block, the
whole chart and its legend with one. On a screen at
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

#### Scenario: The first screen with a target
- **WHEN** M78 is the target and orion2 is charted on a 412 × 915 or a 390 × 844 screen
- **THEN** the guidance block, the title block, the chart and the legend are all fully on
  screen, in that order from the top, and the roster comes after the legend

## ADDED Requirements

### Requirement: Choosing a target

The page SHALL offer, from the top bar, a target sheet that searches the catalog as the user
types and lists each match with its id, Messier number, common name, kind, magnitude and whether
it is pointable now. Picking a match SHALL make it the session's target, and the sheet SHALL also
offer to clear the target.
**Refs**: target-catalog "Search"; push-to "Classes in a search"; DESIGN.md (Sheets, Lists)

#### Scenario: Andromeda by name
- **WHEN** the user types `andro` in the target sheet
- **THEN** NGC 224 (M31), the Andromeda Galaxy, is listed first with the class the search
  answer gives it; picking it makes it the session's target, closes the sheet, and the guidance
  block names it

#### Scenario: Nothing matches
- **WHEN** the user types `qqq`
- **THEN** the sheet lists nothing and says that nothing matches

#### Scenario: Clearing the target
- **WHEN** the user clears the target
- **THEN** the session has no target, and the page returns to its layout without one

### Requirement: The guidance block

With a target set, the page SHALL show a guidance block naming the target and its class: when
pointable, the turn and tilt from the latest pointing to whole degrees ("turn clockwise 34° ·
tilt up 12°") and the pointing's age, or a prompt to solve a frame; when down, when it rises,
transits and sets; under the Sun guard, that the console will not point there; without a site,
that a site is needed, leading to the Setup page. It SHALL refresh at least every 5 s.
**Refs**: push-to (every requirement); GLOSSARY.md (turn, tilt, Sun guard)

#### Scenario: Directions to M78
- **WHEN** orion2 is the latest pointing and M78 the target, under a fixed clock
- **THEN** the block reads the turn and tilt the session gives, rounded to whole degrees, in the
  form "turn clockwise 34° · tilt up 12°", with the pointing's age

#### Scenario: A target that is down
- **WHEN** M8 is the target at 40° N, 75° W at 2026-01-16 02:00 UT
- **THEN** the block says M8 is below the horizon and gives its rising, transit and setting
  times in the device's local time, to the minute, with no turn or tilt

#### Scenario: Near the Sun
- **WHEN** M75 is the target at that time
- **THEN** the block says the console will not point within 15° of the Sun, with no directions

#### Scenario: A target without a site
- **WHEN** M42 is the target and the session has no site
- **THEN** the block says a site is needed and links to `/setup`, with no times or directions

#### Scenario: Changed on another device
- **WHEN** another client puts the target M31 while the page shows M78
- **THEN** the page's guidance block names M31 within 5 s

### Requirement: The target on the chart

When the charted frame is the latest pointing, the page SHALL draw the target on it: on the
frame, its family's atlas symbol inside a target reticle, labelled; off the frame, an arrow at
the frame's edge pointing toward it, labelled with its distance. The target SHALL NOT be a
legend layer, and SHALL follow night mode's red-only rule.
**Refs**: push-to "Placed on the frame"; DESIGN.md (Chart, the Red Torch Rule)

#### Scenario: M78 on orion2
- **WHEN** orion2 is charted as the latest pointing with M78 as the target
- **THEN** the target reticle is centred on M78's mark within 0.5 px, and the legend still has
  its five chips (Grid, Matched, Unseen, Deep sky, Centroids) with the same counts as without a
  target

#### Scenario: M79 off orion2
- **WHEN** M79 is the target instead
- **THEN** an arrow at the frame's edge points within 1° of the direction the report gives M79,
  labelled M79 with its distance in whole degrees

#### Scenario: At night
- **WHEN** orion2 is charted in night mode with M78 as the target
- **THEN** the night pixel check finds no pixel with any green or blue, and none redder than the
  ink beyond two levels of compositing rounding

### Requirement: The Setup page

The page SHALL have a Setup page at `/setup`, reachable from the top bar and set in the field
atlas design, that sets the session's site (typed, or from the device's location) and its
minimum altitude (0–80°, refusing other values with a message), and leads back to the console.
**Refs**: observing-session "Checked edits"; DESIGN.md; the Setup page's surface brief

#### Scenario: Raising the minimum altitude
- **WHEN** the user sets 30° on the Setup page and returns to the console
- **THEN** the session's minimum altitude is 30°, and suggestions that are down read "below 30°"

#### Scenario: A minimum altitude out of range
- **WHEN** the user enters 90°
- **THEN** nothing is saved and the page says that 0–80° is allowed

#### Scenario: Opened directly
- **WHEN** a browser opens `/setup`
- **THEN** the Setup page is shown, from the page's own route fallback

#### Scenario: Setup at night
- **WHEN** the Setup page is shown in night mode
- **THEN** the night pixel check finds no off-red pixel, as for the console
