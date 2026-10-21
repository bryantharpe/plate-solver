## Purpose

Getting from where the telescope points to the target by hand: whether the target is pointable
now and why not, when a target that is down comes up, the turn and tilt from the latest pointing,
and where the target lies on or off the solved frame.

## ADDED Requirements

### Requirement: Pointable

The system SHALL class the session's target at the console's time by precedence: under the Sun
guard (centre within 15° of the Sun's), below the horizon (altitude below 0°), too low (below the
minimum altitude), or else pointable. Without a site it SHALL still apply the Sun guard, and
otherwise say that a site is needed. The Sun guard SHALL be always on, with no override.
**Refs**: GLOSSARY.md (pointable, Sun guard, minimum altitude); sky-ephemeris "The Sun"

#### Scenario: A January evening
- **WHEN** the site is 40° N, 75° W, the time 2026-01-16 02:00 UT and the minimum altitude 15°
- **THEN** M42 (altitude 42.9°) is pointable, M8 (−66.6°) is below the horizon, and M75, 3.8°
  from the Sun though also below the horizon, is under the Sun guard

#### Scenario: A higher minimum altitude
- **WHEN** the minimum altitude is 50° at the same site and time
- **THEN** M42 is too low

#### Scenario: No site
- **WHEN** the session has no site at that time
- **THEN** M75 is under the Sun guard and M42 is classed as needing a site

### Requirement: When a target that is down comes up

For a target below the horizon or too low, the system SHALL give when it next rises to the
minimum altitude, when it next transits and when it then sets below the minimum altitude, or say
that it never reaches the minimum altitude on that date.
**Refs**: sky-ephemeris "Rising, transit and setting"; GLOSSARY.md (minimum altitude)

#### Scenario: M8 at 15°
- **WHEN** M8 is the target at 40° N, 75° W at 2026-01-16 02:00 UT with a minimum of 15°
- **THEN** its rising time is later than that time, and at the rising time the
  finder console's own altitude of M8 is 15° within 0.05°, and climbing

#### Scenario: M8 at 30°
- **WHEN** the minimum altitude is 30° (M8 culminates at about 25.6° from 40° N)
- **THEN** the answer says M8 never reaches the minimum altitude that day, and gives its transit

### Requirement: Turn and tilt

For a pointable target and a latest pointing that has an altitude and azimuth and has not gone
stale, the system SHALL give the turn as the target's azimuth less the pointing's, wrapped to
more than −180° and at most 180°, clockwise (seen from above) when positive, and the tilt as the
target's altitude less the pointing's, up when positive. Otherwise it SHALL give neither.
**Refs**: GLOSSARY.md (push-to, turn, tilt); Cedar's push-to directions (research note §2)

#### Scenario: Across north
- **WHEN** the pointing is at azimuth 350°, altitude 30° and the target at azimuth 24°,
  altitude 42°
- **THEN** the guidance is to turn clockwise 34° and tilt up 12° (within 1e-9°), shown as "turn
  clockwise 34° · tilt up 12°"

#### Scenario: The other way
- **WHEN** the pointing is at azimuth 20°, altitude 50° and the target at azimuth 350°,
  altitude 41°
- **THEN** the guidance is to turn anticlockwise 30° and tilt down 9° (within 1e-9°)

#### Scenario: Not pointable
- **WHEN** the target is M8 below the horizon and a pointing exists
- **THEN** no turn or tilt is given

### Requirement: A still telescope stays put

The system SHALL hold the latest pointing's altitude and azimuth at their values at its solve
time, since an alt-az telescope left alone stays put, while the target's altitude and azimuth
follow the console's clock.
**Refs**: GLOSSARY.md (push-to, tracking); docs/research/2026-10-21-pointing-model-and-tracking.md §3

#### Scenario: Ten minutes later
- **WHEN** orion2 is solved at 2026-01-16 02:00 UT at 40° N, 75° W with M78 as the target, and
  the guidance is read at 02:10 UT
- **THEN** the turn and tilt start from the pointing's altitude and azimuth at 02:00 (within
  1e-9°) and end at M78's at 02:10 (within 1e-9°), so they differ from the 02:00 guidance by
  M78's own motion

### Requirement: Placed on the frame

For the latest pointing's frame, the system SHALL place the target as it places deep-sky objects
when its symbol overlaps the frame (position, axes and angle in the frame's pixels), and
otherwise SHALL give its direction from the frame's centre, in degrees clockwise from up, and its
distance.
**Refs**: finder-console "Deep-sky objects on and near the frame"

#### Scenario: A target on the frame
- **WHEN** orion2 is the latest pointing and M78 the target
- **THEN** the target's placement equals the solve report's mark for M78 (position, axes and
  angle within 1e-9)

#### Scenario: A target off the frame
- **WHEN** orion2 is the latest pointing and M79 the target
- **THEN** the target is off the frame, with the direction and distance the solve report gives
  M79 among the objects just off the frame (within 1e-9°)
