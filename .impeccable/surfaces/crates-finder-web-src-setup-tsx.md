---
version: 1
slug: "crates-finder-web-src-setup-tsx"
primary_target: "crates/finder/web/src/Setup.tsx"
related_targets: ["crates/finder/web/src/App.tsx","crates/finder/web/src/components/TopBar.tsx","crates/finder/web/src/index.css"]
---

# Finder console: Setup page

## Scope and mode

The Setup page at `/setup`, phone first, desktop second. Mode: **Configure**. The observer
sets things once a night, or once ever, and goes back to the console. Specified by
`openspec/changes/feat-12-targets-and-push-to` (the site and the minimum altitude); later
changes add sections to the same page:
- feat-13: the scope node and the finder camera (connection, model, star count, exposure,
  gain, σ, the solved field against the database's range) and the aim point;
- feat-14: the mount and the pointing model ("tilt 0.8° toward 130°", RMS, sync count);
- feat-15: the imaging camera (sensor size in mm, pixel pitch in µm).

## Audience, job, constraints

- The same observer as the console: a 16-inch Dobsonian, a phone in one hand at night or a
  Surface at a desk.
- What is set here belongs to the observing session, shared by every device, so it reads as
  the telescope's settings, not this browser's preferences.
- Site: latitude and longitude typed in (east positive), or the device's location where the
  browser shares it. No GPS work.
- Minimum altitude: one number, 0–80°, default 15°. Say plainly what it does: targets below
  it are "too low"; below 0° they are "below the horizon".
- The Sun guard is not a setting. If it is mentioned, it is a fact ("never within 15° of the
  Sun"), never a switch.
- Night mode: red on black, every pixel; the page joins the night pixel check.

## Direction contract

THESIS: Setup is the atlas's front matter: the observer's place and limits set like an atlas's
table of observing data, in ruled rows with tabular figures, not a settings app's toggles and
cards.

OWN-WORLD: The console's field-atlas world unchanged: cool paper and three inks by day, red on
black by night, Archivo with tabular figures, square 1px ruled boxes, hairline rows, the
doubled rule for the page's title block. No object colours (nothing here is a deep-sky
object).

STORY: The observer opens Setup from the top bar, sees the site and minimum altitude already
set (or a clear "not set"), changes one, sees it saved, and goes back to the chart.

FIRST VIEWPORT: Phone: top bar; a doubled-rule title block "Setup"; the Site section (two
44px fields with ° units, the "Use this device's location" button, Save); the Minimum
altitude section (one field with °, a one-line explanation). Desktop: the same, one column
centred at the console's left-column width.

FORM: Field atlas, as the main screen. Sections are ruled rows under headline titles; each
row has its label, its value or field, and its unit, aligned on the console's 4-track grid.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the
verdict, and DESIGN.md updated with the Setup page as built.

## Memorable moment

The site typed on the laptop appears on the phone's console a few seconds later: one
telescope, one set of settings.

## Unresolved

- Whether Setup is a page or a full-height sheet on phones; the spec says a page at `/setup`,
  and the design pass may present it as a sheet-like page as long as the route holds.
- How saved state is acknowledged (inline "Saved" in Second Ink, or the field settling).
