## ADDED Requirements

### Requirement: Apparent altitude

The system SHALL add refraction to a true (geometric) altitude by Saemundsson's formula and
remove it from an apparent altitude by Bennett's, for the standard atmosphere (1010 hPa, 10 °C),
and SHALL leave the finder console's altitude and azimuth geometric.
**Refs**: Meeus, Astronomical Algorithms 2nd ed., ch. 16 (eqs. 16.3, 16.4); docs/research/2026-10-21-pointing-model-and-tracking.md §4

#### Scenario: At the horizon
- **WHEN** Bennett's refraction is computed for an apparent altitude of 0°
- **THEN** it is 34.5′ ± 0.1′

#### Scenario: At 45°
- **WHEN** Saemundsson's refraction is computed for a true altitude of 45°
- **THEN** it is 1.01′ ± 0.01′

#### Scenario: There and back
- **WHEN** refraction is added to every true altitude from 0° to 90° in 0.1° steps and then
  removed
- **THEN** each comes back within 0.1′

#### Scenario: Geometric stays geometric
- **WHEN** Meeus's example 13.b is computed after this change
- **THEN** the altitude is still 15.1249° within 0.01°, with no refraction in it
