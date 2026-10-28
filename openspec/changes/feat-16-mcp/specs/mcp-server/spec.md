## Purpose

The finder console's MCP server: a Streamable HTTP endpoint on the house LAN whose tools pass
the console's own actions and the scope link's mount calls through to MCP clients, with every
tool that can move the telescope held to the same guards as the page.

## ADDED Requirements

### Requirement: The MCP endpoint

The console SHALL serve MCP over Streamable HTTP at `/mcp` on its own address and port, needing
no token: a POST of a JSON-RPC request SHALL get one `application/json` answer, a notification
SHALL get 202, a GET SHALL get 405, and an unsupported `MCP-Protocol-Version` SHALL get 400. It
SHALL need no `Mcp-Session-Id`, and `finder --no-mcp` SHALL turn the endpoint off.
**Refs**: docs/research/2026-10-25-mcp-server.md §1–2, §5; MCP specification 2025-06-18, Transports (Streamable HTTP)

#### Scenario: Initialising
- **WHEN** a client with no `Authorization` header posts `initialize` to `/mcp` on
  `finder --simulate`
- **THEN** the answer is 200 `application/json` with the server's name and the `tools`
  capability, and a following `tools/list` without an `Mcp-Session-Id` lists at least the tools
  of every change already landed (oracle: the tool table in design.md, exact names)

#### Scenario: No stream
- **WHEN** a client sends GET to `/mcp`
- **THEN** the answer is 405 and the session file is byte-for-byte unchanged

#### Scenario: Turned off
- **WHEN** the console runs with `--no-mcp` and a client posts `initialize` to `/mcp`
- **THEN** the answer's status equals the one for a path the console doesn't serve (oracle: a
  request to `/no-such-path`, exact), and no tool runs

### Requirement: Origin and Host

The endpoint SHALL refuse, with 403 and nothing run, a request whose `Origin` names an origin
other than the one it was sent to, or whose `Host` or `Origin` host is not an IP literal,
`localhost`, or a name given with `--mcp-allow-host`. It SHALL allow a request with no `Origin`.
From a request that fails these checks it SHALL carry out a `tools/call` of `stop` or
`stop_all` and nothing else.
**Refs**: MCP specification 2025-06-18, Transports (Security Warning); RFC 6454 §7; observing-session "Same-origin writes"; docs/research/2026-10-25-mcp-server.md §3

#### Scenario: A page on another site
- **WHEN** `set_target` for M42 is posted with `Origin: http://elsewhere.example`
- **THEN** the answer is 403, and the session's target is unchanged (oracle: `GET /api/session`
  before and after, exact)

#### Scenario: DNS rebinding
- **WHEN** `goto_target` is posted with `Host: evil.example:8080` and
  `Origin: http://evil.example:8080`, and `evil.example` was not given with `--mcp-allow-host`
- **THEN** the answer is 403 and no axis moves (oracle: the simulator's axis state, exact)

#### Scenario: Clients on the LAN
- **WHEN** `get_session` is posted with no `Origin` and `Host: 192.168.1.20:8080`, and again with
  `Host: surface.local:8080` after `--mcp-allow-host surface.local`
- **THEN** both are answered 200 with the session, equal to `GET /api/session`

#### Scenario: Stop from anywhere
- **WHEN** `stop_all` is posted with `Origin: http://elsewhere.example` while a GoTo runs on the
  simulator
- **THEN** the GoTo is halted with the reason Stop and both axes are commanded to stop within
  0.1 s of virtual time

#### Scenario: A move riding on a stop
- **WHEN** a JSON-RPC batch of `stop_all` and `goto_target` is posted with
  `Origin: http://elsewhere.example` while both axes are at rest
- **THEN** both axes are commanded to stop, no GoTo starts, and the session has no `motion`
  (oracle: the simulator's command log, exact)

### Requirement: LAN peers only

The endpoint SHALL refuse, with 403 and nothing run, a request whose peer address is not
loopback, private (RFC 1918), link-local or an IPv6 unique local address, so that it answers
only the house LAN even if the console's port becomes reachable from the internet. From such a
request it SHALL still carry out a `tools/call` of `stop` or `stop_all`.
**Refs**: RFC 1918; RFC 4193; RFC 3927; RFC 4291 §2.5.6; MCP specification 2025-06-18, Transports (Security Warning); owner's decision of 2026-10-25 (open on the LAN)

#### Scenario: From the internet
- **WHEN** `get_session` and `goto_target` are posted from the peer address `203.0.113.7`
  with no `Origin` and `Host: 192.168.1.20:8080`
- **THEN** both are answered 403 and no axis moves (oracle: the simulator's axis state, exact)

#### Scenario: From the house LAN
- **WHEN** `get_session` is posted from the peer addresses `127.0.0.1`, `192.168.1.30`,
  `172.24.0.1`, `fe80::1` and `fd00::30`
- **THEN** each is answered 200 with the session, equal to `GET /api/session`

### Requirement: Tools pass through to the console

Each tool SHALL call the same function as the HTTP route or action it passes through to, take
the session's JSON names, and return that route's JSON as `structuredContent` and as text. A
refusal SHALL be a result with `isError: true` and the same reason the route's 400, 404, 409 or
413 carries. Read-only tools SHALL carry `readOnlyHint: true`.
**Refs**: MCP specification 2025-06-18, Tools (structured content, tool errors, annotations); docs/research/2026-10-25-mcp-server.md §4

#### Scenario: Searching
- **WHEN** `search_targets` is called with `{"q": "orion"}`
- **THEN** its `structuredContent` equals `GET /api/targets?q=orion`'s answer, exactly

#### Scenario: A refusal reads the same
- **WHEN** `set_session` is called with `min_alt_deg` 85, and again with `{"sun_guard": false}`
- **THEN** each result has `isError: true` and the same `error` text `PUT /api/session` answers
  with 400, and the session is unchanged (oracle: `GET /api/session`, exact)

### Requirement: Session and catalog tools

The endpoint SHALL offer at least `get_session`, `set_session` (`site`, `min_alt_deg`,
`target`), `search_targets`, `get_guidance`, `set_target`, `solve_frame` (an image as base64,
held to `POST /api/solve`'s limits once decoded) and `get_pointing`. `set_session` SHALL refuse
the scope node's address and token, and a new site while a mount is connected, since the Sun
guard reckons the Sun from the site.
**Refs**: observing-session "Checked edits", "The latest pointing"; target-catalog "Search"; push-to "Turn and tilt"; finder-console (request limits); GLOSSARY.md (pointable, push-to)

#### Scenario: Push-to over MCP
- **WHEN** the latest pointing is at azimuth 350°, altitude 30°, the target is pointable at
  azimuth 24°, altitude 42° (push-to's "Across north"), and `get_guidance` is called
- **THEN** it gives turn clockwise 34° and tilt up 12° (within 1e-9°), equal to the `guidance`
  in `GET /api/session` for the same session

#### Scenario: The scope node stays put
- **WHEN** `set_session` is called with a scope node address or token
- **THEN** the result has `isError: true`, and the session's scope node address and
  whether a token is set are unchanged (oracle: `GET /api/session`, exact)

#### Scenario: The site stays put under a mount
- **WHEN** the simulated mount is connected at 40° N, 75° W and `set_session` is called with the
  site 40° N, 75° E
- **THEN** the result has `isError: true` saying the site can't change while a mount is
  connected, and the site is unchanged (oracle: `GET /api/session`, exact)

#### Scenario: Solving a saved frame
- **WHEN** `solve_frame` is called with `orion2.jpg` as base64
- **THEN** the RA, Dec and roll equal `POST /api/solve`'s for the same file, exactly, and
  `get_pointing` then returns the session's `pointing`, field for field

#### Scenario: At the limit
- **WHEN** `solve_frame` is called with an image file of exactly 32 MiB, and again with one
  byte more
- **THEN** the first gets the same answer `POST /api/solve` gives for those bytes, exactly, and
  the second has `isError: true` with `POST /api/solve`'s 413 reason and leaves the latest
  pointing unchanged

### Requirement: Live tools

The endpoint SHALL offer at least `get_link`, giving the scope link's health and the live loop's
state as the session reports them, and `get_pointing` SHALL return the latest live solve while
the live loop runs.
**Refs**: scope-link "Link health"; live-finder "Live solving", "Fresh pointings only"

#### Scenario: Live pointing
- **WHEN** `finder --simulate` runs its live loop and `get_pointing` is called
- **THEN** the pointing is within 1′ of the simulator's true eyepiece direction at the frame's
  mid-exposure time

#### Scenario: Link lost
- **WHEN** the simulator stops answering heartbeats and `get_link` is called 2 s later
- **THEN** it reports the link as disconnected, matching `GET /api/session`, exactly

### Requirement: Mount calls through MCP

The endpoint SHALL offer at least `describe_axes`, `get_axis_state`, `goto_axis_position`,
`move_axis` and `stop_all`, mapping onto the scope link's mount calls, and `goto_target` and
`stop`, mapping onto the console's GoTo and Stop. The scope node's token SHALL stay in the
console and appear in no tool's result.
**Refs**: scope-link "Mount calls"; goto "Pointable targets only", "Halts"; ADR-0001; MCP security best practices (token passthrough)

#### Scenario: Axis state
- **WHEN** `describe_axes` and `get_axis_state` are called on `finder --simulate`
- **THEN** each axis's counts per turn, top rate, position, rate and moving flag equal the
  simulator's, exactly

#### Scenario: The token stays home
- **WHEN** every tool is called once on `finder --simulate` and the results are searched for the
  scope node's token
- **THEN** it occurs in none of them

### Requirement: Motion runs at once

A moving tool from an MCP client SHALL start the motion before it answers, with no confirmation
on the page, and SHALL record the motion in the session as coming from an MCP client.
**Refs**: observing-session "The source of motion"; owner's decision of 2026-10-25 (design.md)

#### Scenario: GoTo from an MCP client
- **WHEN** `goto_target` is called for a pointable target on `finder --simulate` after a sync,
  and nothing is done on the page
- **THEN** the GoTo completes with the target within 1′ of the aim point (oracle: the
  simulator's true eyepiece direction), and the session showed `motion.by` as `mcp` while it ran

### Requirement: Guards on every move

Every tool that can move the telescope SHALL run the page's guards before and during the move,
with no argument that turns one off: the Sun guard on motion, the minimum altitude, the
one-turn cable rule, Stop wins and the node's watchdog. A direct axis move SHALL be refused
when the night has no sync, because the console cannot then tell where it would point.
**Refs**: goto "The Sun guard on motion", "Planning a slew", "Halts", "Pointable targets only"; scope-node "Stop wins, and a watchdog"; ADR-0001; GLOSSARY.md (Sun guard, minimum altitude, direct axis move)

#### Scenario: Daylight
- **WHEN** `goto_target`, `goto_axis_position` and `move_axis` are each called while the Sun's
  centre is 5° above the horizon
- **THEN** each result has `isError: true` saying the Sun is up, and no axis moves (oracle: the
  simulator's axis state, exact)

#### Scenario: Pointed at the Sun by axis position
- **WHEN** at dusk, with the Sun 10° below the horizon, `goto_axis_position` asks for the axis
  positions that the pointing model puts 5° from the Sun
- **THEN** the result has `isError: true` naming the Sun guard, and no axis moves (oracle: the
  simulator's axis state, exact)

#### Scenario: A rate toward the Sun
- **WHEN** at dusk `move_axis` asks to turn the azimuth axis toward the Sun's azimuth at its top
  rate for 30 s, from 40° away
- **THEN** the result has `isError: true` naming the Sun guard, and no axis moves (oracle: the
  simulator's axis state, exact)

#### Scenario: Below the minimum altitude
- **WHEN** `goto_axis_position` asks for positions the model puts 5° below the session's minimum
  altitude
- **THEN** the result has `isError: true` saying the position is too low, and no axis moves

#### Scenario: Climbing from low down
- **WHEN** at night, after a sync, the telescope points 3° up with a minimum altitude of 15°,
  and `move_axis` turns the altitude axis up at 1°/s for 20 s
- **THEN** the move is carried out and the axis is commanded to stop 20.0 ± 0.1 s of virtual
  time after the call (oracle: the simulator's command log)

#### Scenario: Past the cable
- **WHEN** `goto_axis_position` asks for an azimuth 1.2 turns from where the night's first sync
  was made
- **THEN** the result has `isError: true` naming the cable rule, and no axis moves

#### Scenario: No sync yet
- **WHEN** `move_axis` is called before the night's first sync
- **THEN** the result has `isError: true` saying a sync is needed, and no axis moves

### Requirement: Rate moves end

`move_axis` SHALL take a signed rate, capped at the axis's top rate, and a duration of at most
30 s, SHALL refuse a move whose path over that duration would break a guard, and SHALL stop the
axis when the duration ends, on Stop, or in time to be at rest before any limit that a change
while it runs (the Sun rising, a new minimum altitude) brings into its path.
**Refs**: scope-link "Mount calls"; goto "Halts"; scope-node "Stop wins, and a watchdog"; design.md (direct axis moves)

#### Scenario: The duration ends
- **WHEN** at night, after a sync, `move_axis` turns the altitude axis at 0.5°/s for 10 s on the
  simulator, clear of every limit
- **THEN** the axis is commanded to stop 10.0 ± 0.1 s of virtual time after the call, and has
  moved 5.0° less the simulated mount's acceleration ramp (oracle: the simulator's motion model,
  within 0.1°)

#### Scenario: Too long
- **WHEN** `move_axis` is called with a duration of 60 s
- **THEN** the result has `isError: true` and no axis moves

#### Scenario: The client goes away
- **WHEN** at night, after a sync, `move_axis` turns the azimuth axis for 20 s clear of every
  limit, and the MCP client then closes its connection and sends nothing more
- **THEN** the axis is commanded to stop 20.0 ± 0.1 s of virtual time after the call, and the
  console's heartbeats to the node carry on throughout (oracle: the simulator's command and
  heartbeat logs)

#### Scenario: Sunrise during a rate move
- **WHEN** the virtual clock passes the moment the Sun's centre reaches −0.83° during a
  `move_axis`
- **THEN** the axis is commanded to stop within 1 s of that moment, and the session says the
  Sun guard ended the move

### Requirement: A rate limit on moving tools

The endpoint SHALL accept at most 2 calls a second to the tools that can move the telescope,
across all MCP clients, refusing the excess with `isError: true` and moving nothing for them.
Reads, `stop` and `stop_all` SHALL NOT be limited.
**Refs**: MCP specification 2025-06-18, Tools (Security Considerations: rate limit tool invocations); docs/research/2026-10-25-mcp-server.md §4

#### Scenario: A burst of moves
- **WHEN** at night, after a sync, ten `move_axis` calls clear of every limit arrive within 1 s
  of virtual time, followed by `stop_all`
- **THEN** at most 2 are carried out, the rest have `isError: true` saying the rate limit was
  reached, and `stop_all` stops both axes (oracle: the simulator's command log, exact count)

### Requirement: Stop is always there

`stop` and `stop_all` SHALL be listed in every state once the mount tools have landed, SHALL
never be refused or limited, whatever the request's headers or peer, and SHALL end any GoTo,
direct axis move or tracking in progress, whoever started it, before the console sends stop
all. A stop SHALL be carried out even when the answer must be an error.
**Refs**: scope-node "Stop wins, and a watchdog"; goto "Halts"; finder-console "GoTo and Stop on the console"; MCP specification 2025-06-18, Transports (Protocol Version Header)

#### Scenario: Stop during an MCP client's GoTo
- **WHEN** `goto_target` runs from an MCP client and `stop_all` is called from another client
- **THEN** the GoTo is halted with the reason Stop, both axes are commanded to stop within 0.1 s
  of virtual time, and no later correction moves them (oracle: the simulator's axis state for
  5 s after)

#### Scenario: Stop with a bad protocol version
- **WHEN** `stop` is posted with `MCP-Protocol-Version: 1999-01-01` while a GoTo runs on the
  simulator
- **THEN** the answer is 400, and both axes are still commanded to stop within 0.1 s of virtual
  time

#### Scenario: A bare stop
- **WHEN** a `tools/call` of `stop_all` is posted with no `initialize` before it, no `Accept`
  header and no `MCP-Protocol-Version`, while a GoTo runs on the simulator
- **THEN** both axes are commanded to stop within 0.1 s of virtual time and the GoTo is halted
  with the reason Stop, whatever the answer's status (oracle: the simulator's command log)

#### Scenario: Stop with nothing moving
- **WHEN** `stop` is called with no mount connected
- **THEN** the result is a success, not an error

### Requirement: Tracking tools

The endpoint SHALL offer at least `set_tracking`, turning Track on or off as the page's toggle
does, refused for the same reasons, and `get_session` SHALL include tracking's readout.
**Refs**: tracking "Ending safely"; observing-session "Track in the session"; finder-console "Track on the console"

#### Scenario: Tracking from an MCP client
- **WHEN** `set_tracking` turns Track on for M42 on `finder --simulate` and 10 min of virtual
  time pass
- **THEN** M42 stays within 1′ of the aim point (oracle: the simulator's true eyepiece direction,
  sampled every 1 s), and `get_session` reads the tracking error equal to the page's readout

#### Scenario: Track refused
- **WHEN** `set_tracking` is called with no mount connected
- **THEN** the result has `isError: true` with the same reason `POST /api/track` gives
