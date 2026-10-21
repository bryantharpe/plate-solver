## ADDED Requirements

### Requirement: The model's state for the night

The session SHALL keep the night's syncs, zero points and base tilt across restarts until the
Sun's next upper transit at the site (local noon) after the night's first sync, and then clear
them. It SHALL keep the fixed terms across nights until they are reset.
**Refs**: pointing-model; GLOSSARY.md (observing session, pointing model); sky-ephemeris "Rising, transit and setting"

#### Scenario: A restart at midnight
- **WHEN** the site is 40° N, 75° W and the binary restarts at 05:30 UT on 2026-01-16, past
  local midnight, with syncs made at 02:00 UT that night
- **THEN** the syncs, zero points and tilt are as before the restart

#### Scenario: The next day
- **WHEN** the console reads the session at that site after the Sun's next upper transit (about
  17:10 UT)
- **THEN** the night's syncs, zero points and tilt are gone, and the fixed terms remain

#### Scenario: Resetting the fixed terms
- **WHEN** the fixed terms are reset
- **THEN** the model holds none, and fits them again once it has enough syncs
