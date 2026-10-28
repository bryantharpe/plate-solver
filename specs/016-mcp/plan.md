# Implementation Plan: The Console's MCP Server

**Branch**: `016-mcp` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Mount an MCP endpoint, `/mcp`, in the console's existing axum router, as `rmcp`'s
Streamable HTTP tower service, behind a door of the console's own. The door checks
`Origin`, `Host` and the peer address, and carries out any stop itself. Each tool is a
thin call into the function behind a console route or action.

Every tool that moves the telescope runs the page's guards. Direct axis moves get a
guard of their own, built from the same pieces. The session records who started each
motion, and the page says so. No change reaches the scope node, its proto or the link,
and `crates/math-core` is untouched.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI. TypeScript with React 19 for the
page.

**Primary Dependencies**:
- already in `crates/finder`: axum 0.8, tokio, serde, clap and tower (dev);
- new: `rmcp` (§ Dependencies);
- the page: Vite, Vitest and Playwright, with no new npm package.

**Storage**: the session file gains an optional `motion` field. Nothing else is
persisted.

**Testing**:
- Rust integration tests in new files under `crates/finder/tests/`. They post JSON-RPC to
  `/mcp` through `router(..)` with `tower::ServiceExt::oneshot`, with peers set through
  axum's `MockConnectInfo`.
- The simulator runs as 013 sets it up: over an in-memory `tokio::io::duplex` transport,
  on a `VirtualClock`, under `#[tokio::test(start_paused = true)]`. Its true eyepiece
  direction, axis state, command log and heartbeat log are the oracles.
- Vitest and one Playwright file in the `simulate` project.

**Target Platform**: the `finder` binary in WSL2 on the Surface; clients on the Surface
(Claude Code, Claude Desktop) and on the phone.

**Constraints**:
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy and rustdoc, the 80% coverage floor,
  and the red-only night mode.
- `web/dist` is committed and must be the build of its source.
- No test moves real hardware or calls a real LLM.

**Scale/Scope**: one console, a handful of MCP clients on the house LAN, and 16 tools.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver or `crates/math-core`. `solve_frame` calls the same function as `POST /api/solve`. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml` or `CODEOWNERS` is edited. New tests go in new files. The owner's decisions are listed in spec § Owner approvals. |
| III. Tests first, independent expectations | Each read tool is compared with its route's JSON. Each moving tool is checked against the simulator's true eyepiece direction, axis state and command log, in virtual time. |
| IV. Critical paths | One dependency change (`rmcp`, plus its tree in `Cargo.lock`), owner-approved. No `unsafe`, no proto change, no database-format change. |
| V. Standalone Rust, honest licensing | `rmcp` is Apache-2.0, and its whole tree passes `deny.toml`'s allow-list (§ Dependencies). |

## Dependencies

**`rmcp` = `=3.5.1`**, the official MCP Rust SDK:
- `default-features = false`, with features `server`, `macros` and
  `transport-streamable-http-server`;
- it is Apache-2.0, as is `rmcp-macros`;
- its MSRV is 1.88, below this repository's 1.96.

3.5.1 (2026-10-05) is the newest release at least two weeks old on 2026-10-28; 3.6.0 is
dated 2026-10-20. Releases are frequent, so the pin is exact and moves only deliberately.

**What it pulls in:** tokio, tokio-util, tokio-stream, serde, serde_json, schemars 1 (with
chrono), uuid, pastey, async-trait, sse-stream, tower-service, http, http-body,
http-body-util, bytes, rand 0.10 and base64.

**Licences.** These were checked on 2026-10-25 with a scratch crate on `cargo +1.96 check`
and `cargo metadata`: all 91 packages are MIT, Apache-2.0, or combinations satisfied by
`deny.toml`'s allow-list. One is `(MIT OR Apache-2.0) AND Unicode-3.0`, and Unicode-3.0 is
allowed.

**Possible duplicate versions.** `cargo deny` may flag `rand` or `base64` as duplicates of
versions already locked. That is a `[bans]` warning to settle in US1 without weakening
`deny.toml`. If it cannot be settled, US1 escalates.

**Rejected:** writing JSON-RPC by hand. The SDK owns protocol negotiation, schemas and
transport details that change between revisions.

## Project Structure

```text
Cargo.lock                                # US1: rmcp and its tree
crates/finder/
├── Cargo.toml                            # US1: rmcp =3.5.1
├── README.md                             # US7: the MCP section
├── src/
│   ├── lib.rs                            # US1: `pub mod mcp;`
│   ├── mcp/mod.rs                        # US1: service config, mounting, McpConfig
│   ├── mcp/door.rs                       # US1: Origin/Host/peer; US4: stops carried out here
│   ├── mcp/tools.rs                      # US1 get_session, set_target; US2 session and catalog;
│   │                                     # US3 frames and link; US4 mount, GoTo, Stop;
│   │                                     # US5 direct moves; US6 set_tracking
│   ├── mcp/guard.rs                      # US5: direct-move guard and forecast
│   ├── mcp/limit.rs                      # US5: the shared move limiter
│   ├── api.rs                            # US1: /mcp mount, GET /api/mcp, AppState::with_mcp
│   ├── session.rs                        # US4: `motion`
│   ├── goto.rs                           # US4: GoTo records motion.by (page or mcp)
│   ├── tracking.rs                       # US6: Track records motion.by (page or mcp)
│   └── main.rs                           # US1: --no-mcp, --mcp-allow-host, connect info
├── tests/
│   ├── mcp_endpoint.rs                   # US1
│   ├── mcp_session_tools.rs              # US2
│   ├── mcp_frames.rs                     # US3
│   ├── mcp_motion.rs                     # US4
│   ├── mcp_axis_moves.rs                 # US5
│   └── mcp_tracking.rs                   # US6
└── web/
    ├── src/components/McpNote.tsx (+ test)     # US7: the note in GuidanceBlock and TopBar
    ├── src/components/McpSection.tsx (+ test)  # US7: Setup's MCP section
    ├── src/components/GuidanceBlock.tsx, TopBar.tsx, SetupPage.tsx   # US7: wiring only
    ├── dist/                                   # US7: rebuilt
    ├── playwright.config.ts                    # US7: only if needed (owner approval 15)
    └── e2e/mcp.spec.ts                         # US7: in the `simulate` project
DESIGN.md, .impeccable/design.json              # US7
```

`goto.rs` and `tracking.rs` are the modules that 014 and 015 add. US4 and US6 change only
how they record the motion's source.

## Interfaces

### The endpoint

`/mcp` uses Streamable HTTP, mounted in the console's router as `rmcp`'s tower service
behind `mcp::door`:
- A POST carries one JSON-RPC message, or a batch on revisions before 2025-06-18.
  Requests get one `application/json` answer; notifications and responses get 202.
- A GET gets 405, because the console sends nothing unasked.
- No `Mcp-Session-Id` is needed. All state is the observing session, which every client
  shares with the page. That works with the stateless 2026-07-28 revision, and with the
  older ones, where the session ID is optional.
- An unsupported `MCP-Protocol-Version` gets 400.

### `rmcp` settings

The defaults don't fit, so each is set explicitly (read from 3.5.1's
`StreamableHttpServerConfig`):

| Setting | Default | Set to | Why |
|---|---|---|---|
| `legacy_session_mode` | true | false | No `Mcp-Session-Id` is needed on older revisions either. |
| `json_response` | false | true | Tools send no notification before their result, so every answer is `application/json`. |
| `allowed_hosts` | loopback names, matched exactly | empty | Exact loopback names would refuse the phone by IP. `rmcp` then checks no `Host`, and the door makes the checks, including the stop exemption that `rmcp`'s checks would break. |
| `allowed_origins` | empty | empty | The door checks `Origin`. |
| `max_request_body_bytes` | 4 MiB | 44 MiB | 4 MiB would cap `solve_frame` at a 3 MiB image. 44 MiB fits a 32 MiB image as base64 (4/3 of it, plus the JSON around it), matching `POST /api/solve`'s own 32 MiB limit and its 413 reason. |

### The door (`mcp::door`)

The door is a tower layer in front of `rmcp`. It runs before anything else, in this order:
1. **Origin.** An `Origin`, if present, must name the origin the request was sent to.
2. **Host.** The host in `Host`, and in `Origin` when present, must be an IP literal,
   `localhost`, or a `--mcp-allow-host` name (repeatable). A DNS-rebinding page sends the
   attacker's name in both `Origin` and `Host`, so check 1 alone passes it. Real MCP
   clients send no `Origin`, and reach the console by an address or a listed name.
3. **Peer.** The peer address (`ConnectInfo<SocketAddr>`) must be loopback, RFC 1918,
   link-local (169.254/16, fe80::/10) or IPv6 unique local (fc00::/7). Under WSL's NAT,
   portproxy hides the real source, so the Windows firewall rule's scope does this job
   there. A phone that reaches the Surface by a global IPv6 address is refused.

**When a check fails.** The answer is 403 with a JSON-RPC error and no id, and nothing
runs. The exception is the stop path below.

**The stop path.** The door reads the body to see whether it holds a `tools/call` of
`stop` or `stop_all`, including inside a batch. For each stop, it calls the same Stop
function as `POST /api/stop` itself, before the request reaches `rmcp`. Then:
- if a check failed, the door answers 403 and runs nothing else in the body, so a move
  can't ride in on a stop;
- otherwise, the request goes on to `rmcp`, which builds the answer. The answer may be an
  error, such as a 400 for an unsupported `MCP-Protocol-Version`, a missing `initialize`
  or a missing `Accept`. The stop has already been carried out.

**The binary serves with connect info.** `main.rs` serves with
`into_make_service_with_connect_info::<SocketAddr>()`, so the peer check sees the real
peer. In tests, `MockConnectInfo` sets the peer.

### The tool table

| Tool | Passes through to | Story | Moves |
|---|---|---|---|
| `get_session` | `GET /api/session` | US1 | no |
| `set_target` | `PUT /api/session` with a target only | US1 | no |
| `set_session` | `PUT /api/session`: site, minimum altitude and target, never the scope node | US2 | no |
| `search_targets` | `GET /api/targets?q=` | US2 | no |
| `get_guidance` | the session's `guidance` from `GET /api/session` | US2 | no |
| `solve_frame` | `POST /api/solve`: an image as base64, held to its limits | US3 | no |
| `get_pointing` | the session's `pointing` from `GET /api/session` | US3 | no |
| `get_link` | the scope link's health and the live loop's state, as the session reports them | US3 | no |
| `describe_axes` | the mount call "describe the axes" | US4 | no |
| `get_axis_state` | the mount call "axis state" | US4 | no |
| `goto_target` | `POST /api/goto` | US4 | yes |
| `stop` | `POST /api/stop` | US4 | stops |
| `stop_all` | `POST /api/stop`, which ends in the mount call "stop all" | US4 | stops |
| `goto_axis_position` | the mount call "go to a position", after the guards | US5 | yes |
| `move_axis` | the mount call "move at a rate", after the guards | US5 | yes |
| `set_tracking` | `POST /api/track` | US6 | yes |

**Arguments and results:**
- Arguments use the session's JSON names.
- Results are the route's JSON, as `structuredContent` with an `outputSchema`, and the
  same JSON as text for older clients.
- A refusal is a tool result with `isError: true` and the route's own reason, from its
  400, 404, 409 or 413. JSON-RPC errors are kept for protocol faults.

**Annotations.** Read-only tools carry `readOnlyHint: true`. Each moving tool's
description says that it moves a real telescope. Annotations are hints that a client may
ignore, so no guard depends on them.

**`set_session`:**
- It takes only `site`, `min_alt_deg` and `target`. The scope node's address and token are
  refused: a client that could change them could point the console at a node of its own,
  or break its pairing. The token is never read back in any case.
- While a mount is connected, it refuses a new site. The Sun guard reckons the Sun from
  the site and the console's clock. A site moved by a few hours of longitude (75° E for
  75° W) would make the Sun look set at noon, and the syncs already made would no longer
  fit. The clock can't be set outside tests and `--simulate`.

**`stop_all`** goes through the console's Stop rather than straight to the link, so a GoTo
or tracking in progress ends too. A bare stop all on the link would be undone by the
GoTo's next correction.

### `GET /api/mcp`

The route is read-only:

```text
{ enabled: bool, local_only: bool, urls: ["http://localhost:8080/mcp", ...],
  allowed_hosts: ["surface.local", ...] }
```

**`urls`.** Under a loopback listen address, `urls` is only `http://localhost:<port>/mcp`,
and `local_only` is true. Under `0.0.0.0` or `::`, it is `localhost` plus one URL for each
non-loopback interface address, with IPv4 and link-local addresses first. Global IPv6
addresses are left out, because the door refuses them. The interface list is read through
a small trait, so tests inject one. With `--no-mcp`, `enabled` is false.

The route is the one interface this trio adds beyond the tool table (owner approval 14).

### The session's `motion`

The session gains `motion: {by: "page" | "mcp", tool, since}`:
- It is set when a GoTo, a direct axis move or tracking starts, and cleared when that
  motion ends.
- It is an optional field in the session file, so older files load as having none.
- `tool` is the page's action name (`goto`, `track`) or the MCP tool's name.

## Shapes that existing tests pin

These are not changed by any story:
- **Rust signatures:** `AppState::new`, `AppState::shared` and `router`. MCP is added with
  a builder, `AppState::with_mcp(McpConfig)`. MCP is on by default, with no allowed host
  names.
- **Existing routes:** `POST /api/stop`'s exemption from the same-origin check, and every
  `/api/*` answer. Tools read those answers and never reshape them.
- **The session:** existing session file fields and the session JSON, which `motion` only
  adds to.
- **The page:** the page's existing components and tests. The note is a new component
  placed inside GuidanceBlock and TopBar, and the Setup section is a new component inside
  SetupPage.
- **Playwright:** `playwright.config.ts`, its servers and its existing projects. The new
  `e2e/mcp.spec.ts` runs in the `simulate` project that 013 created, against its
  `finder --simulate` server and fresh `--state`. If that project's file match doesn't
  already pick the file up, US7 adds this one file to the match and nothing else (owner
  approval 15). Any other config change escalates.

## Decisions and the alternatives rejected

### Where MCP lives

**One endpoint, on the console's own port.**
- *Rejected: a second port for MCP.* Under WSL2 every port needs its own portproxy or
  Hyper-V firewall rule, and two ports double what can go wrong at the telescope.
- *Rejected: stdio.* The phone can't launch a process in WSL.

**Reaching it under WSL2, as the page is reached.** `/mcp` is served wherever the page is,
so it reaches the LAN only when the page does:
- The default `--listen 127.0.0.1:8080` keeps MCP on the Surface, and
  `--listen 0.0.0.0:8080` opens both. MCP adds no flag that widens the listen address.
- On the Surface, Claude Code in WSL and Claude Desktop on Windows reach
  `http://localhost:<port>/mcp`. NAT mode forwards Windows' localhost to WSL, and mirrored
  mode shares it.
- The phone reaches `http://<surface>:<port>/mcp` through whatever the owner set up for
  the page: mirrored mode with a Hyper-V firewall rule, or `netsh interface portproxy`
  with a Windows firewall rule under NAT. Either keeps the client's own `Host`.
- Under NAT, the WSL address the console sees is not the Surface's LAN address. Setup says
  so and names the README's section.

### Who may call it

**Open on the LAN, with `Origin`, `Host` and the peer checked.** The MCP specification
says servers MUST validate `Origin`, SHOULD bind to localhost and SHOULD authenticate. The
console keeps the MUST and binds to localhost by default. By the owner's decision it asks
for no token.
- *Rejected: a token, as the scope link has.* The owner rejected it, because clients on
  the house LAN are trusted. A token can be added later without changing the tools.

**Stop is exempt, and carried out by the door.** A stop must never be refused. Carrying it
out in the console's own layer means nothing in the SDK can stand in front of it: not its
`Accept` or protocol-version checks, not a missing `initialize`, not a future default.

### What the tools do

**Passthrough means the same functions.** Each tool calls the function behind its route,
so the answers can't drift from the page's.

**No confirmation.** A moving tool starts the motion before it answers. The page shows the
note, and Stop stays on every page. The MCP specification's advice to keep a human in the
loop is left to the client, which may ask its own user.

**Safety is not passed through.** Per ADR-0001, the console owns pointing, and the Sun
guard lives in the console. The scope node only drives axes, and can't tell where they
point on the sky. A raw mount call passed straight to the link would skip the Sun guard,
and a 16-inch aperture pointed at the Sun is a fire and eye hazard. So every moving tool
runs the same guards as the page's actions:
- the Sun guard on motion: no motion with the Sun's centre above −0.83°, and no path
  within 15° of the Sun;
- the minimum altitude, never below 0°, at the end of the move and along its path;
- 014's path checks: the one-turn cable rule (azimuth kept within one turn either way of
  its position at the night's first sync), and any range limit the mount adapter reports;
- Stop wins, on the console and on the node, and the node's 1 s watchdog;
- tracking's own ends: sunrise, and the target sinking below the minimum altitude.

*Rejected: passing the raw calls through and trusting the node's watchdog.* The watchdog
catches a silent console, not a bad direction.

**Direct axis moves (`mcp::guard`).** The page has none, so these get a guard of their
own, built from the same pieces:
- **A sync is needed.** Without the pointing model, the console can't turn axis positions
  into a sky direction, so it can't check the Sun. With no sync, both tools are refused.
- **`goto_axis_position`** turns the requested positions into a direction through the
  model. It plans the path as 014's planner does (one axis at a time, in the order that
  keeps clear), and refuses it when no path keeps clear.
- **`move_axis`** takes a signed rate, capped at the axis's top rate, and a duration of at
  most 30 s:
  - The console forecasts the path for that duration, including the distance the axis
    needs to come to rest, and refuses the move if the forecast crosses a limit.
  - While the move runs, the console re-forecasts on its own clock at least every 0.1 s.
    It re-forecasts because the Sun rises, or the minimum altitude is raised, and it
    stops the axis in time to be at rest before any limit.
  - It stops the axis at the end of the duration.
- **A telescope parked low may climb.** The minimum altitude along a direct move's path is
  judged against the lower of the move's starting altitude and the minimum altitude. A
  telescope parked low can climb, as a GoTo from low down can, but no move takes it lower
  once it is below the minimum altitude.

**A rate limit on moves (`mcp::limit`).** The tools page says servers MUST rate limit tool
invocations:
- Moving tools share one limit of 2 calls a second across all clients: `goto_target`,
  `goto_axis_position`, `move_axis` and `set_tracking`.
- A call over the limit is refused with `isError: true`, moves nothing, and does not count
  against later windows.
- Reads, `stop` and `stop_all` are never limited.

This stops a looping client from flooding the scope link's queue, and leaves Stop
untouched.

### Who moved it

The session's `motion` shows the source. The page's note reads "Moved by an MCP client ·
<tool>" while `by` is `mcp`, beside the Stop that is always there. Every moving or
stopping MCP call is also written to stderr with its tool, arguments and result. The
scope node's token is never in a log line or a result.

### How it is tested

**Tests go through the router.** The Rust tests need no MCP client crate and no socket.
Each read tool is compared exactly with its route's answer for the same input. Each moving
tool runs against the simulator in virtual time.

## User Story 1 — The endpoint and its door

**Dependency.** Add `rmcp = { version = "=3.5.1", default-features = false, features =
["server", "macros", "transport-streamable-http-server"] }` to `crates/finder/Cargo.toml`,
and settle any `cargo deny` duplicate warning without editing `deny.toml`.

**The service.** `mcp::mod` builds the service with the settings in § `rmcp` settings.
`api.rs` mounts it at `/mcp` behind `mcp::door` unless MCP is off. With MCP off, `/mcp`
falls through to the router's own not-found answer.

**Flags and the first tools.**
- `main.rs` gains `--no-mcp` and `--mcp-allow-host` (repeatable), builds
  `AppState::with_mcp(..)`, and serves with connect info.
- `GET /api/mcp` is added in `api.rs`.
- `get_session` and `set_target` are the first two tools, in `mcp::tools`.

## User Story 2 — Session and catalog tools

`set_session`, `search_targets` and `get_guidance` go in `mcp::tools`, each calling its
route's function. `set_session` rejects any field but `site`, `min_alt_deg` and `target`
with the route's own 400 reason. That includes the scope node's address and token. It
also refuses a new site when the session says a mount is connected.

## User Story 3 — Frames and the link

**`solve_frame`.** It decodes base64 into bytes, refuses a decoded image over
`POST /api/solve`'s 32 MiB limit with the route's own 413 reason, and otherwise calls the
route's solve function, so the latest pointing is set exactly as the page sets it. The
test synthesises its 32 MiB images at run time.

**`get_pointing` and `get_link`.** `get_pointing` reads the session's `pointing`, which is
live while 013's loop runs. `get_link` reads the link's health and the live loop's state
from the session.

## User Story 4 — GoTo and Stop over MCP, and who moved it

**Reads and moves.** `describe_axes` and `get_axis_state` go over the scope link's mount
calls. `goto_target` calls 014's GoTo start, which records `motion.by = mcp` and
`motion.tool = goto_target`. The page's `POST /api/goto` records `by = page`.

**`motion`.** Add the optional field to `session.rs`, and clear it wherever 014's GoTo
ends.

**Stop.** `stop` and `stop_all` call the Stop behind `POST /api/stop`. `mcp::door`
recognises and carries out stops before `rmcp`, as § The door says, including for batches
and for requests that fail a check.

**The log and the token.** Log moving and stopping calls through a writer that
`AppState` holds: stderr by default, and a shared buffer in tests, set with
`AppState::with_call_log(..)`. One line per call names the tool, its arguments and its
result, and never the scope node's token. Add the token-never-leaves test.

## User Story 5 — Direct axis moves and the rate limit

**`goto_axis_position` and `move_axis`.** Add them, with `mcp::guard`, which works through
four steps:
1. it requires a sync;
2. it turns axis positions into directions through 014's model;
3. it forecasts paths at 0.1 s steps, including stopping distance, against the Sun guard,
   the minimum-altitude rule (§ Decisions) and the cable rule;
4. it runs a re-forecasting watcher while a rate move runs, which stops at the duration's
   end.

**The limiter.** Add `mcp::limit` and apply it to `goto_target` and the two direct moves.
The axis move records `motion.by = mcp`.

## User Story 6 — Tracking over MCP

**`set_tracking`.** It calls the function behind `POST /api/track`, and joins the move
limiter. 015's tracking start records `motion.by` as `mcp` or `page`.

**`get_session`.** It already includes tracking's readout, because it passes the session
through.

## User Story 7 — The page and the README

**The note.** `McpNote` renders "Moved by an MCP client · <tool>" from `useSession()`'s
`motion` while `by` is `mcp`. It is placed in GuidanceBlock and TopBar beside the existing
Stop, which stays enabled.

**Setup's MCP section.** `McpSection` reads `GET /api/mcp` and lists the URLs, the allowed
host names and the off or local-only notes, including the WSL NAT note naming the
README's section.

**Records, tests and the build.**
- The README gains its MCP section; `DESIGN.md` and `.impeccable/design.json` record both
  views as built.
- `e2e/mcp.spec.ts` covers the GoTo flow and the night pixel check of both views.
- `web/dist` is rebuilt.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml`):
  - `lint` = `cargo fmt --all --check` and
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `test` = `cargo test --workspace --all-targets --all-features --locked` and
    `cargo test --workspace --locked --doc`.
- **Rust stories** (US1–US6) also run
  `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps --all-features`. US1 also runs
  `cargo deny check` and a build on Rust 1.96 when they are available locally. CI's
  `cargo-deny` and `msrv` jobs enforce both.
- **US7**, in `crates/finder/web`: `npm ci`, `npm run typecheck`,
  `npm run format:check`, `npm test`, `npm run build` (with `dist/` committed) and
  `npm run e2e`.
- **CI in the merge queue** runs all of the above and more: coverage at 80%, the cedar
  e2e eval and canary, msrv, docs, deny, machete and the secret scan.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** `012-targets-and-push-to`, `013-live-finder`,
   `014-goto` and `015-tracking`. `depends_on_landed` enforces this.
2. **The owner approvals are given**, including the `rmcp` dependency, `GET /api/mcp` and
   the one possible Playwright config line (spec § Owner approvals). Flipping to `ready`
   records them.
3. **No operator-staged files are needed.** No story adds a file over 200 KiB.

## Risks

| Risk | Mitigation |
|---|---|
| Anyone on the house Wi-Fi can move the telescope. | It is the owner's call. The guards bound what they can do, Stop is on every page, and the stderr log shows who did what. |
| An MCP client moves the telescope while someone is at the eyepiece. | No confirmation, by decision. The page says an MCP client is moving it, and Stop wins. |
| A client loses its connection mid-move. | The console owns the motion once the tool answers. `move_axis` stops at the end of its duration (at most 30 s), and the node's 1 s watchdog covers a lost console. |
| `rmcp` releases often (five in a month). | The pin is exact, and the endpoint's tests catch protocol changes on a deliberate update. |
| Two clients at once. | One session, and the last write wins. Stop from any client ends every motion. |
