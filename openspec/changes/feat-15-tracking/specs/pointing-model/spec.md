## ADDED Requirements

### Requirement: Syncs against the approach direction

A sync taken while an axis last moved opposite to a GoTo's last approach SHALL have that axis's
measured backlash applied to its axis position before it is fitted, and until that axis's
backlash is measured such a sync SHALL be held back from the fit.
**Refs**: pointing-model "Every solve is a sync", "Slips"; tracking "Backlash at reversals"

#### Scenario: Syncs after a transit
- **WHEN** M42 is tracked through its transit on the simulator (0.3° of altitude backlash,
  measured at that reversal) and syncs are then taken while the altitude axis tracks down
- **THEN** no new zero-point epoch is opened, and the fitted IE equals the simulator's true
  altitude zero point within 30″
