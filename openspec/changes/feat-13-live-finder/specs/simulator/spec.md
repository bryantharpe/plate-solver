## Purpose

A stand-in scope node for a desk or CI: a simulated telescope with known optics and a virtual
clock, whose finder camera renders the sky from the pattern database's stars, served over the
same link as the real node so the console cannot tell them apart.

## ADDED Requirements

### Requirement: The same surface as a scope node

The simulator SHALL serve the scope link as a scope node does, with a camera adapter that renders
8-bit frames of the pattern database's stars for wherever its simulated telescope points at its
site and virtual time, with pixel noise, and detected as the node detects them.
**Refs**: GLOSSARY.md (simulator); scope-link (every requirement); pattern-database (the star table, brightest first)

#### Scenario: A frame at altitude 50°, azimuth 120°
- **WHEN** the simulated finder camera points at altitude 50°, azimuth 120° from 40° N, 75° W at
  2026-01-16 02:00 UT, and its frame is solved by the console
- **THEN** the solved centre is within 30″ of the camera's true direction

#### Scenario: Indistinguishable on the link
- **WHEN** the console's link client connects to the simulator
- **THEN** it pairs, streams frames, maps the clock and sends settings through exactly the calls
  it uses with a real node

### Requirement: Known optics and an aim-point offset

The simulator SHALL have a known finder camera (a sensor size, and a horizontal field that is
inside the bundled database's range by default and can be set) and an eyepiece direction offset
from the camera's centre to a known pixel, (700, 420) by default, and SHALL expose its true
camera and eyepiece directions to tests.
**Refs**: GLOSSARY.md (aim point, boresight); docs/research/2026-10-21-pointing-model-and-tracking.md §2 (the eyepiece's spot in the finder camera's frame)

#### Scenario: Where the eyepiece looks
- **WHEN** the simulated telescope is placed so that a star lies in its eyepiece's centre
- **THEN** that star's centroid in the next frame is at (700, 420) within 0.3 px

### Requirement: Pushed by hand

The simulator SHALL offer, only under `finder --simulate`, a control that moves its simulated
telescope by hand, to an altitude and azimuth or by a turn and tilt, as an observer pushes a
Dobsonian.
**Refs**: GLOSSARY.md (push-to, turn, tilt)

#### Scenario: A push
- **WHEN** the simulated telescope is pushed to altitude 40°, azimuth 200°
- **THEN** the next frame solves within 30″ of that direction

#### Scenario: Not without the simulator
- **WHEN** the console runs without `--simulate` and a client asks for the simulator's control
- **THEN** the route answers 404 and nothing moves

### Requirement: `finder --simulate`

`finder --simulate` SHALL run the console with an in-process simulator as its scope node, on a
virtual clock that starts at `--sim-start` (default now) and runs at the real rate, and SHALL
never connect to a real scope node, whatever address the session holds.
**Refs**: observing-session "The console's time"; GLOSSARY.md (simulator)

#### Scenario: A night in January, at any hour
- **WHEN** `finder --simulate --sim-start 2026-01-16T02:00:00Z` starts and its session is read
- **THEN** the session's time is 2026-01-16 02:00 UT within 2 s, and frames arrive from the
  simulator

#### Scenario: Never the real node
- **WHEN** the session holds the address of a listening test server and `finder --simulate` runs
  for 5 s
- **THEN** that server receives no connection
