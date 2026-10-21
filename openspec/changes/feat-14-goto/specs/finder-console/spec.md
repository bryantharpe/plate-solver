## ADDED Requirements

### Requirement: GoTo and Stop on the console

While a mount is connected, the guidance block SHALL offer GoTo, enabled only when a GoTo would
be accepted and otherwise saying why not, and SHALL show its progress and result. Whenever the
mount may be moving, every page SHALL show Stop in its top bar, always enabled, at least 44 px
tall and never covered by a sheet or another control.
**Refs**: goto (every requirement); DESIGN.md (Buttons; touch targets); GLOSSARY.md (GoTo)

#### Scenario: A GoTo from the page
- **WHEN** the observer taps GoTo with M42 as the target on `finder --simulate`
- **THEN** the block shows the slew, each correction and "on target" with the final error, and
  the chart's target reticle ends within 1′ of the aim point

#### Scenario: Stop from the page
- **WHEN** the observer taps Stop mid-slew
- **THEN** the simulated axes stop and the block says the observer stopped the GoTo

#### Scenario: Not offered
- **WHEN** the target is below the horizon, or the Sun is up, or the night has no sync yet
- **THEN** GoTo is disabled and the block says why, and Stop is still shown if the mount moves

#### Scenario: Stop from Setup
- **WHEN** the second sync is slewing from the Setup page
- **THEN** Stop is in Setup's top bar, and tapping it stops the simulated axes

### Requirement: Setup's mount and model section

The Setup page SHALL have a mount and model section showing the mount's link and board, the
base tilt ("tilt 0.8° toward 130°"), the model's RMS residual and its number of syncs, with the
one-tap second sync, and resets for the night's model and for the fixed terms.
**Refs**: pointing-model (every requirement); observing-session "The model's state for the night"

#### Scenario: After two syncs
- **WHEN** the simulator (base 1° off level toward azimuth 130°) holds two syncs 90° apart
- **THEN** Setup reads "tilt 1.0° toward 130°" within 0.1° and 2° of direction, with the RMS and
  "2 syncs"

#### Scenario: The second sync from Setup
- **WHEN** the observer taps the second sync with one sync held
- **THEN** the mount slews, a second sync is taken, and the tilt appears
