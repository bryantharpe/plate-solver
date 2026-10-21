## ADDED Requirements

### Requirement: Track on the console

While a mount is connected, the guidance block SHALL offer a Track toggle, and while tracking SHALL
read out the tracking error (RMS over the last minute, in arcseconds), the field rotation (°/min)
and, with an imaging camera set, the longest exposure for 1 px of smear, or why tracking ended.
**Refs**: tracking; sky-ephemeris "Alt-az rates", "The longest exposure"; DESIGN.md (Switch)

#### Scenario: Tracking M42
- **WHEN** Track is on for M42 on `finder --simulate` with a 23.5 × 15.6 mm, 3.76 µm imaging camera
- **THEN** the block reads the error in arcseconds, the field rotation and the longest exposure,
  each matching the session's values as rounded for display

#### Scenario: Tracking ended
- **WHEN** tracking ends because the target sank below the minimum altitude
- **THEN** the toggle is off and the block says the target is too low

### Requirement: Setup's imaging camera section

The Setup page SHALL have an imaging camera section that sets the sensor's width and height in
millimetres and its pixel pitch in micrometres, refusing values out of range with a message.
**Refs**: observing-session "The imaging camera in the session"; DESIGN.md (Inputs)

#### Scenario: A camera set on Setup
- **WHEN** 23.5, 15.6 and 3.76 are entered and saved
- **THEN** the session holds them, and the guidance block's readout gains the longest exposure

#### Scenario: A pitch of zero
- **WHEN** a pitch of 0 is entered
- **THEN** nothing is saved and the page says the allowed range
