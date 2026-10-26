## ADDED Requirements

### Requirement: Alt-az rates

The system SHALL compute, for a position at a site and time, the rates of its altitude and
azimuth, and of field rotation (the parallactic angle), from the spherical relations of Meeus
ch. 13 and 14: the altitude and azimuth rates in arcseconds per second and field rotation in
degrees per hour, positive when the parallactic angle increases.
**Refs**: Meeus ch. 13, 14; SLALIB sla_ALTAZ; docs/research/2026-10-21-pointing-model-and-tracking.md §3

#### Scenario: At transit
- **WHEN** the rates are computed at latitude 40°, declination 20°, hour angle 0
- **THEN** the altitude rate is 0 (within 0.01″/s), the azimuth rate 41.33″/s (within
  0.02″/s) and the field rotation +33.69°/h, the parallactic angle increasing (within 0.02°/h)

#### Scenario: Two hours before transit
- **WHEN** they are computed at hour angle −30°
- **THEN** the altitude rate is +10.07″/s and the azimuth rate 18.45″/s, each within 0.02″/s

### Requirement: The longest exposure

The system SHALL compute the longest exposure an imaging camera allows before field rotation
smears the frame's corner by a given number of pixels, from its sensor size and pixel pitch, and
SHALL say there is no limit where field rotation is zero.
**Refs**: GLOSSARY.md (imaging camera); docs/research/2026-10-21-pointing-model-and-tracking.md §3

#### Scenario: A 23.5 × 15.6 mm sensor at transit
- **WHEN** the longest exposure for 1 px of smear is computed for a 23.5 × 15.6 mm sensor with
  3.76 µm pixels at latitude 40°, declination 20°, hour angle 0
- **THEN** the corner radius is 3,751 px and the exposure 1.63 s, within 0.01 s

#### Scenario: Due east
- **WHEN** it is computed at latitude 40° for a target at altitude 30°, azimuth 90° (due east),
  and again at azimuth 270°
- **THEN** both answers say that field rotation sets no limit (|dq/dt| below 1e-9 rad/s)
