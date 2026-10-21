## Purpose

Time-dependent sky geometry for the console beyond the finder console's altitude and azimuth:
where the Sun is, and when a position rises above, crosses the meridian and sets below a given
altitude, from the worked methods of Meeus's Astronomical Algorithms.

## ADDED Requirements

### Requirement: The Sun

The system SHALL compute the Sun's apparent right ascension and declination for any time, by
Meeus's low-accuracy method (ch. 25), to within 0.01° over the years 1950 to 2100.
**Refs**: Meeus, Astronomical Algorithms 2nd ed., ch. 25 and example 25.a

#### Scenario: Meeus's example 25.a
- **WHEN** the Sun is computed for 1992 October 13.0 TD (JD 2448908.5)
- **THEN** its apparent RA is 198.38083° and its Dec −7.78507°, each within 0.01°

### Requirement: Rising, transit and setting

The system SHALL compute, for a position, a site, a date and an altitude, when the position next
rises above that altitude, crosses the meridian and sets below it, each to within one minute, or
say that on that date it stays above or never reaches that altitude. A position that moves SHALL
be interpolated over the date as Meeus does.
**Refs**: Meeus, Astronomical Algorithms 2nd ed., ch. 15 and example 15.a

#### Scenario: Meeus's example 15.a
- **WHEN** Venus is computed at Boston (42.3333° N, 71.0833° W) for 1988 March 20, altitude
  −0.5667°, from the three positions Meeus gives
- **THEN** it rises at 12:25:26, transits at 19:40:31 and sets at 02:54:40 UT (Meeus's
  corrected 0.51766, 0.81980 and 0.12130 of a day), each within 1 minute

#### Scenario: Always above
- **WHEN** the times of altitude 15° are asked at 40° N for a position at Dec +80°
- **THEN** the answer says it stays above 15° all day, and still gives its transit

#### Scenario: Never that high
- **WHEN** the times of altitude 0° are asked at 40° N for a position at Dec −60°
- **THEN** the answer says it never rises above 0°, and still gives the time it is highest

#### Scenario: A crossing is a crossing
- **WHEN** a fixed position's rising time for altitude 15° is computed at 40° N, 75° W
- **THEN** the finder console's own altitude of that position at that time is 15° within 0.05°,
  and it is climbing
