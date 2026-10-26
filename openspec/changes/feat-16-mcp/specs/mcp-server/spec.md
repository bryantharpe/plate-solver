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
**Refs**: docs/research/2026-10-25-mcp-server.md §1–2; MCP specification 2025-06-18, Transports (Streamable HTTP)

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
- **THEN** the answer is 404, exactly as for any unknown path

### Requirement: Origin and Host

The endpoint SHALL refuse, with 403 and nothing run, a request whose `Origin` names an origin
other than the one it was sent to, or whose `Host` or `Origin` host is not an IP literal,
`localhost`, or a name given with `--mcp-allow-host`. It SHALL allow a request with no `Origin`.
A `tools/call` of `stop` or `stop_all` SHALL be carried out even when these checks fail.
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

### Requirement: Tools pass through to the console

Each tool SHALL call the same function as the HTTP route or action it passes through to, take
the session's JSON names, and return that route's JSON as `structuredContent` and as text. A
refusal SHALL be a result with `isError: true` and the same reason the route's 400, 404 or 409
carries. Read-only tools SHALL carry `readOnlyHint: true`.
**Refs**: MCP specification 2025-06-18, Tools (structured content, tool errors, annotations); docs/research/2026-10-25-mcp-server.md §4

#### Scenario: Searching
- **WHEN** `search_targets` is called with `{"q": "orion"}`
- **THEN** its `structuredContent` equals the HTTP target search's answer for `orion`, exactly

#### Scenario: A refusal reads the same
- **WHEN** `set_session` is called with a minimum altitude of 95°
- **THEN** the result has `isError: true` and the same `error` text `PUT /api/session` answers
  with 400, and the session is unchanged (oracle: `GET /api/session`, exact)

### Requirement: Session and catalog tools

The endpoint SHALL offer at least `get_session`, `set_session`, `search_targets`,
`get_guidance` (the target's pointability and push-to directions), `set_target`, `solve_frame`
(an image as base64, under the same size limit as `POST /api/solve`) and `get_pointing` (the
latest pointing).
**Refs**: observing-session; target-catalog; push-to; finder-console; GLOSSARY.md (pointable, push-to)

#### Scenario: Push-to over MCP
- **WHEN** `set_target` is called with M42 and then `get_guidance`, with the session's latest
  pointing and clock as in push-to's worked example
- **THEN** the turn and tilt equal the page's guidance for the same session, exactly

#### Scenario: Solving a saved frame
- **WHEN** `solve_frame` is called with a saved fixture frame as base64
- **THEN** the RA, Dec and roll equal `POST /api/solve`'s for the same file, exactly, and
  `get_pointing` then returns that solve

#### Scenario: Too large
- **WHEN** `solve_frame` is called with an image larger than `POST /api/solve` accepts
- **THEN** the result has `isError: true` and the latest pointing is unchanged

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
- **THEN** it reports the link as lost, matching `GET /api/session`, exactly

### Requirement: Mount calls through MCP

The endpoint SHALL offer at least `describe_axes`, `get_axis_state`, `goto_axis_position`,
`move_axis` and `stop_all`, mapping onto the scope link's mount calls, and `goto_target` and
`stop`, mapping onto the console's GoTo and Stop. The scope node's token SHALL stay in the
console and appear in no tool's result.
**Refs**: scope-link "Mount calls"; goto; ADR-0001; MCP security best practices (token passthrough)

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

#### Scenario: GoTo from an AI
- **WHEN** `goto_target` is called for a pointable target on `finder --simulate` after a sync,
  and nothing is done on the page
- **THEN** the GoTo completes with the target within 1′ of the aim point (oracle: the
  simulator's true eyepiece direction), and the session showed `motion.by` as `mcp` while it ran

### Requirement: Guards on every move

Every tool that can move the telescope SHALL run the page's guards before and during the move:
the Sun guard on motion, the minimum altitude and horizon, the path planner's mechanical range
and cable rule, Stop wins, and the node's watchdog. A direct axis move SHALL be refused when the
night has no sync, because the console cannot then tell where it would point.
**Refs**: goto "The Sun guard on motion", "Planning a slew", "Halts", "Pointable targets only"; scope-node "Stop wins, and a watchdog"; ADR-0001; GLOSSARY.md (Sun guard, minimum altitude)

#### Scenario: Daylight
- **WHEN** `goto_target`, `goto_axis_position` and `move_axis` are each called while the Sun's
  centre is 5° above the horizon
- **THEN** each result has `isError: true` saying the Sun is up, and no axis moves (oracle: the
  simulator's axis state, exact)

#### Scenario: Pointed at the Sun by axis position
- **WHEN** at dusk, with the Sun 10° below the horizon, `goto_axis_position` asks for the axis
  positions that the pointing model puts 5° from the Sun
- **THEN** the result has `isError: true` naming the Sun guard, and no axis moves

#### Scenario: A rate toward the Sun
- **WHEN** at dusk `move_axis` turns the azimuth axis toward the Sun's azimuth at its top rate
  for 30 s, from 40° away
- **THEN** the simulator's true eyepiece direction, sampled every 0.1 s of virtual time, never
  comes within 15° of the Sun, and the result or the session says the Sun guard stopped it

#### Scenario: Below the minimum altitude
- **WHEN** `goto_axis_position` asks for positions the model puts 5° below the session's minimum
  altitude
- **THEN** the result has `isError: true` saying the position is too low, and no axis moves

#### Scenario: Past the cable
- **WHEN** `goto_axis_position` asks for an azimuth 1.2 turns from where the night's first sync
  was made
- **THEN** the result has `isError: true` naming the cable rule, and no axis moves

#### Scenario: No sync yet
- **WHEN** `move_axis` is called before the night's first sync
- **THEN** the result has `isError: true` saying a sync is needed, and no axis moves

### Requirement: Rate moves end

`move_axis` SHALL take a signed rate, capped at the axis's top rate, and a duration of at most
30 s, and SHALL stop the axis when the duration ends, when a guard's forecast would be crossed
within 1 s, or on Stop.
**Refs**: scope-link "Mount calls"; goto "Halts"; design.md (direct axis moves)

#### Scenario: The duration ends
- **WHEN** `move_axis` turns the altitude axis at 0.5°/s for 10 s on the simulator, clear of
  every limit
- **THEN** the axis is commanded to stop 10.0 ± 0.1 s of virtual time after the call, and has
  moved 5.0 ± 0.1°

#### Scenario: Too long
- **WHEN** `move_axis` is called with a duration of 60 s
- **THEN** the result has `isError: true` and no axis moves

### Requirement: Stop is always there

`stop` and `stop_all` SHALL be listed in every state once the mount tools have landed, SHALL never
be refused, and SHALL end any GoTo, axis move or tracking in progress, whoever started it, before
the console sends stop all.
**Refs**: scope-node "Stop wins, and a watchdog"; goto "Halts"; finder-console "GoTo and Stop on the console"

#### Scenario: Stop during an AI's GoTo
- **WHEN** `goto_target` runs from an MCP client and `stop_all` is called from another client
- **THEN** the GoTo is halted with the reason Stop, both axes are commanded to stop within 0.1 s
  of virtual time, and no later correction moves them (oracle: the simulator's axis state for
  5 s after)

#### Scenario: Stop with nothing moving
- **WHEN** `stop` is called with no mount connected
- **THEN** the result is a success, not an error

### Requirement: Tracking tools

The endpoint SHALL offer at least `set_tracking`, turning Track on or off as the page's toggle
does, refused for the same reasons, and `get_session` SHALL include tracking's readout.
**Refs**: tracking; finder-console "Track on the console"

#### Scenario: Tracking from an AI
- **WHEN** `set_tracking` turns Track on for M42 on `finder --simulate` and 10 min of virtual
  time pass
- **THEN** M42 stays within 1′ of the aim point (oracle: the simulator's true eyepiece direction,
  sampled every 1 s), and `get_session` reads the tracking error equal to the page's readout

#### Scenario: Track refused
- **WHEN** `set_tracking` is called with no mount connected
- **THEN** the result has `isError: true` with the same reason `POST /api/track` gives
