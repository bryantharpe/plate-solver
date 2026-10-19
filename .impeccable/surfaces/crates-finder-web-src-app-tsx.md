---
version: 1
slug: "crates-finder-web-src-app-tsx"
primary_target: "crates/finder/web/src/App.tsx"
related_targets: ["crates/finder/web/src/index.css","crates/finder/web/index.html"]
---

# Finder console: main screen

## Scope and mode

The whole console page (one screen, phone first, desktop second). Mode: **Operate**. The
observer is in a task: find out where a frame points, check that the solve is right, decide
what to look at next.

## Audience, job, constraints

- One observer with a 16-inch Dobsonian, at the telescope at night and indoors in daylight,
  about equally; an Android phone or a Surface laptop.
- Order the user fixed: where it points first (constellation, RA/Dec, what is in the
  field), the annotated frame second, next targets after.
- Night ("dark mode", the user's words): red and black only, no white pixels anywhere,
  frames, previews and embedded views included. It follows the device's dark setting until
  the user picks.
- Works offline (fonts self-hosted); the sky survey view is an online extra.
- Proof, not claims: every matched star sits on the frame where the solution puts it.

## Direction contract

THESIS: The console is an atlas chart of your own frame: the solve proves itself by drawing
the sky's RA/Dec grid and the atlas's object symbols over the frame. It refuses the
planetarium-app default of navy glass cards, cyan glow and starfield wallpaper.

OWN-WORLD: Day is a star-atlas desk edition: cool paper white, black ink, hairline rules, and
the atlas's object colours (green nebulae, gold clusters, red galaxies) used only on objects.
Archivo throughout, italic for object labels, tabular figures, the constellation set as a
letterspaced chart title. Night is the field edition seen under a red torch: black ground,
red ink in three brightnesses, every pixel red.

STORY: The observer reads where the frame points at a glance, sees the grid and symbols
land on the actual stars, then picks what is in the field or what to look at next.

FIRST VIEWPORT: Phone: top bar (Finder, frame chip, site chip, Night switch); the chart's
margin header (constellation, J2000, RA and Dec at display size, field and fit lines); then
the frame, graticule labelled in the paper margin at its edges, matched stars ringed and
named, objects in atlas symbols; legend chips under it. Desktop: header and lists left, the
chart large on the right.

FORM: Field atlas, rank 1 of my 7 (the pick); seed: seed:229035abade06bae. Signature move:
the RA/Dec graticule drawn over the real frame and labelled at its edges like an atlas chart.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Memorable moment

The grid lines and object symbols snap onto the photo of the sky, and the named stars sit
inside their rings: the frame becomes a chart.

## Unresolved

- Bayer letters for star labels (the atlas habit) need a verified table; names only for now.
- Live camera, mount and SkySafari panels come later and inherit this world.
