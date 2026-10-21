## Purpose

The console's live loop over the scope link: solving the newest star frame as it arrives, timing
each pointing on the console's clock, showing the live chart, calibrating the aim point, and
keeping guidance to pointings fresh enough to trust.

## ADDED Requirements

### Requirement: Live solving

While a scope node is connected, the console SHALL solve the newest star frame each time it is
free, dropping frames that arrived while it was busy, and SHALL use the last solved field of view
as the estimate for the next solve, dropping the estimate after three solves in a row fail.
**Refs**: plate-solver "Solve inputs and defaults" (the field-of-view estimate); GLOSSARY.md (lost-in-space)

#### Scenario: Two frames a second for ten seconds
- **WHEN** the simulator streams 2 frames a second for 10 s of virtual time with the telescope
  still
- **THEN** every frame is solved, and every solved pointing is within 30″ of the simulator's true
  eyepiece direction at that frame's mid-exposure

#### Scenario: A slow solver
- **WHEN** each solve is made to take 1.5 s while frames arrive at 2 a second
- **THEN** each solve starts on the newest frame received at that moment, and no older frame is
  solved after a newer one

#### Scenario: The field of view carries over
- **WHEN** a frame solves with a 12.04° field
- **THEN** the next frame is solved with 12.04° as its estimate, and after three failures in a
  row the following frame is solved with no estimate

### Requirement: Timed at mid-exposure

Each live pointing SHALL be timed at its frame's mid-exposure, mapped from the node's clock to the
console's with the link's clock offset, and its altitude and azimuth SHALL be computed for that
time.
**Refs**: scope-link "Clock mapping"; docs/research/2026-10-21-pointing-model-and-tracking.md §4 (1 s of clock is 15″)

#### Scenario: A worked timing
- **WHEN** the link's offset is +0.050 s and a frame was exposed from 200.000 to 200.200 s on the
  node's clock
- **THEN** its pointing's time is 200.050 s on the console's clock (within 1 µs)

### Requirement: The live chart

While a scope node is connected, the page SHALL update its chart with each live solve over a
server-sent event stream, show the node's previews under the frame through night mode's
red-only filter, and say when the newest pointing is older than 5 s. With no node connected the
page SHALL open no stream and behave as it did before.
**Refs**: finder-console "Night mode is red on black", "Works offline"; DESIGN.md (the Red Torch Rule)

#### Scenario: Live from the simulator
- **WHEN** the page is open on `finder --simulate` and the simulated telescope is pushed
- **THEN** within 2 s the chart shows a pointing within 30″ of the simulator's new true
  direction, and the preview appears under the frame

#### Scenario: No node, no stream
- **WHEN** the page is open on a console with no scope node
- **THEN** it opens no event stream, and the page reaches network idle as before

#### Scenario: Live at night
- **WHEN** the live chart with a preview is shown in night mode
- **THEN** the night pixel check finds no pixel with any green or blue, and none redder than the
  ink beyond two levels of compositing rounding

### Requirement: Calibrating the aim point

The Setup page SHALL let the observer, with a star centred in the eyepiece, tap that star on its
live frame, and the console SHALL set the aim point to the detected centroid nearest the tap
within 10 px, keeping it until it is reset. A tap with no centroid within 10 px SHALL set nothing and
say why.
**Refs**: GLOSSARY.md (aim point); Cedar's and StarSense's calibration (research note §2)

#### Scenario: The simulator's offset recovered
- **WHEN** the simulated eyepiece is centred on a star and the observer taps 6 px from that star
  on the live frame
- **THEN** the aim point becomes (700, 420) within 0.5 px, and later pointings are the
  simulator's true eyepiece direction within 30″

#### Scenario: A tap on empty sky
- **WHEN** the tap is more than 10 px from every centroid
- **THEN** the aim point is unchanged and the page says no star is close enough to the tap

#### Scenario: Reset
- **WHEN** the aim point is reset
- **THEN** pointings are at the frame centre again

### Requirement: Fresh pointings only

A pointing from the live loop SHALL go stale for push-to guidance once it is more than 5 s old
by the console's clock, whether or not the node is still connected, and the guidance SHALL then
say that the pointing is stale instead of giving a turn and tilt.
**Refs**: push-to "Turn and tilt"; scope-link "Link health"

#### Scenario: The node stops sending
- **WHEN** the last live pointing is 5.1 s old by the console's clock and the node is still
  connected
- **THEN** the guidance gives no turn or tilt and says the pointing is stale, and gives them
  again with the next solve

#### Scenario: The node goes away
- **WHEN** the link drops and the last live pointing then passes 5 s old
- **THEN** the guidance still gives no turn or tilt, and says the pointing is stale
