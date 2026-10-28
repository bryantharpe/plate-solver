---
state: draft
depends_on_landed: ["012-targets-and-push-to", "013-live-finder", "014-goto", "015-tracking"]
# Converted 2026-10-28 from the OpenSpec change feat-16-mcp (reviewed and refined in
# #102). This trio is now the feature's only specification. It stays draft until the
# owner has reviewed it and given the approvals listed under "Owner approvals".
# Flipping it to `ready` is the act that dispatches it.
---

# Feature Specification: The Console's MCP Server

**Feature Branch**: `016-mcp`

**Created**: 2026-10-28

**Status**: Draft

**Input**: The owner wants an AI to steer the telescope: find a target, solve a frame,
point the scope and track it. The AI might be Claude on the Surface, or an app on the
phone. MCP is how those clients call tools.

This feature gives the console an MCP endpoint at `/mcp`, on its own address and port.
Its tools pass two kinds of call through to MCP clients on the house LAN:
- the console's own actions;
- the scope link's mount calls.

Every move still goes through the console's guards, so no MCP client can ever point the
16-inch aperture at the Sun. There is no MCP on the scope node. The Pi is controlled only
over the `scope_node.v1` link, and the console stays the only holder of its token.

## Clarifications

### Session 2026-10-28

- Q: What does a story read besides this trio? → A: Nothing about the feature. The trio is
  self-contained: `spec.md` holds every requirement and scenario, and `plan.md` holds
  every design decision, the interfaces, the shapes that existing tests pin and the
  alternatives rejected. A story that would change a decision escalates rather than
  re-deciding it.
- Q: Can the tools land in steps, after each earlier feature lands? → A: No. This spec
  waits for 012, 013, 014 and 015 to land, because a spec's `depends_on_landed` gates the
  whole epic. So every tool's underlying action already exists when US1 starts, and the
  stories are sliced by concern rather than by the feature that introduced each action:
  1. the endpoint and its door;
  2. the session and catalog tools;
  3. frames and the link;
  4. motion and Stop;
  5. direct axis moves;
  6. tracking;
  7. the page.
- Q: How does the page learn which MCP URLs to show on Setup? → A: From a read-only
  `GET /api/mcp`. It reports whether MCP is on, whether only this machine can reach it,
  the URLs the listen address serves, and the allowed host names. The URLs depend on the
  console's listen address and the host's interfaces, which only the server knows. This
  route is the one interface this trio adds beyond the tool table, and it is named for
  the owner's review (FR-019).
- Q: How does Impeccable take part? → A: `DESIGN.md`, `.impeccable/design.json` and the
  surface briefs under `.impeccable/surfaces/` are the design system that US7 builds to.
  The Impeccable review itself is outside the factory, because a "ship verdict" is not
  evidence the judge can see in a diff. US7 records the new views in `DESIGN.md` as built,
  and the maintainer runs the review after US7 lands.
- Q: What is left open for the owner and not built here? → A: Three questions:
  - whether the page's own routes should also refuse peers outside the house LAN, as
    `/mcp` does;
  - whether the page should also refuse a new site while a mount is connected;
  - whether a token should be offered later as an option, for a network the owner doesn't
    control (a star party).

  The page's routes already have the `Host` check, since 012 ("Known hosts only").
- Q: Who approves the decisions below? → A: The owner, by reviewing this spec and flipping
  it to `ready`. The decisions were taken or recommended on 2026-10-25.

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that makes
it. No existing test, golden or CI file changes beyond approval 15.

1. **Open on the LAN, with no token.** MCP clients need no token. `Origin` is still
   validated (FR-001, FR-002).
2. **Safety is not passed through.** Every moving tool goes through the console's guards:
   - the Sun guard;
   - the minimum altitude and the horizon;
   - the motion and path checks, including the cable rule;
   - Stop wins, and the watchdog.

   This is the one place where "passthrough" isn't literal (FR-009).
3. **The `Host` check.** The `Host`, and the `Origin` host when present, must be an IP
   literal, `localhost`, or a name given with `--mcp-allow-host`. The `Origin` rule alone
   lets a DNS-rebinding page through (FR-002).
4. **Stop and stop all skip the door's checks.** They skip the `Origin`, `Host` and
   peer-address checks, and are carried out even when the answer must be an error. From a
   request that fails a check, only the stops run (FR-002, FR-003, FR-012).
5. **Direct axis moves need a sync.** Without the pointing model the Sun guard can't be
   checked (FR-009).
6. **`move_axis` takes a duration of at most 30 s**, and stops at its end (FR-010).
7. **MCP is on by default**, and `finder --no-mcp` turns it off (FR-001).
8. **A rate limit of 2 moving calls a second**, across all clients. Reads, `stop` and
   `stop_all` are unlimited (FR-011).
9. **No confirmation on the page** for an MCP motion. The page shows "Moved by an MCP
   client" (FR-008, FR-015).
10. **LAN peers only.** `/mcp` refuses peers outside loopback, RFC 1918, link-local and
    IPv6 unique local ranges (FR-003).
11. **A telescope parked low may climb.** A direct axis move that starts below the minimum
    altitude may move only upward or level (FR-009).
12. **No new site over MCP while a mount is connected**, because a wrong site would blind
    the Sun guard (FR-020).
13. **A new dependency: `rmcp` = 3.5.1**, with no default features and only `server`,
    `macros` and `transport-streamable-http-server`. This is a change to the dependency
    graph on a critical path (FR-016; plan.md § Dependencies).
14. **A new read-only route, `GET /api/mcp`**, for Setup's MCP section (FR-019).
15. **One possible line in `playwright.config.ts`.** If the `simulate` project's file
    match doesn't already pick up the new `e2e/mcp.spec.ts`, US7 adds that one file to the
    match and changes nothing else in the config (FR-015).

## User Scenarios & Testing *(mandatory)*

All Rust scenarios share one setup. They post JSON-RPC to `/mcp` through the console's
router with `tower::ServiceExt::oneshot`, as `tests/api.rs` posts to `/api/*`. Peer
addresses are set with axum's `MockConnectInfo`. Every scenario that names the simulator
runs it as 013 does: over an in-memory transport, on a `VirtualClock`, under
`#[tokio::test(start_paused = true)]`. Its true eyepiece direction, axis state, command
log and heartbeat log are the oracles. No test moves real hardware or calls a real LLM.

### User Story 1 - The MCP endpoint, and the door that guards it (Priority: P1)

As an MCP client on the Surface or the phone, I reach `http://<console>/mcp` with no token.
I get JSON answers with no session ID, and I can read the session and set the target.

As the console, I check three things before anything runs:
- `Origin`;
- `Host` against DNS rebinding;
- the peer address, to keep "open on the LAN" on the LAN.

`finder --no-mcp` turns the endpoint off. Setup can ask `GET /api/mcp` which URLs to show.

**Why this priority**: This story is the door every later tool walks through. Getting
its checks wrong opens a 16-inch telescope to any web page or to the internet.

**Independent Test**: `cargo test -p finder --locked --test mcp_endpoint` runs
`crates/finder/tests/mcp_endpoint.rs` through the router. `--no-mcp` and
`--mcp-allow-host` are exercised through the `AppState` builder that `main.rs` uses for
them.

**Acceptance Scenarios**:

1. **Given** a console built with the defaults, **When** a committed test posts `initialize`
   to `/mcp` with no `Authorization` header, **Then** it asserts three things:
   - a 200 `application/json` answer with the server's name and the `tools` capability;
   - a following `tools/list`, sent without an `Mcp-Session-Id`, lists `get_session` and
     `set_target` by those exact names;
   - `get_session` carries `readOnlyHint: true`, and `set_target` does not.
2. **Given** a console with a state file, **When** a committed test sends GET to `/mcp`,
   **Then** it asserts a 405 answer and a state file that is byte-for-byte unchanged.
3. **Given** the console built with MCP turned off (`--no-mcp`), **When** a committed test
   posts `initialize` to `/mcp`, **Then** it asserts that the answer's status equals the
   status of a request to `/no-such-path`, and that no tool ran.
4. **Given** the endpoint, **When** a committed test posts `get_session` with
   `MCP-Protocol-Version: 1999-01-01`, **Then** it asserts a 400 answer and that nothing
   ran.
5. **Given** the endpoint, **When** a committed test posts a JSON-RPC notification,
   **Then** it asserts a 202 answer.
6. **Given** a session with no target, **When** a committed test posts `set_target` for
   M42 with `Origin: http://elsewhere.example`, **Then** it asserts a 403 answer and that
   the session's target is unchanged. The oracle is `GET /api/session` before and after,
   exactly.
7. **Given** `evil.example` was not given with `--mcp-allow-host`, **When** a committed
   test posts `set_target` for M42 with `Host: evil.example:8080` and
   `Origin: http://evil.example:8080`, **Then** it asserts a 403 answer and that the
   target is unchanged.
8. **Given** two requests, **When** a committed test posts `get_session` with no `Origin`
   and `Host: 192.168.1.20:8080`, and again with `Host: surface.local:8080` on a console
   built with `--mcp-allow-host surface.local`, **Then** it asserts that both are answered
   200 with the session, equal to `GET /api/session`.
9. **Given** the peer addresses `127.0.0.1`, `192.168.1.30`, `172.24.0.1`, `fe80::1` and
   `fd00::30`, **When** a committed test posts `get_session` from each, **Then** it
   asserts that each is answered 200 with the session, equal to `GET /api/session`.
10. **Given** the peer address `203.0.113.7`, **When** a committed test posts
    `get_session` and `set_target` for M42 from it, with no `Origin` and
    `Host: 192.168.1.20:8080`, **Then** it asserts that both are answered 403, and that
    the session is unchanged.
11. **Given** the same input to the tool and to its route, **When** a committed test calls
    `get_session` and `set_target` and their HTTP routes, **Then** it asserts that each
    tool's `structuredContent` equals its route's JSON exactly, and that its text content
    is that same JSON.
12. **Given** three consoles, **When** a committed test reads `GET /api/mcp` on each,
    **Then** it asserts:
    - with `--listen 0.0.0.0:8080` and `--mcp-allow-host surface.local`, the route lists
      `http://localhost:8080/mcp`, one URL for each non-loopback IPv4, link-local and
      unique local address in the interface list the test injects, none for its global
      IPv6 address, and `surface.local`;
    - with the default `--listen 127.0.0.1:8080`, it lists only
      `http://localhost:8080/mcp` and reports that only this machine can reach it;
    - with `--no-mcp`, it reports that MCP is off.
13. **Given** `crates/finder/Cargo.toml` and `Cargo.lock`, **When** they are read,
    **Then** `rmcp` is pinned to exactly `=3.5.1`, with `default-features = false` and
    only the features `server`, `macros` and `transport-streamable-http-server`.
14. **Given** `crates/finder/src/main.rs`, **When** it is read, **Then** three things
    hold:
    - the binary serves the router with connect info, so the peer check sees the real
      peer address;
    - `--no-mcp` and `--mcp-allow-host` (repeatable) are options;
    - no option widens the listen address.

---

### User Story 2 - The session and catalog tools (Priority: P1)

As an MCP client, I search the catalog, read the target's guidance, and change the site
and minimum altitude. Each answer is the same JSON the page gets, and each refusal carries
the same reason.

I can't touch the scope node's address or token. I can't move the site while a mount is
connected, because the Sun guard reckons the Sun from the site.

**Why this priority**: These tools let an AI find a target and learn whether it is
pointable before anything moves. The site lock keeps a careless client from blinding the
Sun guard.

**Independent Test**: `cargo test -p finder --locked --test mcp_session_tools` runs
`crates/finder/tests/mcp_session_tools.rs` through the router. The mount scenario uses
the simulator.

**Acceptance Scenarios**:

1. **Given** the catalog, **When** a committed test calls `search_targets` with
   `{"q": "orion"}`, **Then** it asserts that the tool's `structuredContent` equals
   `GET /api/targets?q=orion`'s answer, exactly.
2. **Given** a session, **When** a committed test calls `set_session` with `min_alt_deg`
   85, and again with `{"sun_guard": false}`, **Then** it asserts three things for each:
   - the result has `isError: true`;
   - its text is the same `error` text that `PUT /api/session` answers with 400;
   - the session is unchanged (oracle: `GET /api/session`, exactly).
3. **Given** a session whose latest pointing is at azimuth 350°, altitude 30°, with a
   target pointable at azimuth 24°, altitude 42°, **When** a committed test calls
   `get_guidance`, **Then** it asserts a turn of clockwise 34° and a tilt of up 12°,
   within 1e-9°, equal to the `guidance` in `GET /api/session` for the same session.
4. **Given** a session, **When** a committed test calls `set_session` with a scope node
   address, and again with a scope node token, **Then** it asserts that each result has
   `isError: true`. The session's scope node address, and whether a token is set, are
   unchanged (oracle: `GET /api/session`, exactly).
5. **Given** the simulated mount connected at 40° N, 75° W, **When** a committed test
   calls `set_session` with the site 40° N, 75° E, **Then** it asserts two things:
   - the result has `isError: true`, saying the site can't change while a mount is
     connected;
   - the site is unchanged (oracle: `GET /api/session`, exactly).
6. **Given** the same input to each tool and to its route, **When** a committed test calls
   `set_session` (site, minimum altitude, target), `search_targets` and `get_guidance`
   beside their routes, **Then** it asserts three things:
   - each tool's `structuredContent` equals its route's JSON exactly;
   - `search_targets` and `get_guidance` carry `readOnlyHint: true`;
   - `tools/list` names all three.

---

### User Story 3 - Frames and the link (Priority: P1)

As an MCP client, I send a saved frame as base64 and get the same solve the page gets,
held to the same 32 MiB limit. I read the latest pointing, live while the scope node
streams. I read the link's health.

**Why this priority**: Solving and pointing are how an AI learns where the telescope is
before it asks the telescope to move.

**Independent Test**: `cargo test -p finder --locked --test mcp_frames` runs
`crates/finder/tests/mcp_frames.rs`. The saved-frame scenarios use the bundled
`orion2.jpg`. The 32 MiB images are synthesised by the test at run time and never
committed. The live scenarios use the simulator.

**Acceptance Scenarios**:

1. **Given** `orion2.jpg`, **When** a committed test calls `solve_frame` with it as
   base64, **Then** it asserts two things:
   - the RA, Dec and roll equal `POST /api/solve`'s for the same file, exactly;
   - a following `get_pointing` returns the session's `pointing`, field for field.
2. **Given** an image file of exactly 32 MiB, and one of one byte more, both synthesised by
   the test, **When** a committed test calls `solve_frame` with each as base64, **Then** it
   asserts:
   - the first gets the same answer `POST /api/solve` gives for those bytes, exactly;
   - the second has `isError: true` with `POST /api/solve`'s 413 reason, and leaves the
     latest pointing unchanged.
3. **Given** the simulator's live loop running, **When** a committed test calls
   `get_pointing`, **Then** it asserts a pointing within 1′ of the simulator's true
   eyepiece direction at the frame's mid-exposure time.
4. **Given** the simulator stops answering heartbeats, **When** a committed test calls
   `get_link` 2 s of virtual time later, **Then** it asserts that the link is reported as
   disconnected, matching `GET /api/session` exactly.
5. **Given** `tools/list`, **When** a committed test reads it, **Then** it asserts that it
   names `solve_frame`, `get_pointing` and `get_link`, and that `get_pointing` and
   `get_link` carry `readOnlyHint: true`.

---

### User Story 4 - GoTo and Stop over MCP, and who moved the telescope (Priority: P1)

As an MCP client, I read the axes, start a GoTo that runs at once with no tap to confirm,
and stop the telescope. The session records that the motion came from an MCP client, and
every moving or stopping call is logged.

As the console, I carry out a stop whatever the request's headers, peer, protocol version
or batch around it. I carry it out myself, before the request reaches the MCP SDK, so
nothing in the SDK can stand in front of a stop. The scope node's token never leaves me.

**Why this priority**: This story is where an MCP client first moves real metal. Stop
has to win here before any other moving tool exists.

**Independent Test**: `cargo test -p finder --locked --test mcp_motion` runs
`crates/finder/tests/mcp_motion.rs` against the simulator in virtual time, after a sync.

**Acceptance Scenarios**:

1. **Given** the simulator, **When** a committed test calls `describe_axes` and
   `get_axis_state`, **Then** it asserts that each axis's counts per turn, top rate,
   position, rate and moving flag equal the simulator's, exactly.
2. **Given** a sync and a pointable target, **When** a committed test calls `goto_target`
   and does nothing on the page, **Then** it asserts two things:
   - the GoTo completes with the target within 1′ of the aim point (oracle: the
     simulator's true eyepiece direction);
   - the session showed `motion.by` as `mcp` while it ran.
3. **Given** `goto_target` called over MCP, **When** a committed test polls
   `GET /api/session` every 0.5 s of virtual time, **Then** it asserts that `motion.by` is
   `mcp` and `motion.tool` is `goto_target` until the GoTo ends, and that there is no
   `motion` after.
4. **Given** the same target, **When** a committed test starts GoTo through the page's
   `POST /api/goto` and polls the session every 0.5 s of virtual time, **Then** it asserts
   that `motion.by` is `page` while the GoTo runs.
5. **Given** a session file written before this feature, **When** a committed test loads
   it, **Then** it asserts that it loads without error and reports no `motion`, with every
   other field equal to the saved file's, exactly.
6. **Given** `goto_target` running from one MCP client, **When** a committed test calls
   `stop_all` from another, **Then** it asserts three things:
   - the GoTo is halted with the reason Stop;
   - both axes are commanded to stop within 0.1 s of virtual time;
   - no later correction moves them (oracle: the simulator's axis state for 5 s after).
7. **Given** a GoTo running, **When** a committed test posts `stop` with
   `MCP-Protocol-Version: 1999-01-01`, **Then** it asserts that the answer is 400, and
   that both axes are still commanded to stop within 0.1 s of virtual time.
8. **Given** a GoTo running, **When** a committed test posts a `tools/call` of `stop_all`
   with no `initialize` before it, no `Accept` header and no `MCP-Protocol-Version`,
   **Then** it asserts that both axes are commanded to stop within 0.1 s of virtual time
   and the GoTo is halted with the reason Stop, whatever the answer's status (oracle: the
   simulator's command log).
9. **Given** no mount connected, **When** a committed test calls `stop`, **Then** it
   asserts that the result is a success, not an error, and that `tools/list` still names
   `stop` and `stop_all`.
10. **Given** a GoTo running, **When** a committed test posts `stop_all` with
    `Origin: http://elsewhere.example`, **Then** it asserts that the GoTo is halted with
    the reason Stop, and both axes are commanded to stop within 0.1 s of virtual time.
11. **Given** both axes at rest, **When** a committed test posts a JSON-RPC batch of
    `stop_all` and `goto_target` with `Origin: http://elsewhere.example`, **Then** it
    asserts that both axes are commanded to stop, that no GoTo starts, and that the
    session has no `motion` (oracle: the simulator's command log, exactly).
12. **Given** `evil.example` was not given with `--mcp-allow-host`, **When** a committed
    test posts `goto_target` with `Host: evil.example:8080` and
    `Origin: http://evil.example:8080`, **Then** it asserts a 403 answer and that no axis
    moves (oracle: the simulator's axis state, exactly).
13. **Given** the peer address `203.0.113.7`, **When** a committed test posts
    `get_session` and `goto_target` from it, with no `Origin` and
    `Host: 192.168.1.20:8080`, **Then** it asserts that both are answered 403 and that no
    axis moves (oracle: the simulator's axis state, exactly).
14. **Given** every tool landed so far, **When** a committed test calls each one once with
    a low-entropy test token configured for the scope node, and searches the results,
    **Then** it asserts that the token occurs in none of them.
15. **Given** the console's call log, which goes to stderr by default and which the test
    captures in a buffer, **When** a committed test calls `goto_target` and then
    `stop_all`, **Then** it asserts one log line for each, naming the tool, its arguments
    and its result.

---

### User Story 5 - Direct axis moves, held to the page's guards (Priority: P1)

As an MCP client, I move an axis to a position, or at a rate for at most 30 s, after the
night's first sync. As the console, I hold each move to the page's guards:
- the Sun guard;
- the minimum altitude, judged against the lower of the start and the minimum, so a
  telescope parked low may climb;
- the cable rule.

I forecast a rate move's path, including its stopping distance, and refuse one that
crosses a limit. While it runs I re-forecast and stop it in time, and I stop it at the end
of its duration. Moving tools share a limit of 2 calls a second.

**Why this priority**: The page has no direct axis moves, so these tools are the only path
by which an MCP client could aim the telescope without a target. They are where the Sun
guard must be rebuilt from its pieces.

**Independent Test**: `cargo test -p finder --locked --test mcp_axis_moves` runs
`crates/finder/tests/mcp_axis_moves.rs` against the simulator in virtual time. The
simulator is sampled every 0.1 s of virtual time.

**Acceptance Scenarios**:

1. **Given** the Sun's centre 5° above the horizon, **When** a committed test calls
   `goto_target`, `goto_axis_position` and `move_axis`, **Then** it asserts that each
   result has `isError: true` saying the Sun is up, and that no axis moves (oracle: the
   simulator's axis state, exactly).
2. **Given** dusk, with the Sun 10° below the horizon, **When** a committed test calls
   `goto_axis_position` with the axis positions that the pointing model puts 5° from the
   Sun, **Then** it asserts that the result has `isError: true` naming the Sun guard, and
   that no axis moves.
3. **Given** dusk, **When** a committed test calls `move_axis` to turn the azimuth axis
   toward the Sun's azimuth at its top rate for 30 s, from 40° away, **Then** it asserts
   that the result has `isError: true` naming the Sun guard, and that no axis moves.
4. **Given** a session minimum altitude, **When** a committed test calls
   `goto_axis_position` with positions that the model puts 5° below it, **Then** it
   asserts that the result has `isError: true` saying the position is too low, and that no
   axis moves.
5. **Given** night, a sync, the telescope pointing 3° up and a minimum altitude of 15°,
   **When** a committed test calls `move_axis` to turn the altitude axis up at 1°/s for
   20 s, **Then** it asserts that the move is carried out, and that the axis is commanded
   to stop 20.0 ± 0.1 s of virtual time after the call (oracle: the simulator's command
   log).
6. **Given** the night's first sync, **When** a committed test calls `goto_axis_position`
   for an azimuth 1.2 turns from where that sync was made, **Then** it asserts that the
   result has `isError: true` naming the cable rule, and that no axis moves.
7. **Given** no sync yet that night, **When** a committed test calls `move_axis`, **Then**
   it asserts that the result has `isError: true` saying a sync is needed, and that no
   axis moves.
8. **Given** night and a sync, **When** a committed test calls `move_axis` to turn the
   altitude axis at 0.5°/s for 10 s, clear of every limit, **Then** it asserts two things:
   - the axis is commanded to stop 10.0 ± 0.1 s of virtual time after the call;
   - it has moved 5.0° less the simulated mount's acceleration ramp, within 0.1° (oracle:
     the simulator's motion model).
9. **Given** a session, **When** a committed test calls `move_axis` with a duration of
   60 s, **Then** it asserts that the result has `isError: true` and that no axis moves.
10. **Given** night and a sync, **When** a committed test calls `move_axis` to turn the
    azimuth axis for 20 s clear of every limit, and the client then closes its connection
    and sends nothing more, **Then** it asserts two things from the simulator's command
    and heartbeat logs:
    - the axis is commanded to stop 20.0 ± 0.1 s of virtual time after the call;
    - the console's heartbeats to the node carry on throughout.
11. **Given** a `move_axis` running, **When** the virtual clock passes the moment the
    Sun's centre reaches −0.83°, **Then** a committed test asserts that the axis is
    commanded to stop within 1 s of that moment, and that the session says the Sun guard
    ended the move.
12. **Given** night and a sync, **When** ten `move_axis` calls clear of every limit arrive
    within 1 s of virtual time, followed by `stop_all`, **Then** a committed test asserts,
    by exact count from the simulator's command log:
    - at most 2 calls are carried out;
    - the rest have `isError: true` saying the rate limit was reached;
    - `stop_all` stops both axes.
13. **Given** `tools/list`, **When** a committed test reads it, **Then** it asserts that
    `goto_axis_position` and `move_axis` are named, and that each one's description says
    it moves a real telescope.

---

### User Story 6 - Tracking over MCP (Priority: P2)

As an MCP client, I turn Track on or off as the page's toggle does, refused for the same
reasons. I read tracking's readout in `get_session`.

**Why this priority**: Tracking keeps a target in the eyepiece once a GoTo has put it
there. It is P2 because it is a thin passthrough over tracking that has already landed.

**Independent Test**: `cargo test -p finder --locked --test mcp_tracking` runs
`crates/finder/tests/mcp_tracking.rs` against the simulator in virtual time. The console
is fed the simulator's true pointing plus noise, as the long tracking tests do.

**Acceptance Scenarios**:

1. **Given** M42 and the simulator, **When** a committed test calls `set_tracking` to turn
   Track on and lets 10 min of virtual time pass, **Then** it asserts two things:
   - M42 stays within 1′ of the aim point (oracle: the simulator's true eyepiece
     direction, sampled every 1 s);
   - `get_session` reads a tracking error equal to the page's readout, and `motion.by` is
     `mcp` with `motion.tool` `set_tracking` while Track runs.
2. **Given** no mount connected, **When** a committed test calls `set_tracking`, **Then**
   it asserts that the result has `isError: true`, with the same reason
   `POST /api/track` gives.
3. **Given** the rate limit on moving tools, **When** a committed test calls `set_tracking`
   in a burst with `move_axis`, **Then** it asserts that `set_tracking` counts against the
   same limit of 2 calls a second.

---

### User Story 7 - The page says an MCP client is moving the telescope, and Setup shows the MCP address (Priority: P2)

As the observer at the eyepiece, while an MCP client moves the telescope, the guidance
block and the top bar say "Moved by an MCP client · goto_target", beside a Stop that stays
enabled. On Setup, an MCP section lists the URLs I can paste into a client, or says that
MCP is off, or reachable only from this machine. The README tells me how to reach the
endpoint from the phone under WSL2.

**Why this priority**: The owner chose no confirmation. The page's note and its Stop are
what keep the person at the eyepiece informed and in control.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`:
- the new Vitest files are `src/components/McpNote.test.tsx` and
  `src/components/McpSection.test.tsx`;
- the build-freshness check passes on the rebuilt `web/dist`;
- `npm run e2e` runs the new `e2e/mcp.spec.ts` in the `simulate` project, and every
  existing test passes unchanged.

**Acceptance Scenarios**:

1. **Given** the page open on a console in simulate mode, **When** a committed Playwright
   test in the `simulate` project calls `goto_target` over HTTP, **Then** it asserts three
   things:
   - within 1 s the page renders "Moved by an MCP client · goto_target" in the guidance
     block and the top bar;
   - Stop is enabled;
   - pressing Stop halts the GoTo (oracle: the simulator's axis state, exactly).
2. **Given** a `GET /api/mcp` answer for a console started with `--listen 0.0.0.0:8080`
   and `--mcp-allow-host surface.local`, **When** a committed Vitest test renders Setup's
   MCP section, **Then** it asserts two things:
   - the section lists every URL in the answer, in the answer's order, and
     `surface.local`;
   - it says that under WSL's NAT the phone needs the README's portproxy or mirrored-mode
     setup.
3. **Given** a `GET /api/mcp` answer for the default `--listen 127.0.0.1:8080`, **When** a
   committed Vitest test renders the section, **Then** it asserts that only
   `http://localhost:8080/mcp` is listed, with the note that MCP clients on other devices
   can't reach it until the console listens on the LAN. For an answer with MCP off, it
   asserts that the section says so.
4. **Given** the MCP note and Setup's MCP section in night mode, **When** a committed
   Playwright test runs the night pixel check on their screenshots, **Then** it asserts no
   pixel outside the red-only palette (tolerance 0 pixels). No existing test or golden is
   edited.
5. **Given** `crates/finder/README.md`, **When** it is read, **Then** it documents six
   things:
   - the `/mcp` URLs;
   - `--mcp-allow-host` and `--no-mcp`;
   - that MCP reaches the LAN only with `--listen 0.0.0.0:8080`;
   - that portproxy or mirrored mode carries it under WSL2;
   - that the Windows firewall rule should be scoped to the local subnet, because
     portproxy hides the peer address;
   - that every moving tool runs the page's guards.
6. **Given** `DESIGN.md` and `.impeccable/design.json`, **When** they are read, **Then**
   they describe the MCP note and Setup's MCP section as built. The rebuilt `web/dist` is
   the build of its source, and `package.json` dependencies are unchanged.

---

### Edge Cases

- **Protocol versions and session IDs:**
  - A request on any protocol revision, including the stateless 2026-07-28 one, works
    without an `Mcp-Session-Id`.
  - A client that sends one anyway is answered the same as without it.
- **Batches.** A JSON-RPC batch that passes every check runs all its calls. One that fails
  a check runs only its stops.
- **Two clients at once.** The page and an MCP client, or two MCP clients, share one
  session, and the last write wins. Stop from any of them ends every motion, whoever
  started it.
- **A client that loses its connection mid-move.** The console owns the motion from the
  moment the tool answers:
  - a GoTo ends by its own rules;
  - `move_axis` stops at the end of its duration (at most 30 s);
  - tracking runs until its own ends.
- **Rate limits:**
  - A `move_axis` rate above the axis's top rate is capped at the top rate, not refused.
  - A call refused by the rate limit moves nothing and does not count toward later
    windows.
- **Guard hints.** Annotations such as `readOnlyHint` are hints that a client may ignore,
  so no guard depends on them.

## Requirements *(mandatory)*

### Functional Requirements

**The endpoint and its door** (US1):

- **FR-001**: *(Owner approvals 1 and 7.)* The console SHALL serve MCP over Streamable
  HTTP at `/mcp`, on its own address and port, needing no token:
  - a POST of a JSON-RPC request SHALL get one `application/json` answer;
  - a notification SHALL get 202;
  - a GET SHALL get 405;
  - an unsupported `MCP-Protocol-Version` SHALL get 400.

  It SHALL need no `Mcp-Session-Id`. MCP SHALL be on by default, and `finder --no-mcp`
  SHALL turn the endpoint off. **Refs**: `docs/research/2026-10-25-mcp-server.md` §1–2,
  §5; MCP specification 2025-06-18, Transports (Streamable HTTP).
- **FR-002**: *(Owner approvals 3 and 4.)* The endpoint SHALL refuse a request with 403,
  and run nothing, when either of these holds:
  - its `Origin` names an origin other than the one it was sent to;
  - its `Host`, or the host in its `Origin`, is not an IP literal, `localhost`, or a name
    given with `--mcp-allow-host`.

  It SHALL allow a request with no `Origin`. From a request that fails these checks, it
  SHALL carry out a `tools/call` of `stop` or `stop_all` and nothing else. **Refs**: MCP
  specification 2025-06-18, Transports (Security Warning); RFC 6454 §7.
- **FR-003**: *(Owner approvals 4 and 10.)* The endpoint SHALL refuse a request with 403,
  and run nothing, when its peer address is not one of these:
  - loopback;
  - private (RFC 1918);
  - link-local;
  - an IPv6 unique local address.

  It then answers only the house LAN, even if the console's port becomes reachable from
  the internet. From such a request it SHALL still carry out a `tools/call` of `stop` or
  `stop_all`. **Refs**: RFC 1918; RFC 4193; RFC 3927; RFC 4291 §2.5.6; owner's decision
  of 2026-10-25.
- **FR-004**: Each tool SHALL call the same function as the HTTP route or action it passes
  through to, and take the session's JSON names. It SHALL return that route's JSON as
  `structuredContent` and as text. A refusal SHALL be a result with `isError: true` and
  the same reason the route's 400, 404, 409 or 413 carries. Read-only tools SHALL carry
  `readOnlyHint: true`, and JSON-RPC errors are kept for protocol faults. **Refs**: MCP
  specification 2025-06-18, Tools.
- **FR-016**: *(Owner approval 13.)* The endpoint SHALL be built on `rmcp`, pinned to the
  exact version `=3.5.1` with `default-features = false` and only the features `server`,
  `macros` and `transport-streamable-http-server`. It SHALL be configured with:
  - `legacy_session_mode` off;
  - `json_response` on;
  - empty `allowed_hosts` and `allowed_origins`, because the console's own layer makes
    those checks;
  - a request body limit of 44 MiB.

  The dependency SHALL build on Rust 1.96 and pass `cargo-deny` and `cargo-machete`.
  **Refs**: `docs/research/2026-10-25-mcp-server.md` §5; `deny.toml`.
- **FR-019**: *(Owner approval 14.)* The console SHALL answer a read-only `GET /api/mcp`
  with four things:
  - whether MCP is on;
  - whether only this machine can reach it;
  - the `/mcp` URLs its listen address serves: only `localhost` under a loopback listen
    address, and otherwise `localhost` plus one URL for each non-loopback address of the
    host;
  - the names given with `--mcp-allow-host`.

  **Refs**: `docs/research/2026-10-25-mcp-server.md` §6.

**Session and catalog tools** (US1, US2):

- **FR-005**: The endpoint SHALL offer `get_session`, passing through to
  `GET /api/session`, and `set_target`, passing through to `PUT /api/session` with a
  target only. **Refs**: 012 (One session for every device, Checked edits).
- **FR-020**: *(Owner approval 12.)* The endpoint SHALL offer at least three more tools:
  - `set_session`, for `site`, `min_alt_deg` and `target` only;
  - `search_targets`;
  - `get_guidance`, the session's pointability and push-to turn and tilt.

  `set_session` SHALL refuse the scope node's address and token. It SHALL also refuse a
  new site while a mount is connected, since the Sun guard reckons the Sun from the site.
  **Refs**: 012 (Checked edits, Search, Turn and tilt); GLOSSARY.md (pointable, push-to).

**Frames and the link** (US3):

- **FR-021**: The endpoint SHALL offer `solve_frame`, which takes an image as base64 and
  holds it to `POST /api/solve`'s limits once decoded. It SHALL also offer `get_pointing`,
  the session's latest pointing. **Refs**: 012 (The latest pointing); the finder
  console's request limits.
- **FR-006**: The endpoint SHALL offer at least `get_link`, giving the scope link's health
  and the live loop's state as the session reports them. `get_pointing` SHALL return the
  latest live solve while the live loop runs. **Refs**: 013 (Link health, Live solving,
  Fresh pointings only).

**Motion and Stop** (US4):

- **FR-007**: The endpoint SHALL offer these mount tools:
  - at least `describe_axes`, `get_axis_state`, `goto_axis_position`, `move_axis` and
    `stop_all`, mapping onto the scope link's mount calls;
  - `goto_target` and `stop`, mapping onto the console's GoTo and Stop.

  The scope node's token SHALL stay in the console and appear in no tool's result.
  **Refs**: 014 (Mount calls, Pointable targets only, Halts); ADR-0001; MCP security best
  practices (token passthrough).
- **FR-008**: *(Owner approval 9.)* A moving tool from an MCP client SHALL start the motion
  before it answers, with no confirmation on the page. It SHALL record the motion in the
  session as coming from an MCP client. **Refs**: FR-014; owner's decision of
  2026-10-25.
- **FR-012**: *(Owner approval 4.)* `stop` and `stop_all` SHALL meet five rules:
  - They are listed in every state.
  - They are never refused or limited, whatever the request's headers or peer.
  - They end any GoTo, direct axis move or tracking in progress, whoever started it,
    before the console sends stop all.
  - A stop is carried out even when the answer must be an error, such as an unsupported
    `MCP-Protocol-Version`.
  - The console carries out a stop itself, through the same Stop function as
    `POST /api/stop`, before the request reaches `rmcp`.

  **Refs**: 013/014 (Stop wins, and a watchdog; Halts; GoTo and Stop on the console); MCP
  specification 2025-06-18, Transports (Protocol Version Header).
- **FR-014**: While a GoTo, a direct axis move or tracking runs, the session SHALL hold
  `motion`, with three fields:
  - `by`: `page` or `mcp`;
  - `tool`: the action or tool that started it;
  - `since`: when it started.

  It SHALL clear `motion` when the motion ends. A session file without `motion` SHALL
  load as having none. **Refs**: FR-008; 012 (Same-origin writes).
- **FR-017**: Every moving or stopping MCP call SHALL be written to stderr with its tool,
  its arguments and its result. **Refs**: plan.md § Decisions (who moved it).

**Direct axis moves** (US5):

- **FR-009**: *(Owner approvals 2, 5 and 11.)* Every tool that can move the telescope SHALL
  run the page's guards before and during the move, with no argument that turns one off:
  - the Sun guard on motion;
  - the minimum altitude;
  - the one-turn cable rule;
  - Stop wins;
  - the node's watchdog.

  A direct axis move SHALL be refused when the night has no sync, because the console
  cannot then tell where it would point. The minimum altitude along a direct axis move's
  path SHALL be judged against the lower of the move's starting altitude and the minimum
  altitude. **Refs**: 014 (The Sun guard on motion, Planning a slew, Halts, Pointable
  targets only); ADR-0001; GLOSSARY.md (Sun guard, minimum altitude, direct axis move).
- **FR-010**: *(Owner approval 6.)* `move_axis` SHALL take a signed rate, capped at the
  axis's top rate, and a duration of at most 30 s. It SHALL refuse a move whose path over
  that duration, including its stopping distance, would break a guard. It SHALL stop the
  axis in three cases:
  - when the duration ends;
  - on Stop;
  - in time to be at rest before any limit that a change brings into its path while it
    runs, such as the Sun rising or a new minimum altitude.

  **Refs**: 014 (Mount calls, Halts, Stop wins and a watchdog).
- **FR-011**: *(Owner approval 8.)* The endpoint SHALL accept at most 2 calls a second to
  the tools that can move the telescope, across all MCP clients. It SHALL refuse the
  excess with `isError: true`, and move nothing for them. Reads, `stop` and `stop_all`
  SHALL NOT be limited. **Refs**: MCP specification 2025-06-18, Tools (Security
  Considerations).

**Tracking** (US6):

- **FR-013**: The endpoint SHALL offer at least `set_tracking`, which turns Track on or off
  as the page's toggle does and is refused for the same reasons. `get_session` SHALL
  include tracking's readout. **Refs**: 015 (Ending safely, Track in the session, Track on
  the console).

**The page and the README** (US7):

- **FR-015**: *(Owner approvals 9 and 15.)* While the session's `motion.by` is `mcp`, the guidance
  block and the top bar SHALL say that the telescope is being moved by an MCP client,
  naming the tool, beside a Stop that stays enabled. Setup SHALL show an MCP section with
  three things from `GET /api/mcp`:
  - the endpoint's URLs that the listen address serves;
  - the allowed host names;
  - whether MCP is off, or reachable only from this machine.

  Both views SHALL follow the red-only night rule. **Refs**: FR-014, FR-019; DESIGN.md;
  `docs/research/2026-10-25-mcp-server.md` §6.
- **FR-018**: `crates/finder/README.md` SHALL document six things:
  - the MCP URLs;
  - `--mcp-allow-host` and `--no-mcp`;
  - that MCP reaches the LAN only with `--listen 0.0.0.0:8080`;
  - that portproxy or mirrored mode carries it under WSL2;
  - that the Windows firewall rule should be scoped to the local subnet;
  - that every moving tool runs the page's guards.

  `DESIGN.md` and `.impeccable/design.json` SHALL record the MCP note and Setup's MCP
  section as built. **Refs**: `docs/research/2026-10-25-mcp-server.md` §6;
  `docs/research/2026-10-21-scope-node-runtime.md` §5.

### Key Entities

- **The MCP endpoint**: `/mcp` on the console's router. Its door makes the `Origin`,
  `Host` and peer checks, with the stop exemption, in front of `rmcp`'s tower service,
  which is configured as FR-016 says.
- **Tool**: a name, a JSON input schema, an `outputSchema` and annotations. Each tool is a
  thin call into the function behind a console route or action (plan.md § Interfaces).
- **Motion source**: `motion: {by: "page" | "mcp", tool, since}` in the session, present
  only while something moves.
- **MCP status**: `GET /api/mcp` answers
  `{enabled, local_only, urls: [...], allowed_hosts: [...]}`.
- **Move limiter**: one shared window of 2 moving calls a second.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it.
  `cargo test --workspace --locked`, `npm test` and `npm run e2e` pass.
- **SC-002**: No tool can move the simulated telescope with the Sun's centre above
  −0.83°, or along a path within 15° of the Sun. This is shown by the refusals in US4 and
  US5, with the simulator's axis state unchanged in every one.
- **SC-003**: Through MCP, a stop reaches both axes within 0.1 s of virtual time in every
  case: wrong headers, wrong peer, wrong protocol version, no `initialize`, or a batch.
- **SC-004**: No existing test, golden, `scripts/ci/` file, `.github/workflows/ci.yml` or
  `CODEOWNERS` entry is edited, and workspace line coverage stays at or above 80%.
- **SC-005**: The scope node's token appears in no MCP result, and in no log line.

## Assumptions

- **Earlier features are in place.** 012, 013, 014 and 015 have landed: the observing
  session and its routes, the scope link and the simulator, GoTo and Stop with the pointing
  model and planner, and tracking. Their functions are what the tools call.
- **Out of scope:**
  - MCP on the scope node;
  - MCP resources, prompts and sampling;
  - server-to-client streams (a GoTo's progress is read with a tool, as the page polls);
  - OAuth;
  - a chat panel on the page;
  - the mobile app and SkySafari.
- **Outside the factory:**
  - the maintainer's Impeccable review of the new views, after US7 lands;
  - the owner's first use from a real MCP client (Claude Code or Claude Desktop on the
    Surface, or the phone) against a real mount at the telescope.
- **No large files are committed.** The 32 MiB test images are synthesised at test time.
  No operator-staged file is needed.
- **The factory's gates and CI:**
  - The factory's local gates are `lint` and `test`, which are cargo only.
  - US7 must also pass `npm run typecheck`, `npm run format:check`, `npm test`, the
    build-freshness check and `npm run e2e`, all of which run in CI's required
    `finder-page` check in the merge queue.
  - Its tasks run them before proposing.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-016, FR-019]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-020]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-006, FR-021]
US4:
  depends_on: []
  depends_on_merged: [US3]
  implements: [FR-007, FR-008, FR-012, FR-014, FR-017]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-009, FR-010, FR-011]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-013]
US7:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-015, FR-018]
```

Every edge is a merge edge, because each dependent imports what its predecessor creates:

| Story | Builds on |
|---|---|
| US2, US3 | the endpoint, its door and `src/mcp/tools.rs` |
| US4 | the door, for the stop exemption |
| US5 | US4's `motion`, its Stop path and its tools |
| US6 | US5's shared move limiter, which `set_tracking` joins |
| US7 | US4's `motion` and `goto_target`, for the Playwright flow; and US1's `GET /api/mcp` |

US7 touches only `crates/finder/web/`, `crates/finder/README.md`, `DESIGN.md` and
`.impeccable/design.json`, so it may run beside US5 and US6. Those two touch only
`crates/finder/src/` and `crates/finder/tests/`.
