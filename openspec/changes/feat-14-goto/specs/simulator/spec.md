## ADDED Requirements

### Requirement: The simulated mount

The simulator SHALL have a mount behind the mount port whose base is 1° off level by default,
whose axis zero points are offset from true north and level, with 0.3° of backlash on each axis,
a top rate of 3.4°/s, and acceleration and deceleration of 3°/s², and SHALL expose its true
eyepiece direction to tests.
**Refs**: GLOSSARY.md (simulator); docs/research/2026-10-21-synscan-mount-control.md §1 (top speed); docs/research/2026-10-21-pointing-model-and-tracking.md §1 (base tilt)

#### Scenario: Backlash
- **WHEN** a simulated axis goes from −1° up to 0°, on up to +1°, and back down to 0°
- **THEN** its axis position reads 0° both times it passes there, and the true directions at
  those two moments differ by 0.3° within 0.01°

#### Scenario: Top speed
- **WHEN** a simulated axis is sent 90° away
- **THEN** it never exceeds 3.4°/s and arrives 27.6 s ± 0.1 s later (90/3.4 + 3.4/3, accelerating
  and slowing at 3°/s²)

#### Scenario: Stopping from top speed
- **WHEN** a simulated axis turning at 3.4°/s is told to stop
- **THEN** it is at rest 1.13 s later (±0.05 s), having slowed at 3°/s²

#### Scenario: The base is off level
- **WHEN** the simulated tube is pointed at axis altitude 45° at axis azimuths 0° and 180° (the
  base tilt toward 0°)
- **THEN** the true altitudes differ by 2° within 0.01°

### Requirement: Pushes and slips on the simulated mount

With the mount connected, the simulator's push by hand SHALL move the true direction without
changing the axis positions, as on a Dob with its auxiliary encoders off, and a test SHALL be
able to make each axis slip by a set angle on every move.
**Refs**: GLOSSARY.md (simulator); pointing-model "Slips"; docs/research/2026-10-21-synscan-mount-control.md §1 (Freedom Find)

#### Scenario: A push the encoders miss
- **WHEN** the simulated tube is pushed 3° in azimuth with the mount connected
- **THEN** the true direction moves 3° (within 0.01°) and both axis positions are unchanged

#### Scenario: A slipping axis
- **WHEN** a slip of 0.5° per move is set on the azimuth axis and it is sent 10° away
- **THEN** its axis position reads 10° and its true direction has turned 9.5° (within 0.01°)
