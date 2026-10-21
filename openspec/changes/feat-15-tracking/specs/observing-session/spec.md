## ADDED Requirements

### Requirement: Track in the session

The session SHALL hold whether tracking is wanted, set by `POST /api/track` with `on` true or
false, and report the tracking state and why it last ended. Turning it on SHALL answer 409 with a
JSON `error`, naming the reason, unless a mount is connected, the night has a sync, the Sun is
down and the target is pointable.
**Refs**: tracking; goto "Pointable targets only"; observing-session "Same-origin writes"

#### Scenario: On after a GoTo
- **WHEN** Track is turned on and a GoTo to M42 completes on the simulator
- **THEN** tracking starts within 2 s of the end of the GoTo, and the session reports it
  tracking M42

#### Scenario: Refused
- **WHEN** Track is turned on with M8 below the horizon as the target
- **THEN** the answer is 409 saying why, and nothing moves

### Requirement: The mount's backlash in the session

The session SHALL keep each axis's measured backlash across restarts and nights, with the fixed
terms, and SHALL clear it when the fixed terms are reset.
**Refs**: tracking "Feed-forward rates"; observing-session "The model's state for the night"

#### Scenario: Measured once, kept
- **WHEN** the altitude axis's backlash has been measured on the simulator and the binary
  restarts on the same `--state` folder the next day
- **THEN** the session holds the same backlash for that axis exactly

### Requirement: The imaging camera in the session

The session SHALL hold the imaging camera's sensor width and height in millimetres and its pixel
pitch in micrometres, each more than 0 and at most 100 mm or 50 µm, kept across restarts.
**Refs**: GLOSSARY.md (imaging camera); sky-ephemeris "The longest exposure"

#### Scenario: Kept
- **WHEN** a 23.5 × 15.6 mm sensor with 3.76 µm pixels is put and the binary restarts
- **THEN** the session holds the same three values exactly

#### Scenario: An impossible pixel pitch
- **WHEN** a pixel pitch of 0 µm is put
- **THEN** the answer is 400 with a JSON `error`, and nothing changes
