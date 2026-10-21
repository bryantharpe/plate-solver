## Purpose

The fitted relation between the mount's axis positions and where the telescope points on the
sky, built from plate solves instead of a hand controller's star alignment and refined by every
sync, so the console can turn a target into axis positions.

## ADDED Requirements

### Requirement: Every solve is a sync

The console SHALL take as a sync each solved star frame from the scope node that carries axis
positions, when both axes moved at no more than 0.2°/s during its exposure: the apparent
altitude and azimuth at the aim point at mid-exposure, with the axis positions interpolated to
the same moment. Syncs SHALL be thinned to one per 2° of sky per 2 minutes.
**Refs**: GLOSSARY.md (sync, axis position); docs/research/2026-10-21-pointing-model-and-tracking.md §1–2

#### Scenario: A still telescope
- **WHEN** the simulated mount is still and frames solve at 2 a second for 10 s
- **THEN** one sync is kept, and its axis positions equal the simulator's at mid-exposure within
  one count

#### Scenario: During a slew
- **WHEN** a frame is exposed while an axis turns at 2°/s
- **THEN** it is solved and charted but not taken as a sync

#### Scenario: A frame uploaded by hand
- **WHEN** a frame is uploaded to `/api/solve` while the mount is connected
- **THEN** it is solved and becomes the latest pointing, but is not taken as a sync

#### Scenario: Interpolated to mid-exposure
- **WHEN** a frame is exposed while the azimuth axis tracks from count 1000 to count 1060
- **THEN** the sync's azimuth axis position is count 1030

### Requirement: Zero points and the base tilt

The model SHALL fit the azimuth and altitude zero points (IA, IE) from one sync, and the base
tilt (AN, AW) once it holds two syncs at least 90° apart in azimuth, in TPoint's alt-az terms
with refraction removed first.
**Refs**: TPoint manual v18.20 §7.2; Taki (1989); docs/research/2026-10-21-pointing-model-and-tracking.md §1

#### Scenario: Synthetic terms recovered
- **WHEN** syncs are made from synthetic axis positions with IA 2°, IE −0.5°, AN 0.6° and AW
  −0.8°, two of them 90° apart in azimuth
- **THEN** the fitted IA, IE, AN and AW each equal the synthetic value within 1″

#### Scenario: One sync is not enough
- **WHEN** the base is tilted 1° toward azimuth 0°, the only sync is at azimuth 0°, altitude 45°,
  and the model predicts azimuth 180°, altitude 45°
- **THEN** its error there is 2.00° ± 0.01°, twice the tilt, as the research notes predict

### Requirement: Slips

A sync more than 0.5° from the model's prediction SHALL open a new zero-point epoch: the zero
points SHALL be fitted again from that sync and later ones, keeping the base tilt and the fixed
terms.
**Refs**: docs/research/2026-10-21-synscan-mount-control.md §1 (Freedom Find), §5

#### Scenario: Pushed by hand
- **WHEN** the simulated tube is pushed 3° by hand with the auxiliary encoders off, a frame
  solves, and a GoTo is then made
- **THEN** a new epoch is opened at that sync, and the GoTo still ends with the true eyepiece
  within 1′ of the target

### Requirement: Fixed terms

The model SHALL fit the collimation error (CA), the axes' non-perpendicularity (NPAE), the tube's
flexure (TF) and the finder camera's mounting on the tube once it holds at least 20 syncs
spanning 180° of azimuth and 30° of altitude, and keep them across nights until they are reset.
**Refs**: TPoint manual v18.20 §4.1, §7.2; docs/research/2026-10-21-pointing-model-and-tracking.md §1

#### Scenario: Synthetic fixed terms
- **WHEN** 24 synthetic syncs spanning 200° of azimuth and 40° of altitude carry CA 10′, NPAE 3′,
  TF 5′ and the finder camera turned 4° about its axis on the tube, besides the zero points and
  tilt
- **THEN** CA, NPAE, TF and the camera's turn are each fitted within 1″

#### Scenario: Too few
- **WHEN** only 19 such syncs are held
- **THEN** no fixed term is fitted and the model says how many more syncs it needs

### Requirement: One-tap second sync

On request, the console SHALL slew to a pointable spot at least 90° in azimuth from the night's
syncs and outside the Sun guard, solve there, and take the sync, so the base tilt can be fitted.
**Refs**: GLOSSARY.md (sync); docs/research/2026-10-21-pointing-model-and-tracking.md §5

#### Scenario: From one sync to tilt
- **WHEN** the simulator holds one sync at azimuth 120° and the second sync is requested
- **THEN** the mount slews to a pointable spot whose azimuth differs from 120° by at least 90°,
  a sync is taken there, and the base tilt is fitted within 1′ of the simulator's
