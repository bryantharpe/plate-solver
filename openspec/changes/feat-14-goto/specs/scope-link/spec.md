## ADDED Requirements

### Requirement: Mount calls

The scope link SHALL carry the mount port's calls: describe the axes (counts per revolution, top
rate, whether each can go to a position), each axis's state (position, rate, whether it is
moving), go to a position, move at a rate, and stop all. Every call except stop all SHALL need
the token, so anyone on the network can halt the telescope but no one else can move it.
**Refs**: scope-node "The mount port"; ADR-0001; GLOSSARY.md (mount port)

#### Scenario: A move without the token
- **WHEN** a client with no token asks the node to move an axis
- **THEN** the call fails with UNAUTHENTICATED and no axis moves

#### Scenario: Stop without the token
- **WHEN** a client with no token calls stop all while both axes move
- **THEN** the call succeeds and both axes stop

#### Scenario: Axis state over the link
- **WHEN** the console describes the simulated mount's axes and reads their state through the
  link
- **THEN** it gets each axis's counts per revolution and top rate, and its position, rate and moving
  flag, exactly as the simulator holds them
