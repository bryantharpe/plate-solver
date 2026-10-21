## Purpose

One night's shared state at the telescope, held by the console and the same for every phone and
laptop that opens it: the site, the minimum altitude, the target and the latest pointing, read
and changed through the console's API and timed by the console's own clock.

## ADDED Requirements

### Requirement: One session for every device

The system SHALL hold one observing session and SHALL answer `GET /api/session` with its site,
minimum altitude, target, latest pointing and the console's time, the same for every client. A
fresh session SHALL have no site, no target and no pointing, and a minimum altitude of 15°.
**Refs**: GLOSSARY.md (observing session, site, target, minimum altitude); ADR-0001

#### Scenario: What one device writes, another reads
- **WHEN** client A puts a site of 40° N, 75° W and the target `M42`, and client B then gets
  the session
- **THEN** B reads latitude 40 and longitude −75 exactly, and the target NGC 1976 with its
  Messier number M42

#### Scenario: A fresh server
- **WHEN** a server starts with no state file and its session is read
- **THEN** it has no site, no target and no pointing, and a minimum altitude of exactly 15°

### Requirement: Checked edits

The system SHALL accept `PUT /api/session` with any of `site`, `min_alt_deg` and `target`,
keeping fields not sent and clearing `site` or `target` sent as null. It SHALL refuse a latitude
outside ±90°, a longitude outside ±180°, a minimum altitude outside 0–80° or an unknown field
with 400, and a target the catalog does not hold with 404, each with a JSON `error`, changing
nothing.
**Refs**: GLOSSARY.md (site: east positive; minimum altitude; Sun guard: no override)

#### Scenario: Values that are out of range
- **WHEN** a client puts latitude 95, or a minimum altitude of 85°, or `{"sun_guard": false}`
- **THEN** each answer is 400 with a JSON `error` naming the field, and a following read of the
  session is identical to the one before

#### Scenario: A target the catalog does not hold
- **WHEN** a client puts the target `NGC 99999`
- **THEN** the answer is 404 with a JSON `error`, and the session's target is unchanged

#### Scenario: One field at a time
- **WHEN** a session holds a site and a target, and a client puts only `{"min_alt_deg": 30}`
- **THEN** the minimum altitude is 30° and the site and target are exactly as before

### Requirement: Kept across restarts

The `finder` binary SHALL keep the session's site, minimum altitude and target in a state file,
by default under `$XDG_STATE_HOME/finder/` (`~/.local/state/finder/` when that is unset) and
under the folder `--state` names otherwise,
written atomically so that a crash never leaves a partial file, and SHALL load it on start. The
library SHALL keep the session in memory unless it is given a state file.
**Refs**: XDG Base Directory Specification (`XDG_STATE_HOME`)

#### Scenario: A restart
- **WHEN** a site, a minimum altitude of 20° and the target M31 are set, and the binary is
  stopped and started again with the same `--state` folder
- **THEN** the session holds the same site, minimum altitude and target exactly, and no pointing

#### Scenario: An unreadable state file
- **WHEN** the binary starts and its state file holds `not json`
- **THEN** it serves a fresh session, says on stderr that the file could not be read, and keeps
  that file beside the new one with a `.bad` suffix

#### Scenario: Tests write nothing
- **WHEN** the library's state is built without a state file and the session is changed through
  the API
- **THEN** no file is created under the state folder or anywhere else

### Requirement: Same-origin writes

Every route that changes the session, except one that only stops the telescope, SHALL refuse,
with 403 and a JSON `error`, a request whose `Origin` header names an origin other than the one
the request was sent to, and SHALL allow a request with no `Origin`. Edits through
`PUT /api/session` SHALL also need a JSON body, refusing other media types with 415, so that a
page on another site cannot drive the console.
**Refs**: RFC 6454 §7 (the Origin header); OWASP CSRF Prevention Cheat Sheet (verifying origin)

#### Scenario: A page on another site
- **WHEN** a request to put the target M42 carries `Origin: http://elsewhere.example`, and a
  frame is posted to `/api/solve` with the same header
- **THEN** both are answered 403 with a JSON `error`, and the session's target and pointing are
  unchanged

#### Scenario: The console's own page
- **WHEN** the same edit carries an `Origin` equal to the scheme, host and port it was sent to,
  or no `Origin` at all
- **THEN** it is applied and answered 200 with the session

#### Scenario: A form post
- **WHEN** an edit is sent as `text/plain` whose body would parse as JSON
- **THEN** the answer is 415 with a JSON `error`, and nothing changes

### Requirement: The latest pointing

The system SHALL make each frame that solves the session's latest pointing: the J2000 RA and Dec
at the aim point (the frame centre until an aim point is set), its altitude and azimuth at the
solve time when the session has a site, and the solve time on the console's clock. A frame that
does not solve SHALL leave the pointing as it was.
**Refs**: GLOSSARY.md (aim point, sync); finder-console "The solve report" (orion2's pointing)

#### Scenario: orion2 under a fixed clock
- **WHEN** `orion2.jpg` is solved with the console's clock fixed at 2026-01-16 02:00 UT and the
  site at 40° N, 75° W
- **THEN** the pointing is RA 85.190964° and Dec −4.246105° (within 1e-4°, orion2's solved
  centre), its altitude and azimuth equal the finder console's own altitude and azimuth of that
  position for that site and time within 1e-9°, and its time is the fixed clock's exactly

#### Scenario: A frame that does not solve
- **WHEN** orion2 has been solved and `roof.jpg` is then solved at the defaults
- **THEN** the session's pointing is still orion2's, field for field

#### Scenario: No site yet
- **WHEN** orion2 is solved while the session has no site
- **THEN** the pointing has RA, Dec and time, and no altitude or azimuth

### Requirement: The console's time

The session SHALL report the console's clock as its time, and everything the system derives from
the session SHALL use that clock. A clock other than the system's SHALL be settable only by tests
and by a simulation mode that never reaches real hardware.
**Refs**: GLOSSARY.md (Sun guard: always on); docs/research/2026-10-21-scope-node-runtime.md §7

#### Scenario: A fixed clock in tests
- **WHEN** the library's clock is fixed at 2026-01-16 02:00 UT and the session is read twice
- **THEN** both reads report the time 1768528800 exactly

#### Scenario: The binary's own clock
- **WHEN** the binary serves outside any simulation mode and its session is read
- **THEN** the reported time is within 2 s of the system clock at the moment of the read
