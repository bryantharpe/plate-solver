## Purpose

The mount following the target across the sky so that it stays at the aim point: rates from the
pointing model, corrected by the finder camera's solves, through the zenith, and ending safely.

## ADDED Requirements

### Requirement: Feed-forward rates

While tracking, the console SHALL command each axis at the rate that keeps the target's apparent
place at the aim point through the pointing model, recomputed and sent at least once a second.
**Refs**: sky-ephemeris "Alt-az rates"; pointing-model; docs/research/2026-10-21-synscan-mount-control.md §3 (the host computes alt-az tracking)

#### Scenario: Rates follow the sky
- **WHEN** M42 is tracked on a simulator with no rate error, its backlash set to 0 and its true
  base tilt and zero points loaded into the console's model, for 10 minutes of virtual time
- **THEN** a new rate is sent to each axis at least once a second, and the true eyepiece stays
  within 10″ of M42 throughout

### Requirement: Backlash at reversals

The console SHALL measure each axis's backlash at that axis's first reversal on this mount, and
before each later reversal of either axis, forecast from its feed-forward rate changing sign,
SHALL stop that axis and take up the measured backlash.
**Refs**: design.md (Backlash at reversals); simulator "A backlash setting"; docs/research/2026-10-21-synscan-mount-control.md §3

#### Scenario: The first reversal measures the backlash
- **WHEN** a target is tracked through its transit on the simulator (0.3° of backlash) before
  any backlash has been measured on this mount
- **THEN** the altitude axis stops before it reverses, the target is back within 30″ of the aim
  point within 15 s of the reversal, and the backlash measured there is 0.3° within 30″

#### Scenario: Later reversals take it up
- **WHEN** another target is tracked through its transit on the same simulated mount
- **THEN** the altitude axis stops, takes up the measured backlash and reverses, and the target
  stays within 30″ of the aim point through the reversal

#### Scenario: An azimuth reversal
- **WHEN** M81 (Dec +69°) is tracked at latitude 40° through its greatest elongation (hour angle
  about 43.5°, where its azimuth rate changes sign) on the simulator, after the azimuth axis's
  backlash has been measured
- **THEN** the azimuth axis stops, takes up the measured backlash and reverses, and the target
  stays within 30″ of the aim point through the reversal

### Requirement: Closed by the finder camera

While tracking, the console SHALL correct the axis rates from at least one solve every 5 s, with
proportional-integral control on the error between the target and the aim point, resolved into
the two axes, ignoring errors under 10″: inside that deadband the held integral SHALL keep
applying but SHALL NOT be updated.
**Refs**: docs/research/2026-10-21-pointing-model-and-tracking.md §3 (Wallace 2002; PHD2's algorithms)

#### Scenario: A mount that runs fast
- **WHEN** M42 is tracked for 10 minutes of virtual time on a simulator whose axes run 1% faster
  than commanded, first with feed-forward alone, then with the finder camera's corrections, the
  pointings being the simulator's truth plus 3″ of noise every 2 s
- **THEN** feed-forward alone drifts more than 1′ (about 90″), and with the corrections the RMS
  error over the last 8 minutes is at most 20″

#### Scenario: Inside the deadband
- **WHEN** M42 is tracked for 2 minutes of virtual time on a simulator with no rate error,
  starting with a zero integral, and every pointing's error is under 10″
- **THEN** each axis's commanded rate equals the feed-forward rate within 0.001″/s, and the
  tracking offset stays 0

#### Scenario: Rendered frames agree with truth plus noise
- **WHEN** M42 is tracked for the same 2 minutes of virtual time on the simulator twice, once on
  rendered and solved frames and once on its truth plus 3″ of noise every 2 s
- **THEN** the two runs' tracking errors agree within 5″ RMS

### Requirement: The zenith keyhole

When the target's path will come within 2° of the zenith, the console SHALL stop following it at
that circle, slew ahead to where the target will leave the circle, and on its exit solve and
resume tracking.
**Refs**: Borkowski (1987), via docs/research/2026-10-21-pointing-model-and-tracking.md §3

#### Scenario: A target passing 1° from the zenith
- **WHEN** a target at declination φ − 1° is tracked at latitude φ = 40° through its transit
- **THEN** tracking pauses where the target enters the 2° circle, the telescope waits where it
  will leave, and within 30 s of the target leaving the target is back within 1′ of the aim point

### Requirement: Ending safely

Tracking SHALL end, leaving both axes stopped, on Stop, a lost link, the Sun's centre rising
above −0.83°, or the target's altitude (geometric, as pointable is judged) sinking below the
minimum altitude, and the console SHALL report which.
**Refs**: goto "Halts"; push-to "Pointable"; GLOSSARY.md (Sun guard, minimum altitude)

#### Scenario: Setting below the minimum altitude
- **WHEN** a target is tracked as it sets through the minimum altitude of 15°
- **THEN** tracking ends with both axes stopped, within 1 s of the target's geometric altitude
  reaching 15°, saying the target is too low

#### Scenario: Stop while tracking
- **WHEN** Stop is sent while tracking
- **THEN** both axes stop within 0.5 s of virtual time and tracking reports the observer stopped
  it

#### Scenario: Link lost while tracking
- **WHEN** the simulator's link stops answering heartbeats while M42 is tracked
- **THEN** the node stops both axes 1.0 ± 0.1 s after the last heartbeat by its watchdog,
  tracking ends saying the link was lost, and it does not resume when the link returns

#### Scenario: Sunrise
- **WHEN** the virtual clock passes the moment the Sun's centre reaches −0.83° while tracking
- **THEN** both axes are commanded to stop within 1 s of that moment, and tracking ends saying
  the Sun is up
