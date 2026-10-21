## Purpose

The service that runs on the small computer riding on the telescope: it takes frames from the
finder camera through a camera port, finds their stars as the console would, keeps the star
count in range, and serves them over the scope link.

## ADDED Requirements

### Requirement: The camera port

The scope node SHALL take frames only through a camera port that reports the sensor's model,
size, modes, and exposure and gain ranges, and yields 8-bit gray frames, each with its exposure
start and end on the node's clock. Every camera SHALL be reached through an adapter behind this
port.
**Refs**: ADR-0001 (ports and adapters); GLOSSARY.md (camera port, camera adapter)

#### Scenario: The simulator's camera
- **WHEN** the simulator's camera adapter is asked what it is and for one frame
- **THEN** it reports its model, size and ranges, and yields an 8-bit gray frame of that size
  whose exposure end minus start equals the exposure asked for within 1 µs

#### Scenario: An exposure out of range
- **WHEN** an exposure longer than the reported range is asked for
- **THEN** the port refuses it with an error naming the range, and no frame is taken

### Requirement: The Arducam adapter

The Arducam adapter SHALL run `rpicam-vid` as a subprocess writing YUV420 frames and their
metadata, and take each frame's Y plane at the stride the camera reports, with exposure start
from the sensor timestamp and end from the exposure time. It SHALL read the camera's model and
modes from `rpicam-hello --list-cameras`.
**Refs**: Raspberry Pi camera documentation (rpicam-apps); libcamera's SensorTimestamp control; docs/research/2026-10-21-scope-node-runtime.md §2

#### Scenario: A saved camera listing
- **WHEN** a saved `rpicam-hello --list-cameras` output is parsed
- **THEN** the adapter reports the sensor model, every mode's size and the active mode exactly as
  listed

#### Scenario: A saved stream with padded rows
- **WHEN** a saved YUV420 stream whose row stride is wider than its width is read, with its
  saved metadata
- **THEN** each frame's gray image equals the saved Y plane byte for byte without the padding,
  and its exposure start and end equal the saved timestamp and timestamp plus exposure time

#### Scenario: The subprocess dies
- **WHEN** `rpicam-vid` exits during a stream
- **THEN** the adapter reports the error and its stderr, and the node restarts the camera
  without restarting the service

### Requirement: The same stars as the console

The scope node SHALL find stars with the same star detection the console uses, so that for the
same frame and the same detection settings the node's centroids equal the console's.
**Refs**: star-detection "Parity with cedar-detect"; plate-solver "Cedar's detection settings"

#### Scenario: One frame, two places
- **WHEN** a saved frame from the finder camera is detected by the node and by the console with
  cedar's settings
- **THEN** the two centroid lists are identical, value for value and in the same order

### Requirement: Auto-exposure

Unless the console has set the exposure and gain, the scope node SHALL adjust them so frames hold
25 to 40 detected stars, aiming at 30. Exposure and gain the console has set SHALL be held as
set.
**Refs**: docs/research/2026-10-21-pointing-model-and-tracking.md §2 (Cedar and PiFinder solve 10–30 times a second); plate-solver "Solve inputs and defaults"

#### Scenario: From too few stars
- **WHEN** the simulator starts at an exposure that gives 5 stars
- **THEN** within 6 frames the star count is between 25 and 40, and stays there while the field
  is unchanged

#### Scenario: Manual settings
- **WHEN** the console sets 200 ms and gain 4 and the field changes to one that gives 5 stars
- **THEN** every later frame is still at 200 ms and gain 4

### Requirement: A configured service

The scope node SHALL read its listen address, token, camera adapter and starting camera
settings from a config file, and SHALL refuse to start on an unknown key or a bad value, naming
it. `scope-node --check-config` SHALL print the config it would run with, the token masked, and
exit.
**Refs**: systemd.service(5); GLOSSARY.md (scope node)

#### Scenario: Checking a config
- **WHEN** `scope-node --check-config` runs on a config holding the token `test-token`
- **THEN** it prints every setting with the token shown as `****`, never the token, and exits 0

#### Scenario: A bad config
- **WHEN** the config holds the key `exposre_ms` or a gain of `loud`
- **THEN** the node exits with a nonzero status and a message naming that key
