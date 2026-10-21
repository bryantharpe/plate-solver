## ADDED Requirements

### Requirement: A rate error

The simulated mount SHALL take a rate-error setting, a fraction by which every axis runs faster
(or slower, when negative) than commanded, zero by default.
**Refs**: GLOSSARY.md (simulator); tracking "Closed by the finder camera"

#### Scenario: One per cent fast
- **WHEN** the rate error is 1% and an axis is commanded at 0.01°/s for 100 s of virtual time
- **THEN** it turns 1.01° within 0.001°
