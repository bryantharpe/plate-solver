# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

React + TypeScript + Vite + Tailwind CSS for the page, served by the Rust `finder` binary
(axum), with the production bundle committed and embedded in the binary so `cargo build` and
CI never need Node. The user ruled out plain HTML/CSS/JS ("bland, and scalability suffers")
and delegated the framework: React was chosen for the ecosystem the later pieces need (live
camera view, mount controls, an AI chat panel, offline use) and because v1's `ps-web` already
proved this shape in this repo.

## Users

One amateur astronomer, the project's maintainer, with a Sky-Watcher 400P (16-inch) GoTo
Dobsonian and a finder camera. They use the console about equally at the telescope at night
and indoors testing frames, on an Android phone or a Surface laptop.

## Product Purpose

The finder console turns one camera frame into where the telescope is pointing, what is in
and near the field, and what to look at next. Today it solves still frames (the bundled
samples or an upload). It is the program that will later run the live camera, the SkySafari
link, GoTo control and AI steering. Success now: at a glance, and with proof, the user can
see that the solver works and where a frame points.

## Positioning

The solver behind it gives cedar's answers on cedar's reference frames and runs fast enough
for a live camera stream (README benchmark), and the console is built to grow into
closed-loop GoTo and AI-steered observing rather than stopping at push-to.

## Operating Context

- At the telescope: dark site, red light only, phone in one hand, glancing between eyepiece
  and screen. Often no internet (inferred).
- Indoors: desk in normal light, Surface (Windows on ARM, WSL2) or phone browser.
- The solver runs on the Surface now (a Raspberry Pi later); the phone opens the page over
  Wi-Fi, and an Android phone could run the solver itself through Termux.
- Later: SkySafari (6+ Plus/Pro, or 8) connects to the finder as an LX200 telescope, the
  mount is driven through its SynScan controller, and an AI assistant uses the console's
  actions as tools.

## Capabilities and Constraints

- Solve a frame: star detection and a lost-in-space solve with no pointing prior; RA/Dec,
  roll, field of view, matched catalog stars, residuals, timing.
- After a solve, "where it points" comes first: large RA/Dec, the constellation, and what is
  in the field. The annotated frame comes second (user's answer).
- The annotated frame shows detected stars, matched catalog stars with names, and Messier
  objects in the field: the proof.
- Next targets: objects near the field, and objects similar to one in it that are up now.
  Rules today; AI later.
- Works offline; online extras (Aladin survey imagery) are optional.
- The default database covers 10–30° fields; frames from other cameras need a matching one.
- Not yet: live camera, mount control, SkySafari link, AI.

## Brand Commitments

None yet. The repository is `plate-solver`; the app is the finder console.

## Evidence on Hand

- 21 cedar sample frames in `reference-solutions/cedar-solve/examples/data/medium_fov/`;
  20 solve, `roof.jpg` does not.
- Benchmark numbers in `README.md`, from one Snapdragon laptop under WSL2.
- No frames from the user's own finder camera yet: never imply real-hardware results.

## Product Principles

1. Prove, don't claim: every answer shows its evidence, the matched stars on the frame.
2. Night is a first-class mode: nothing on screen may break dark adaptation.
3. Glanceable first, detail on demand.
4. Every view is a client of the same pointing state that SkySafari, the mount and the AI
   will use.

## Accessibility & Inclusion

- Night mode is red-only and dim, with no white flashes, images and embedded views included.
- Large touch targets for one-handed use in the cold (inferred).
- Day mode meets WCAG AA contrast; night mode trades contrast for dark adaptation (inferred).
