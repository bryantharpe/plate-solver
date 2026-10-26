## Context

See proposal.md for why. After feat-15:
- the console holds one observing session, with its injectable `Clock`, state file and
  same-origin rule, and serves the page and `/api/*` from one axum router;
- the console is the only client of the scope node that holds its token; the scope link carries
  frames and feat-14's mount calls, and anyone on the network may call stop all;
- GoTo, Stop and Track each run behind the page's guards: the Sun guard on motion, the minimum
  altitude and horizon, the path planner's mechanical range and cable rule, stop-before-reverse,
  and the node's 1 s watchdog;
- the simulator stands in for the scope node, on a virtual clock, and exposes its true eyepiece
  direction.

feat-12 to feat-15 must be archived before this change; its deltas only ADD. Its tasks can land
in steps after each of them (tasks.md).

**The owner's decisions (2026-10-25).**
- The scope node is controlled only over the `scope_node.v1` link. There is no MCP on the Pi.
- The console, in WSL2 on the Surface, exposes telescope control over MCP as a passthrough to
  that link and to its own actions, reachable over the house LAN.
- Open on the LAN: MCP clients need no token. `Origin` is still validated.
- An MCP client's GoTo or direct axis move runs straight away, with no tap to confirm.
- Safety is not passed through (below).

**Sources:** `docs/research/2026-10-25-mcp-server.md` (the MCP specification's Streamable HTTP
transport, its security guidance and tools; the `rmcp` crate), and the runtime note's §5 on WSL2
networking.

## Goals / Non-Goals

**Goals:**
- Let an MCP client do everything the page does, and call the mount directly, with the same
  answers the page gets.
- Make it impossible for an MCP client to move the telescope anywhere the page could not.
- Reach the console from the Surface and the phone with no setup beyond what the page needs.

**Non-Goals:**
- MCP on the scope node, or giving any MCP client the scope node's token.
- Authentication (owner's decision), OAuth, and MCP resources, prompts or sampling.
- Server-to-client streams. A GoTo's progress is read with a tool, as the page polls.
- A chat panel on the page (PRODUCT.md's AI panel is a later change).

## Decisions

### One endpoint, on the console's own port

`/mcp`, Streamable HTTP, mounted in the console's existing router as `rmcp`'s tower service:
- POST carries one JSON-RPC message; requests get one `application/json` answer, notifications
  and responses get 202;
- GET answers 405, because the console sends nothing unasked;
- no `Mcp-Session-Id` is needed: all state is the observing session, which every client shares
  with the page. That works with the stateless 2026-07-28 revision and the older ones, where the
  session ID is optional (research note §1);
- an unsupported `MCP-Protocol-Version` gets 400, as the transport requires.

**`rmcp`'s defaults don't fit and are set explicitly** (research note §5, read from 3.5.1's
`StreamableHttpServerConfig`):
- `legacy_session_mode` defaults to true (a session per client); it is set false, so no
  `Mcp-Session-Id` is needed on the older revisions either.
- `json_response` defaults to false (SSE answers); it is set true. The console's tools send no
  notification before their result, so every answer is `application/json`.
- `allowed_hosts` defaults to loopback names only, matched exactly, which would refuse the phone
  by IP. It is set empty (rmcp then checks no `Host`) and `allowed_origins` left empty, because
  the console's own layer in front of the service makes the checks below, including the stop
  exemption that rmcp's checks would break.
- `max_request_body_bytes` defaults to 4 MiB, which would cap `solve_frame` at a 3 MiB image.
  It is set to 44 MiB, enough for a 32 MiB image as base64 (4/3 of it, plus the JSON around it),
  so `solve_frame` meets `POST /api/solve`'s own 32 MiB limit and that route's 413 reason.

*Alternative:* a second port for MCP. Rejected: under WSL2 every port needs its own portproxy
or Hyper-V firewall rule, and two ports double what can go wrong at the telescope.

*Alternative:* stdio. Rejected: the phone can't launch a process in WSL.

### Reaching it under WSL2

The same as the page (research note §6, runtime note §5). `/mcp` is served wherever the page is,
so it reaches the LAN only when the page does: the console's default `--listen` is
`127.0.0.1:8080`, which keeps MCP on the Surface, and the finder README's
`--listen 0.0.0.0:8080` opens both. MCP adds no flag that widens the listen address.
- On the Surface, Claude Code in WSL and Claude Desktop on Windows reach
  `http://localhost:<port>/mcp` (NAT mode forwards Windows' localhost to WSL; mirrored mode shares
  it).
- The phone reaches `http://<surface>:<port>/mcp` through whatever the owner set up for the
  page: mirrored mode with a Hyper-V firewall rule (what the finder README documents), or
  `netsh interface portproxy` and a Windows firewall rule under NAT.
- Setup lists the URLs the console can see for itself (localhost and each non-loopback address
  of the host), so the owner can paste one into a client. Under NAT the WSL address it sees is
  not the Surface's LAN address; Setup says so and names the README's section.

### Open, with `Origin`, `Host` and the peer checked

The MCP specification says servers MUST validate `Origin` against DNS rebinding, SHOULD bind to
localhost, and SHOULD authenticate. The console keeps the MUST and, by the owner's decision,
not the two SHOULDs: it listens where the page listens, and asks for no token.

**What it checks, before running anything:**
1. `Origin`, if present, must name the origin the request was sent to (feat-12's rule).
2. The host in `Host`, and in `Origin` when present, must be an IP literal, `localhost`, or a
   name given with `--mcp-allow-host` (repeatable; for example the Surface's `.local` name).

3. The peer address must be loopback, private (RFC 1918), link-local, or an IPv6 unique local
   address (`fc00::/7`).

Rule 2 is the addition against DNS rebinding. A rebinding page's requests carry the attacker's
name in both `Origin` and `Host`, so rule 1 alone passes them. Real MCP clients send no
`Origin`, and reach the console by an address or a name the owner listed. A failure is 403 with
a JSON-RPC error and no id; nothing runs.

Rule 3 keeps "open on the LAN" on the LAN. With no token, a router port forward, a global IPv6
address with an open firewall, or a public network the Surface joins would otherwise hand the
telescope to the internet. Under WSL's NAT, portproxy hides the real source and the console sees
the Windows host's private address, so there the Windows firewall rule's scope (local subnet)
does this job; the README says so. A phone that reaches the Surface by a global IPv6 address is
refused; Setup lists IPv4 and link-local URLs for that reason.

**Stop is exempt**, as on the page: a `tools/call` for `stop` or `stop_all` is carried out even
when a check fails, because a stop must never be refused. The body is read to see whether it is
a stop; if it is not, nothing in it runs. From a JSON-RPC batch (allowed by revisions before
2025-06-18) that fails a check, only the stops run, so a move can't ride in on a stop. A stop
is also carried out when the answer must be an error for another reason, such as an unsupported
`MCP-Protocol-Version`: the 400 still goes back, after the stop.

*Alternative:* a token, as the scope link has. Rejected by the owner: clients on the house LAN
are trusted. Listed below to confirm.

### Passthrough means the same functions

Each tool is a thin call into the function behind the matching HTTP route, so the answers can't
drift. Arguments use the session's JSON names; results are the same JSON the route returns, as
`structuredContent` with an `outputSchema`, and the same JSON as text for older clients.

| Tool | Passes through to | Lands after | Moves the telescope |
|---|---|---|---|
| `get_session` | `GET /api/session` | feat-12 | no |
| `set_session` | `PUT /api/session` (site, minimum altitude, target; never the scope node) | feat-12 | no |
| `search_targets` | `GET /api/targets?q=` | feat-12 | no |
| `get_guidance` | the session's `guidance` (pointability and push-to's turn and tilt) from `GET /api/session` | feat-12 | no |
| `set_target` | `PUT /api/session` with a target only | feat-12 | no |
| `solve_frame` | `POST /api/solve` (an image as base64, held to its limits) | feat-12 | no |
| `get_pointing` | the session's `pointing` from `GET /api/session` | feat-12 | no |
| `get_link` | the scope link's health and the live loop's state, as `GET /api/session` reports them | feat-13 | no |
| `describe_axes` | the mount call "describe the axes" | feat-14 | no |
| `get_axis_state` | the mount call "axis state" | feat-14 | no |
| `goto_axis_position` | the mount call "go to a position", after the guards | feat-14 | yes |
| `move_axis` | the mount call "move at a rate", after the guards | feat-14 | yes |
| `goto_target` | `POST /api/goto` | feat-14 | yes |
| `stop` | `POST /api/stop` | feat-14 | stops |
| `stop_all` | `POST /api/stop`, which ends in the mount call "stop all" | feat-14 | stops |
| `set_tracking` | `POST /api/track` | feat-15 | yes |

- `set_session` takes only `site`, `min_alt_deg` and `target`. The scope node's address and
  token (feat-13) are refused: an MCP client that could change them could point the console at a
  node of its own, or break its pairing. The token is never read back in any case.
- While a mount is connected, `set_session` also refuses a new site. The Sun guard reckons where
  the Sun is from the site and the console's clock; a client that moved the site by a few hours
  of longitude (75° E for 75° W) would make the Sun look set at noon, and the syncs already made
  would no longer fit. The clock can't be set outside tests and `--simulate` (feat-12).
- Read-only tools carry `readOnlyHint: true`; the moving ones say in their description that they
  move a real telescope. Annotations are hints a client may ignore, so no guard depends on them.
- `stop_all` goes through the console's Stop rather than straight to the link, so a GoTo or
  tracking in progress ends too. A bare stop all on the link would be undone by the GoTo's next
  correction.
- A refusal (the Sun is up, no sync, a bad argument) is a tool result with `isError: true` and
  the same reason the page's 409 or 400 carries, so the model can read it. JSON-RPC errors are
  kept for protocol faults.

### No confirmation

A moving tool starts the motion before it answers. The page shows it (below) and Stop stays on
every page; there is no tap to confirm. The MCP specification's advice to keep a human in the
loop is left to the client, which may ask its own user.

### Safety is not passed through

This is the one place where "passthrough" isn't literal. Per ADR-0001 the console owns pointing
and the Sun guard lives in the console; the scope node only drives axes and can't tell where
they point on the sky. A raw mount call passed straight to the link would skip the Sun guard,
and a 16-inch aperture pointed at the Sun is a fire and eye hazard: it can set the finder, the
spider or a hand alight, and blind anyone at the eyepiece in an instant. So every moving tool
runs the same guards as the page's actions:
- the Sun guard on motion (no motion with the Sun's centre above −0.83°, no path within 15° of
  the Sun);
- the minimum altitude (never below 0°, the horizon), at the end of the move and along its
  path;
- feat-14's path checks: the one-turn cable rule (azimuth kept within one turn either way of its
  position at the night's first sync), and any range limit the mount adapter reports;
- Stop wins, on the console and on the node, and the node's 1 s watchdog;
- tracking's own ends (sunrise, sinking below the minimum altitude).

**Direct axis moves.** The page has none, so `goto_axis_position` and `move_axis` get a guard
of their own, built from the same pieces:
- They need a sync, because without the pointing model the console can't turn axis positions
  into a sky direction, so it can't check the Sun. With no sync they are refused.
- `goto_axis_position` turns the requested positions into a direction through the model, plans
  the path as feat-14's planner does (one axis at a time, in the order that keeps clear) and
  refuses it when no path keeps clear.
- `move_axis` takes a signed rate (capped at the axis's top rate) and a duration of at most 30 s.
  The console forecasts the path for that duration, including the distance the axis needs to
  come to rest; it refuses the move when the forecast crosses a limit. While it runs it
  re-forecasts on the console's clock (the Sun rises, the minimum altitude is raised) and stops
  the axis in time to be at rest before any limit. It stops the axis at the end of the duration.
- The minimum altitude along a direct axis move's path is judged against the lower of the
  move's starting altitude and the minimum altitude, so a telescope parked low can climb, as a
  GoTo from low down can, but no move takes it lower once it is below the minimum altitude.

*Alternative:* pass the raw calls through and trust the node's watchdog. Rejected: the watchdog
catches a silent console, not a bad direction.

### A rate limit on moves

The tools page says servers MUST rate limit tool invocations. Moving tools share one limit of 2
calls a second across all clients; a call over it is refused with `isError: true` and moves
nothing. Reads, `stop` and `stop_all` are never limited. That stops a looping client from
flooding the scope link's queue, and leaves Stop untouched.

### Who moved it

The session gains `motion: {by, tool, since}`, set when a GoTo, axis move or tracking starts
and cleared when it ends; `by` is `page` or `mcp`. The page's guidance block and the top bar
show "Moved by an MCP client" while `by` is `mcp`, beside the Stop that is always there. Every
moving or stopping MCP call is also written to stderr with its tool, arguments and result.

### Tests through the router

The Rust tests post JSON-RPC to `/mcp` with `tower::ServiceExt::oneshot`, as the HTTP tests do,
so they need no client crate and no socket. Each read tool is compared with its route's answer
for the same input, exactly. Each moving tool runs against the simulator in virtual time, with
the true eyepiece direction sampled every 0.1 s as the oracle. One Playwright flow in the
`simulate` project calls `goto_target` over HTTP and checks the page's note and Stop.

## Risks / Trade-offs

- **Anyone on the house Wi-Fi can move the telescope.** → The owner's call. The guards bound
  what they can do, Stop is on every page, and the stderr log shows who did what. A token can be
  added later without changing the tools.
- **An MCP client moves the telescope while someone is at the eyepiece.** → No confirmation, by decision.
  The page says an MCP client is moving it, and Stop wins.
- **A client that loses its connection mid-move.** → Every moving tool answers as soon as the
  motion starts, and the console owns the motion from then on: a GoTo ends by its own rules,
  `move_axis` stops at the end of its duration (at most 30 s), and tracking runs until its own
  ends (Stop, a lost link, sunrise, the target sinking below the minimum altitude), as when the
  page started it. The node's 1 s watchdog covers a lost console, not a lost MCP client.
- **`rmcp` releases often** (five in a month). → Pin an exact version at least two weeks old and
  update deliberately; the endpoint's tests catch protocol changes.
- **Two clients at once** (the page and an MCP client, or two MCP clients). → One session, last write wins, as
  with two browsers today; Stop from any of them ends every motion.

## Migration Plan

- No data migration: `motion` is a new, optional field in the session file, absent in old ones.
- Tasks land in steps after feat-12, feat-13, feat-14 and feat-15; the change archives after
  all four.
- `finder --no-mcp` turns the endpoint off.

## Decisions for the owner

1. **Safety is not passed through.** Every moving tool goes through the Sun guard, the minimum
   altitude, the horizon, the motion and path checks, the cable rule, Stop wins and the
   watchdog, as above. Recommended: confirm. This is the one place where "passthrough" isn't
   literal.
2. **The `Host` check** (an IP literal, `localhost`, or a name from `--mcp-allow-host`), beyond
   feat-12's `Origin` rule, because the `Origin` rule alone lets a DNS-rebinding page through.
   Recommended: confirm. feat-12's page routes have the same gap; this change doesn't alter them
   (see Open Questions).
3. **Stop and stop all skip the `Origin`, `Host` and peer-address checks**, as `POST /api/stop`
   does, and are carried out even when the answer is an error (an unsupported
   `MCP-Protocol-Version`). From a request that fails a check only the stops run. Recommended:
   confirm.
4. **Direct axis moves need a sync.** Recommended: confirm; without the model the Sun guard
   can't be checked.
5. **`move_axis` takes a duration of at most 30 s** and stops at its end. Recommended: 30 s.
6. **MCP is on by default**, with `finder --no-mcp` to turn it off. Recommended: on.
7. **A rate limit of 2 moving calls a second**, across all clients, with reads and Stop
   unlimited. Recommended: 2 per second.
8. **No confirmation on the page** for an MCP motion, as decided; the page shows "Moved by an
   MCP client". Recommended: keep the note.
9. **LAN peers only:** `/mcp` refuses a peer address outside loopback, RFC 1918, link-local and
   IPv6 unique local ranges, so "open on the LAN" can't become open to the internet through a
   port forward or a global IPv6 address. It is added in review. Recommended: confirm. Under
   WSL's NAT the Windows firewall rule's scope does this job instead (above).
10. **A telescope parked low may climb.** A direct axis move that starts below the minimum
   altitude may move only upward or level, as a GoTo from low down may. Added in review.
   Recommended: confirm.
11. **No new site over MCP while a mount is connected**, because a wrong site would blind the
   Sun guard. Added in review. Recommended: confirm. The page can still change the site at any
   time (see Open Questions).

## Open Questions

- Should feat-12's same-origin rule for the page's routes gain the same `Host` check? That
  would be a change to feat-12's requirement and is left for the owner; this change touches
  only `/mcp`.
- Should a token be offered as an option later, for a network the owner doesn't control (a star
  party)? Not specified here; rule 3 lets in anyone on such a network's private range.
- Should the page also refuse a new site while a mount is connected? That would change feat-12's
  "Checked edits" through feat-14 and is left for the owner.
- Should feat-12's page routes refuse non-LAN peers as `/mcp` does? Left for the owner with the
  `Host` question above.
