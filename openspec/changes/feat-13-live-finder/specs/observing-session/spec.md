## ADDED Requirements

### Requirement: The scope node in the session

The session SHALL hold the scope node's address and token, set by `PUT /api/session` like its
other fields. The token SHALL be write-only: reads SHALL say only whether one is set. The state
file SHALL keep both, readable and writable by its owner only.
**Refs**: scope-link "A paired link"; observing-session "Checked edits", "Kept across restarts"

#### Scenario: A token is never read back
- **WHEN** a client puts the node `192.168.1.40:50051` with the token `test-token` and reads the
  session
- **THEN** the read gives the address and `token_set: true`, and the text `test-token` appears
  nowhere in it

#### Scenario: Kept, privately
- **WHEN** the binary restarts on the same `--state` folder
- **THEN** the node's address and token are as set, and the state file's mode is 0600

#### Scenario: A bad address
- **WHEN** a client puts the node `not an address`
- **THEN** the answer is 400 with a JSON `error`, and nothing changes

### Requirement: Camera settings in the session

The session SHALL hold the finder camera's exposure (automatic, or milliseconds within the
camera's range), gain (automatic, or a value within its range) and detection threshold, keep
them across restarts, and send them to the scope node when they change and when it connects.
**Refs**: scope-link "Settings from the console"; scope-node "Auto-exposure"

#### Scenario: Set once, sent on connect
- **WHEN** an exposure of 200 ms is put while no node is connected, and the simulator then
  connects
- **THEN** the simulator's frames report 200 ms from the second frame after connecting

#### Scenario: An exposure the camera can't take
- **WHEN** an exposure longer than the connected camera's range is put
- **THEN** the answer is 400 with a JSON `error` naming the range, and nothing changes

### Requirement: The aim point in the session

The session SHALL hold the aim point as a pixel position in the finder camera's frame, kept
across restarts until it is reset; with none set, the frame centre SHALL be used.
**Refs**: GLOSSARY.md (aim point); observing-session "The latest pointing"

#### Scenario: Kept until reset
- **WHEN** the aim point is set to (700, 420), the binary restarts, and the aim point is then
  reset
- **THEN** after the restart it is (700, 420) exactly, and after the reset the session has none
