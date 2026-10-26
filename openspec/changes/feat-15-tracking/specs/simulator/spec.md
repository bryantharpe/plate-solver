## ADDED Requirements

### Requirement: A rate error

The simulated mount SHALL take a rate-error setting, a fraction by which every axis runs faster
(or slower, when negative) than commanded, zero by default.
**Refs**: GLOSSARY.md (simulator); tracking "Closed by the finder camera"

#### Scenario: One per cent fast
- **WHEN** the rate error is 1% and an axis is commanded at 0.01°/s for 100 s of virtual time
- **THEN** it turns 1.01° within 0.001°

### Requirement: A backlash setting

The simulated mount SHALL take a backlash setting for each axis, 0.3° by default.
**Refs**: GLOSSARY.md (simulator); tracking "Backlash at reversals"

#### Scenario: No backlash
- **WHEN** an axis's backlash is set to 0 and it goes up 1° and then down 1°
- **THEN** its true direction returns to where it started within 1″

### Requirement: A known model for tests

Tests SHALL be able to load the simulator's true base tilt and zero points into the console's
pointing model.
**Refs**: pointing-model "Zero points and the base tilt"; GLOSSARY.md (simulator)

#### Scenario: The truth loaded
- **WHEN** the simulator's truth (base 1° off level toward 130°, zero points offset) is loaded
  into the console's model
- **THEN** the model's prediction for axis altitude 45°, azimuth 120° equals the simulator's true
  eyepiece direction there within 1″
