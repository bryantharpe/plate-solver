---
description: "Task list for 016-mcp: The Console's MCP Server"
---

# Tasks: The Console's MCP Server

**Prerequisites** (plan.md § Prerequisites):
- `012-targets-and-push-to`, `013-live-finder`, `014-goto` and `015-tracking` have
  landed on `main`;
- the owner approvals in spec.md are given, including the `rmcp` dependency;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, `scripts/ci/`, `.github/`, `deny.toml` or
  `CODEOWNERS`.
- New Rust tests go in new files under `crates/finder/tests/`, or in-module.
- `crates/math-core`, `proto/`, the scope node and the scope link's proto are never
  touched.
- No test moves real hardware or calls a real LLM. The mount is reached only through the
  simulator, on its virtual clock.

## Phase 1: User Story 1 — The MCP endpoint, and the door that guards it (Priority: P1)

### Tests first

- [ ] T001 [US1] Write failing tests in a new `crates/finder/tests/mcp_endpoint.rs`. Each
  posts JSON-RPC to `/mcp` through `finder::router` with `tower::ServiceExt::oneshot`,
  with peers set through axum's `MockConnectInfo`:
  - `initialize` with no `Authorization`, then `tools/list` with no `Mcp-Session-Id`,
    naming `get_session` (read-only hint) and `set_target` (US1-S1);
  - GET gets 405, with the state file byte-for-byte unchanged (US1-S2);
  - with MCP off, `/mcp` answers as `/no-such-path` does (US1-S3);
  - `MCP-Protocol-Version: 1999-01-01` gets 400 (US1-S4);
  - a notification gets 202 (US1-S5).
- [ ] T002 [US1] Add failing door tests to `crates/finder/tests/mcp_endpoint.rs`:
  - a foreign `Origin` gets 403, with the target unchanged (US1-S6);
  - DNS rebinding with `evil.example:8080` gets 403 (US1-S7);
  - `192.168.1.20:8080`, and `surface.local:8080` under `--mcp-allow-host surface.local`,
    are answered 200 (US1-S8);
  - the peers `127.0.0.1`, `192.168.1.30`, `172.24.0.1`, `fe80::1` and `fd00::30` are
    answered 200 (US1-S9);
  - the peer `203.0.113.7` gets 403 for `get_session` and `set_target` (US1-S10).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/mcp_endpoint.rs`:
  - `get_session` and `set_target` equal their routes' JSON, as `structuredContent` and as
    text (US1-S11);
  - `GET /api/mcp` on three consoles, with an injected interface list (US1-S12).

### Implementation

- [ ] T004 [US1] Add `rmcp = { version = "=3.5.1", default-features = false, features =
  ["server", "macros", "transport-streamable-http-server"] }` to
  `crates/finder/Cargo.toml`, and update `Cargo.lock`. Settle any `cargo deny` duplicate
  warning without editing `deny.toml`. Escalate if it cannot be settled (US1-S13).
- [ ] T005 [US1] Add `pub mod mcp;` to `crates/finder/src/lib.rs`. Create
  `crates/finder/src/mcp/mod.rs` with `McpConfig` (enabled, allowed host names) and the
  `rmcp` service built with the settings in plan.md § `rmcp` settings.
- [ ] T006 [US1] Create `crates/finder/src/mcp/door.rs`: the tower layer with the
  `Origin`, `Host` and peer checks in plan.md's order, answering 403 with a JSON-RPC error
  and no id.
- [ ] T007 [US1] Create `crates/finder/src/mcp/tools.rs` with `get_session` and
  `set_target`. Each calls the function behind its route and returns its JSON as
  `structuredContent` with an `outputSchema` and as text. Refusals are `isError: true`
  with the route's reason.
- [ ] T008 [US1] In `crates/finder/src/api.rs`:
  - add `AppState::with_mcp(McpConfig)`;
  - mount the service at `/mcp` behind the door unless MCP is off;
  - add `GET /api/mcp` with an injectable interface list (plan.md § `GET /api/mcp`).
- [ ] T009 [US1] In `crates/finder/src/main.rs`, add `--no-mcp` and `--mcp-allow-host`
  (repeatable), build `AppState::with_mcp(..)`, and serve with
  `into_make_service_with_connect_info::<SocketAddr>()`. Add no option that widens the
  listen address (US1-S14).
- [ ] T010 [US1] Run the `lint` and `test` gates and
  `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps --all-features`. Confirm
  `tests/api.rs` and `tests/cli.rs` are unchanged. Commit the green story (FR-001, FR-002,
  FR-003, FR-004, FR-005, FR-016, FR-019; US1-S1, US1-S2, US1-S3, US1-S4, US1-S5, US1-S6,
  US1-S7, US1-S8, US1-S9, US1-S10, US1-S11, US1-S12, US1-S13, US1-S14).

## Phase 2: User Story 2 — The session and catalog tools (Priority: P1)

### Tests first

- [ ] T011 [US2] Write failing tests in a new `crates/finder/tests/mcp_session_tools.rs`:
  - `search_targets` for "orion" equals `GET /api/targets?q=orion` (US2-S1);
  - `set_session` with `min_alt_deg` 85, and with `{"sun_guard": false}`, gets the
    route's 400 text as `isError: true`, with the session unchanged (US2-S2);
  - `get_guidance` from 350°/30° to 24°/42° gives clockwise 34° and up 12°, within 1e-9°
    (US2-S3);
  - the scope node's address and token are refused (US2-S4);
  - with the simulated mount connected at 40° N, 75° W, the site 40° N, 75° E is refused
    (US2-S5);
  - each tool equals its route's JSON, the read-only hints are set, and `tools/list`
    names all three (US2-S6).

### Implementation

- [ ] T012 [US2] Add `set_session`, `search_targets` and `get_guidance` to
  `crates/finder/src/mcp/tools.rs`, each calling its route's function.
- [ ] T013 [US2] In `set_session` in `crates/finder/src/mcp/tools.rs`, accept only
  `site`, `min_alt_deg` and `target`, refusing anything else with the route's own 400
  reason, and refuse a new site while the session says a mount is connected.
- [ ] T014 [US2] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Commit the green story (FR-020; US2-S1, US2-S2, US2-S3, US2-S4, US2-S5, US2-S6).

## Phase 3: User Story 3 — Frames and the link (Priority: P1)

### Tests first

- [ ] T015 [US3] Write failing tests in a new `crates/finder/tests/mcp_frames.rs`:
  - `solve_frame` with `orion2.jpg` as base64 equals `POST /api/solve`, and
    `get_pointing` then returns the session's pointing (US3-S1);
  - images of exactly 32 MiB and 32 MiB plus one byte, synthesised at run time and never
    committed: the first answered as the route answers, the second refused with the
    route's 413 reason and the pointing unchanged (US3-S2);
  - on the simulator's live loop, `get_pointing` is within 1′ of the true eyepiece
    direction at mid-exposure (US3-S3);
  - `get_link` 2 s of virtual time after heartbeats stop reads disconnected, as the
    session does (US3-S4);
  - `tools/list` names the three tools, with the read-only hints (US3-S5).

### Implementation

- [ ] T016 [US3] Add `solve_frame` to `crates/finder/src/mcp/tools.rs`: decode base64,
  refuse a decoded image over 32 MiB with `POST /api/solve`'s 413 reason, and otherwise
  call the route's solve function, so the latest pointing is set as the page sets it.
- [ ] T017 [US3] Add `get_pointing` and `get_link` to `crates/finder/src/mcp/tools.rs`,
  reading the session's `pointing`, link health and live loop state.
- [ ] T018 [US3] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm no file over 200 KiB is added. Commit the green story (FR-006, FR-021; US3-S1,
  US3-S2, US3-S3, US3-S4, US3-S5).

## Phase 4: User Story 4 — GoTo and Stop over MCP, and who moved the telescope (Priority: P1)

### Tests first

- [ ] T019 [US4] Write failing tests in a new `crates/finder/tests/mcp_motion.rs`, on the
  simulator in virtual time after a sync:
  - `describe_axes` and `get_axis_state` equal the simulator's (US4-S1);
  - `goto_target` completes within 1′ with no page action, with `motion.by` as `mcp`
    (US4-S2);
  - polled every 0.5 s: `motion` is `mcp` and `goto_target` until the end, then absent
    (US4-S3);
  - the page's `POST /api/goto` gives `motion.by` as `page` (US4-S4);
  - a session file from before this feature loads with no `motion` (US4-S5).
- [ ] T020 [US4] Add failing stop tests to `crates/finder/tests/mcp_motion.rs`:
  - `stop_all` from a second client halts a GoTo within 0.1 s, with no correction for
    5 s (US4-S6);
  - `stop` with `MCP-Protocol-Version: 1999-01-01` gets 400 and still stops (US4-S7);
  - a bare `stop_all`, with no `initialize`, `Accept` or protocol version, stops (US4-S8);
  - `stop` with no mount is a success, and both stops stay listed (US4-S9);
  - `stop_all` with a foreign `Origin` stops (US4-S10);
  - a batch of `stop_all` and `goto_target` with a foreign `Origin` stops and starts no
    GoTo (US4-S11);
  - `goto_target` with `Host: evil.example:8080` gets 403, and no axis moves (US4-S12);
  - `get_session` and `goto_target` from `203.0.113.7` get 403, and no axis moves
    (US4-S13).
- [ ] T021 [US4] Add failing tests to `crates/finder/tests/mcp_motion.rs`:
  - no tool result holds the scope node's test token (US4-S14);
  - the captured call log has one line each for `goto_target` and `stop_all`, naming
    tool, arguments and result (US4-S15).

### Implementation

- [ ] T022 [US4] Add the optional `motion: {by, tool, since}` to
  `crates/finder/src/session.rs`, loading as absent from older files. Set and clear it
  where `crates/finder/src/goto.rs` starts and ends a GoTo, with `by = page` for
  `POST /api/goto`.
- [ ] T023 [US4] Add `describe_axes`, `get_axis_state`, `goto_target` (`by = mcp`),
  `stop` and `stop_all` to `crates/finder/src/mcp/tools.rs`. The stops call the Stop
  behind `POST /api/stop`.
- [ ] T024 [US4] In `crates/finder/src/mcp/door.rs`, recognise a `tools/call` of `stop`
  or `stop_all`, alone or in a batch, and carry it out through the console's Stop before
  the checks answer and before `rmcp` sees the request. When a check fails, run nothing
  else (plan.md § The door).
- [ ] T025 [US4] Add `AppState::with_call_log(..)` in `crates/finder/src/api.rs` (stderr
  by default). Log every moving or stopping call from `crates/finder/src/mcp/tools.rs` and
  `crates/finder/src/mcp/door.rs`, never with the token.
- [ ] T026 [US4] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm 014's existing GoTo tests are unchanged. Commit the green story (FR-007, FR-008,
  FR-012, FR-014, FR-017; US4-S1, US4-S2, US4-S3, US4-S4, US4-S5, US4-S6, US4-S7, US4-S8,
  US4-S9, US4-S10, US4-S11, US4-S12, US4-S13, US4-S14, US4-S15).

## Phase 5: User Story 5 — Direct axis moves, held to the page's guards (Priority: P1)

### Tests first

- [ ] T027 [US5] Write failing guard tests in a new
  `crates/finder/tests/mcp_axis_moves.rs`, on the simulator in virtual time, sampled every
  0.1 s:
  - with the Sun 5° up, all three moving tools are refused and nothing moves (US5-S1);
  - at dusk (Sun −10°), a position 5° from the Sun is refused (US5-S2);
  - top rate toward the Sun for 30 s from 40° away is refused (US5-S3);
  - a position 5° below the minimum altitude is refused (US5-S4);
  - from 3° up with a minimum of 15°, 1°/s up for 20 s is carried out and stops at
    20.0 ± 0.1 s (US5-S5);
  - 1.2 turns from the first sync is refused by the cable rule (US5-S6);
  - with no sync, `move_axis` is refused (US5-S7).
- [ ] T028 [US5] Add failing rate-move and limit tests to
  `crates/finder/tests/mcp_axis_moves.rs`:
  - 0.5°/s for 10 s stops at 10.0 ± 0.1 s, having moved 5.0° less the ramp, within 0.1°
    (US5-S8);
  - a 60 s duration is refused (US5-S9);
  - a 20 s move whose client goes away stops at 20.0 ± 0.1 s, with heartbeats carrying on
    (US5-S10);
  - sunrise during a rate move stops it within 1 s (US5-S11);
  - ten moves within 1 s: at most 2 carried out, and `stop_all` stops (US5-S12);
  - `tools/list` describes both tools as moving a real telescope (US5-S13).

### Implementation

- [ ] T029 [US5] Create `crates/finder/src/mcp/guard.rs` and add `pub mod guard;` to
  `crates/finder/src/mcp/mod.rs`. Require a sync, turn axis positions into directions
  through 014's model, and forecast a path at 0.1 s steps, including stopping distance,
  against the Sun guard, the minimum altitude (the lower of start and minimum) and the
  cable rule.
- [ ] T030 [US5] Create `crates/finder/src/mcp/limit.rs` and add `pub mod limit;` to
  `crates/finder/src/mcp/mod.rs`: one window of 2 moving calls a second, across all
  clients, where a refused call doesn't count.
- [ ] T031 [US5] Add `goto_axis_position` and `move_axis` to
  `crates/finder/src/mcp/tools.rs`:
  - both pass the guard and record `motion` with `by = mcp`;
  - `move_axis` caps the rate at the top rate, refuses more than 30 s, re-forecasts at
    least every 0.1 s on the console's clock while it runs, and stops at the duration's
    end;
  - apply the limiter to them and to `goto_target`.
- [ ] T032 [US5] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Commit the green story (FR-009, FR-010, FR-011; US5-S1, US5-S2, US5-S3, US5-S4, US5-S5,
  US5-S6, US5-S7, US5-S8, US5-S9, US5-S10, US5-S11, US5-S12, US5-S13).

## Phase 6: User Story 6 — Tracking over MCP (Priority: P2)

### Tests first

- [ ] T033 [US6] Write failing tests in a new `crates/finder/tests/mcp_tracking.rs`,
  feeding the console the simulator's true pointing plus noise:
  - `set_tracking` on M42 holds within 1′ over 10 min of virtual time, sampled every 1 s,
    with the session's tracking error equal to the page's readout and `motion` naming
    `set_tracking` (US6-S1);
  - with no mount, it is refused with `POST /api/track`'s reason (US6-S2);
  - it counts against the move limiter with `move_axis` (US6-S3).

### Implementation

- [ ] T034 [US6] Add `set_tracking` to `crates/finder/src/mcp/tools.rs`, calling the
  function behind `POST /api/track` and joining the limiter. Record `motion.by` as `mcp`
  or `page` where `crates/finder/src/tracking.rs` starts Track, and clear it where Track
  ends.
- [ ] T035 [US6] Run the `lint` and `test` gates and `cargo doc -p finder --no-deps`.
  Confirm 015's existing tracking tests are unchanged. Commit the green story (FR-013;
  US6-S1, US6-S2, US6-S3).

## Phase 7: User Story 7 — The page says an MCP client is moving the telescope, and Setup shows the MCP address (Priority: P2)

### Tests first

- [ ] T036 [US7] Write failing Vitest tests:
  - in a new `crates/finder/web/src/components/McpNote.test.tsx`, the note renders only
    while `motion.by` is `mcp`, naming the tool;
  - in a new `crates/finder/web/src/components/McpSection.test.tsx`, a LAN answer lists
    every URL and `surface.local` with the WSL NAT note (US7-S2), and a loopback answer
    lists only `localhost`, with the off state shown as off (US7-S3).
- [ ] T037 [US7] Write `crates/finder/web/e2e/mcp.spec.ts` for the `simulate` project:
  - `goto_target` posted over HTTP shows the note in the guidance block and top bar within
    1 s, and Stop halts the GoTo (US7-S1);
  - the night pixel check on the note and on Setup's MCP section, at 0 pixels (US7-S4).

### Implementation

- [ ] T038 [US7] Add `crates/finder/web/src/components/McpNote.tsx`, and place it in
  `crates/finder/web/src/components/GuidanceBlock.tsx` and
  `crates/finder/web/src/components/TopBar.tsx` beside the existing Stop.
- [ ] T039 [US7] Add `crates/finder/web/src/components/McpSection.tsx`, reading
  `GET /api/mcp`, and place it in `crates/finder/web/src/components/SetupPage.tsx`.
- [ ] T040 [US7] Only if the `simulate` project doesn't already match
  `e2e/mcp.spec.ts`, add that one file to its match in
  `crates/finder/web/playwright.config.ts` and change nothing else (owner approval 15).
- [ ] T041 [US7] Add the MCP section to `crates/finder/README.md` (US7-S5). Record the
  note and the Setup section as built in `DESIGN.md` and `.impeccable/design.json`
  (US7-S6).
- [ ] T042 [US7] In `crates/finder/web`, run `npm ci`, `npm run typecheck`,
  `npm run format:check`, `npm test`, `npm run build` and `npm run e2e`, with every
  existing Vitest and Playwright test unchanged and green. Commit the rebuilt
  `crates/finder/web/dist`. Confirm `package.json` dependencies are unchanged. Run the
  `lint` and `test` gates. Commit the green story (FR-015, FR-018; US7-S1, US7-S2, US7-S3,
  US7-S4, US7-S5, US7-S6).

## Execution Order

The stories land in this order:

```text
US1 → US2 → US3 → US4 ─┬→ US5 → US6
                       └→ US7
```

Every arrow is a merge edge: the later story starts from a `main` that holds the earlier
one.
- **US2 to US6** each add tools to `crates/finder/src/mcp/tools.rs`, so they run one after
  another.
- **US4** also changes `mcp/door.rs` and `api.rs` from US1. **US5** adds modules to
  `mcp/mod.rs`.
- **US7** may run beside US5 and US6, because it touches only `crates/finder/web/`,
  `crates/finder/README.md`, `DESIGN.md` and `.impeccable/design.json`.
- **`[P]`** marks nothing here: every task within a story shares a file with another.
- **Out of scope for every story:** `crates/math-core`, `proto/`, the scope node, the
  scope link's proto, `scripts/ci/`, `.github/`, `deny.toml` and existing tests.
