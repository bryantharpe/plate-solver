## Purpose

Bringing the target to the aim point with the mount's motors: planning each slew through the
pointing model, keeping the Sun guard on every motion, closing the loop with plate solves until
the target is within 1′, and halting safely.

## ADDED Requirements

### Requirement: Planning a slew

The console SHALL plan each slew to the target's apparent place (refraction added) at the
expected arrival time, through the pointing model, to axis positions that keep the cable within
one revolution either way of the night's first sync, measured as the telescope's azimuth
unwrapped continuously across syncs and zero-point epochs.
**Refs**: pointing-model; sky-ephemeris "Apparent altitude"; docs/research/2026-10-21-synscan-mount-control.md §1 (cable wrap)

#### Scenario: Unwinding the cable
- **WHEN** the azimuth axis has turned 350° clockwise since the first sync and the target is at
  azimuth 20°, 30° clockwise of where the telescope points
- **THEN** the slew turns anticlockwise 330°, not clockwise 30°

#### Scenario: Unwinding after a power cycle
- **WHEN** the telescope has turned 350° clockwise since the first sync, the simulated mount is
  power-cycled (its counts reset), a sync opens a new zero-point epoch, and the target is 30°
  clockwise of where the telescope points
- **THEN** the slew turns anticlockwise 330°

#### Scenario: Where the target will be
- **WHEN** a slew is expected to take 40 s
- **THEN** it is planned to the target's apparent place 40 s after it starts (within 1 s of
  planning time)

### Requirement: Above the minimum altitude all the way

Every point of a planned slew, including the last approach and each correction, SHALL stay at or
above the minimum altitude; when the last approach from below would go under it, the GoTo SHALL
be refused with the reason.
**Refs**: GLOSSARY.md (minimum altitude); design.md (Plan: the approach from below; Mechanical limits)

#### Scenario: Too close to the limit
- **WHEN** the minimum altitude is 15° and a GoTo is asked for a target at 15.2° on the simulator
- **THEN** it is refused with 409 and a JSON `error` saying the approach would go below the
  minimum altitude, and no axis moves

#### Scenario: Just clear of the limit
- **WHEN** the minimum altitude is 15° and a GoTo is made to a target at 16° on the simulator
- **THEN** the simulator's true eyepiece altitude, sampled every 0.1 s of virtual time, never goes
  below 15°

### Requirement: The Sun guard on motion

The console SHALL NOT move the mount while the Sun's centre is above −0.83° (sunrise and
sunset as Meeus defines them), nor along a path that comes within 15° of the Sun. It SHALL
re-plan a path that would, by moving the axes one at a time in the other order or through a
higher altitude, and refuse the slew when no path keeps clear.
**Refs**: GLOSSARY.md (Sun guard); Meeus ch. 15 (h₀ = −0°50′ for the Sun); sky-ephemeris "The Sun"

#### Scenario: Daylight
- **WHEN** a GoTo is asked for while the Sun's centre is 5° above the horizon
- **THEN** it is refused with 409 and a JSON `error` saying the Sun is up, and no axis moves

#### Scenario: A slew at dusk
- **WHEN** the Sun is 10° below the western horizon, the telescope points 3° up and 30° south
  of the Sun's azimuth, and the target is 40° up and 30° north of it
- **THEN** the GoTo completes, and the simulator's true eyepiece direction, sampled every 0.1 s
  of virtual time, never comes within 15° of the Sun

### Requirement: Closed loop to 1′

A GoTo SHALL slew at the mount's top speed, make its last approach on each axis from one fixed
direction, then solve at the aim point and correct, at most 5 times, until the target is within
1′ of the aim point, and otherwise SHALL end and report that it failed, with the last error.
**Refs**: GLOSSARY.md (GoTo, slew); N.I.N.A.'s and Ekos's centring loops (research note §2)

#### Scenario: Five targets on the simulator
- **WHEN** GoTos to M42, M31, M45, M81 and M1 are made in turn on the simulator (base 1° off
  level, zero points offset, 0.3° backlash) after a second sync, at 40° N, 75° W from
  2026-01-16 02:00 UT (`--sim-start 2026-01-16T02:00:00Z`) with a minimum altitude of 15°
- **THEN** each ends with the simulator's true eyepiece direction within 1′ of its target, using
  at most 5 corrections

#### Scenario: A GoTo that cannot converge
- **WHEN** the simulated mount is made to slip 0.5° on every correction
- **THEN** after 5 corrections the GoTo ends as failed, reporting the last error, and the axes
  are stopped

### Requirement: Halts

Stop, a lost link and the Sun guard SHALL each leave both axes stopped, and the console SHALL
report which ended the motion. `POST /api/stop` SHALL be accepted from any origin, with no body,
and SHALL stop both axes whatever else is under way.
**Refs**: scope-node "Stop wins, and a watchdog"; GLOSSARY.md (Sun guard)

#### Scenario: Stop during a slew
- **WHEN** `POST /api/stop` is sent mid-slew with a foreign `Origin` and no body
- **THEN** it is answered 200, the node commands both simulated axes to stop within 0.1 s of
  virtual time, both are at rest within 1.5 s, and the GoTo reports that the observer stopped it

#### Scenario: The link drops during a slew
- **WHEN** the console stops sending heartbeats mid-slew
- **THEN** the node commands both axes to stop 1.0 ± 0.1 s after the last heartbeat, and the
  console reports the GoTo halted by a lost link

#### Scenario: Sunrise during a slew
- **WHEN** the virtual clock passes the moment the Sun's centre reaches −0.83° mid-slew
- **THEN** both axes are commanded to stop within 1 s of that moment, and the GoTo reports the
  Sun guard

### Requirement: Pointable targets only

`POST /api/goto` SHALL start a GoTo to the session's target and report its progress with the
session, and SHALL answer 409 with a JSON `error` naming the reason when the target is not
pointable, no mount is connected, the night has no sync yet, or the Sun is up.
**Refs**: push-to "Pointable"; observing-session "Same-origin writes"

#### Scenario: Below the horizon
- **WHEN** a GoTo to M8 is asked for at 40° N, 75° W at 2026-01-16 02:00 UT
- **THEN** the answer is 409 saying M8 is below the horizon, and no axis moves

#### Scenario: Progress
- **WHEN** a GoTo to M42 runs on the simulator at 40° N, 75° W from 2026-01-16 02:00 UT with a
  minimum altitude of 15°
- **THEN** the session reports it slewing, then solving, then each correction by number, then
  done with the final error in arcseconds
