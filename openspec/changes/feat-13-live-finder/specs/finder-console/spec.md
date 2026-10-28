## ADDED Requirements

### Requirement: Setup's scope node and camera section

The Setup page SHALL have a scope node and camera section that sets the node's address and token
(never showing the token) and the camera's exposure, gain and detection threshold, and shows the
link's state, the camera's model, the latest frame's star count and the aim point with its reset.
**Refs**: observing-session (the scope node, camera settings and aim point); scope-link "Link health"; DESIGN.md

#### Scenario: Connected to the simulator
- **WHEN** Setup is open on `finder --simulate`
- **THEN** it shows the link as connected, the simulated camera's model, and the latest frame's
  star count (between 25 and 40), updated within 2 s of each frame

#### Scenario: A token kept out of sight
- **WHEN** a token has been saved and Setup is opened on another device
- **THEN** the token field is empty, says a token is set, and the page never received the token

### Requirement: The finder camera's field against the database

The Setup page SHALL show the finder camera's solved field of view beside the loaded pattern
database's range, and SHALL warn that a database built for the camera's field may be needed when
the field lies within 1° of either end of the range or outside it, or when frames holding at
least 10 stars have not solved for 30 s.
**Refs**: finder-console "Report the loaded database"; database-generation; PRODUCT.md (the finder camera's field)

#### Scenario: Inside the range
- **WHEN** the simulator's 12° field solves against the bundled 10–30° database
- **THEN** Setup shows 12° against 10–30° with no warning

#### Scenario: Near the edge
- **WHEN** the latest solved field is 10.5° against the bundled 10–30° database
- **THEN** Setup shows it with the warning

#### Scenario: Frames that never solve
- **WHEN** the simulator streams frames of 25 or more stars for 30 s of virtual time and every
  solve fails (an injected failing solver, the seam "A slow solver" uses)
- **THEN** Setup warns that the camera's field may be outside the database's 10–30° range and that
  a matching database may be needed
