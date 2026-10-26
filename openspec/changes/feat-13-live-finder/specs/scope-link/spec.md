## Purpose

The protocol between the console and the scope node: how they pair, how star frames and previews
travel from the node, how the node's clock is mapped to the console's, and how the console's
camera and detection settings reach the node.

## ADDED Requirements

### Requirement: A paired link

The scope node SHALL serve the gRPC service `scope_node.v1`, and the console SHALL connect to it
at the address the session holds. Every call except one that can only stop the telescope SHALL
carry the session's token, and the node SHALL answer such a call with no token or another token
with UNAUTHENTICATED, serving nothing.
**Refs**: ADR-0001; docs/research/2026-10-21-scope-node-runtime.md §5 (WSL connects out)

#### Scenario: No token or the wrong one
- **WHEN** a client calls the node's frame stream with no token, then with `wrong-token`
- **THEN** both calls fail with UNAUTHENTICATED and no frame is sent

#### Scenario: The paired console
- **WHEN** the console calls with the node's configured token
- **THEN** the stream opens and frames arrive

### Requirement: Star frames

The node SHALL stream one star frame per exposure, each with: an id that grows by one per
exposure; exposure start and end on the node's clock; frame size, exposure and gain; the
detection settings used; and up to 200 centroids, brightest first, in full-frame pixels.
**Refs**: ADR-0001 (stars, not frames, cross the Wi-Fi); star-detection "Brightness measurement and ordering"

#### Scenario: What a frame carries
- **WHEN** the simulator's node sends ten frames of a field holding more than 200 detectable stars
- **THEN** each frame has every field above, ids rise by one, each exposure's end minus start
  equals its exposure time within 1 µs, and each holds exactly 200 centroids in descending
  brightness

#### Scenario: A dropped frame shows
- **WHEN** the console misses one frame of the stream
- **THEN** the next frame's id is two more than the last one it received

### Requirement: Previews

The node SHALL send with a star frame a preview of it, a JPEG at most 640 px on its longer side,
no more often than once every 5 s unless the console asks, and SHALL send one with the next
frame after the console asks.
**Refs**: docs/research/2026-10-21-scope-node-runtime.md §3 (send previews only now and then)

#### Scenario: Previews are rationed
- **WHEN** the simulator's node streams at 2 frames a second for 20 s of virtual time
- **THEN** at most 5 frames carry a preview, every preview is at most 640 px on its longer side,
  and any two are at least 5 s apart

#### Scenario: One on request
- **WHEN** the console asks for a preview 1 s after the last one
- **THEN** the next frame carries one

### Requirement: Clock mapping

The console and node SHALL exchange heartbeats at least every 250 ms, each recording t0 (console
sends), t1 (node receives), t2 (node replies) and t3 (console receives). The console SHALL map
node time to its own with the offset ((t1 − t0) + (t2 − t3)) / 2 of the exchange with the least
delay (t3 − t0) − (t2 − t1) among those of the last 8 s, so that node time T is T − offset on
the console's clock.
**Refs**: RFC 5905 §8 (NTP's offset and delay); docs/research/2026-10-21-pointing-model-and-tracking.md §4 (1 s is 15″)

#### Scenario: A worked exchange
- **WHEN** exchange A has t0 100.000, t1 100.070, t2 100.071 and t3 100.041 s, and exchange B
  has t0 101.000, t1 101.150, t2 101.151 and t3 101.121 s
- **THEN** A's delay is 0.040 s and B's 0.120 s, and the offset used is A's, +0.050 s (within
  1 µs)

#### Scenario: An old best exchange
- **WHEN** the least-delay exchange (delay 0.010 s, offset +0.020 s) is 9 s old and every
  exchange of the last 8 s has delay 0.040 s and offset +0.050 s
- **THEN** the offset used is +0.050 s (within 1 µs), and node time 200.100 s maps to 200.050 s

#### Scenario: Often enough
- **WHEN** a link runs for 10 s of virtual time
- **THEN** the console has recorded at least 40 heartbeats

### Requirement: Link health

The console SHALL count the node as connected while heartbeats arrive, and as disconnected once
1 s has passed without one, and SHALL then reconnect, waiting at most 5 s between tries. It SHALL
report the link's state, the latest frame's age and the clock offset.
**Refs**: GLOSSARY.md (scope node); ADR-0001

#### Scenario: The node goes away and comes back
- **WHEN** the simulator's node stops answering for 3 s of virtual time and then answers again
- **THEN** the console reports it disconnected 1 s (±0.1 s) after the last heartbeat, and
  connected again within 5.5 s of its return

### Requirement: Settings from the console

The node SHALL detect stars with cedar's detection settings unless the console sends others, and
SHALL apply the camera and detection settings the console sends from the next exposure on. Each
star frame SHALL report the settings it was made with.
**Refs**: plate-solver "Cedar's detection settings"; observing-session (camera settings)

#### Scenario: Cedar's by default
- **WHEN** a node starts and the console has sent no settings
- **THEN** every frame reports cedar's detection settings

#### Scenario: A lower threshold
- **WHEN** the console sends a detection threshold of 5σ
- **THEN** every frame from the second one after it reports 5σ
