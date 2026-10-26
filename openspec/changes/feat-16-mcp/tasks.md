## 1. Endpoint and checks (after feat-12)

- [ ] 1.1 Add `rmcp` pinned to an exact version at least two weeks old, with `default-features = false` and `server`, `macros` and `transport-streamable-http-server`; verified by `cargo deny check`, the msrv build on 1.96, and clippy `--all-features` on x86_64
- [ ] 1.2 Mount `/mcp` in the console's router, JSON answers only, 405 on GET, 400 on an unsupported `MCP-Protocol-Version`, no session ID needed, and `--no-mcp`, with `rmcp`'s `legacy_session_mode` off, `json_response` on, its own host and origin lists empty and a 44 MiB body limit (design.md); verified by the "Initialising", "No stream" and "Turned off" tests through `tower::ServiceExt::oneshot`
- [ ] 1.3 Add the `Origin`, `Host` and peer-address checks with `--mcp-allow-host`, as a layer in front of `rmcp`, letting only `stop` and `stop_all` through from a request that fails them; verified by the "A page on another site", "DNS rebinding", "Clients on the LAN", "From the internet" and "From the house LAN" tests now (peer addresses set with axum's `MockConnectInfo`), and "Stop from anywhere" and "A move riding on a stop" once 4.4 lands
- [ ] 1.4 Return route JSON as `structuredContent` and text, refusals as `isError: true` with the route's reason, and `readOnlyHint` on read tools; verified by the "A refusal reads the same" test

## 2. Session and catalog tools (after feat-12)

- [ ] 2.1 Add `get_session`, `set_session` (site, minimum altitude and target only), `set_target` and `search_targets`, each calling the route's function; verified by the "Searching" and "The scope node stays put" tests (the latter once feat-13's scope node fields exist) and a test comparing each tool with its route for the same input
- [ ] 2.2 Add `get_guidance`; verified by the "Push-to over MCP" test against push-to's worked example
- [ ] 2.3 Add `solve_frame` and `get_pointing`, with the `/api/solve` size limit; verified by the "Solving a saved frame" test on `orion2.jpg` and the "At the limit" test at 32 MiB and one byte over

## 3. Live tools (after feat-13)

- [ ] 3.1 Add `get_link`, and have `get_pointing` return live solves; verified by the "Live pointing" and "Link lost" tests against the simulator in virtual time

## 4. Mount tools and guards (after feat-14)

- [ ] 4.1 Add `describe_axes` and `get_axis_state` over the scope link; verified by the "Axis state" test
- [ ] 4.2 Add the session's `motion` field, set by the page and by MCP, and cleared at the end; verified by the "Started over MCP", "Started on the page" and "An older session file" tests
- [ ] 4.3 Add `goto_target`, starting at once and recording `motion.by = mcp`; verified by the "GoTo from an MCP client" test against the simulator's true eyepiece direction
- [ ] 4.4 Add `stop` and `stop_all` through the console's Stop; verified by the "Stop during an MCP client's GoTo", "Stop with a bad protocol version", "Stop with nothing moving", "Stop from anywhere" and "A move riding on a stop" tests
- [ ] 4.5 Add the direct-move guard (needs a sync; the model, the planner, the Sun guard, the minimum altitude judged against the lower of the start and the minimum, and the cable rule) and `goto_axis_position`; verified by the "Daylight", "Pointed at the Sun by axis position", "Below the minimum altitude", "Past the cable" and "No sync yet" tests
- [ ] 4.6 Add `move_axis` with the 30 s cap, the forecast (stopping distance included) that refuses a move crossing a limit, the re-forecast stop while it runs and the end-of-duration stop; verified by the "A rate toward the Sun", "Climbing from low down", "The duration ends", "Too long", "The client goes away" and "Sunrise during a rate move" tests, sampling the simulator every 0.1 s of virtual time
- [ ] 4.7 Log every moving or stopping MCP call to stderr with tool, arguments and result; verified by a test that captures the log for one `goto_target` and one `stop_all`
- [ ] 4.8 Check that the scope node's token never leaves the console; verified by the "The token stays home" test, with a low-entropy test token
- [ ] 4.9 Add the shared rate limit on moving tools, leaving reads and Stop unlimited; verified by the "A burst of moves" test against the simulator's command log
- [ ] 4.10 Have `set_session` refuse a new site while a mount is connected; verified by the "The site stays put under a mount" test on the simulated mount

## 5. Tracking tools (after feat-15)

- [ ] 5.1 Add `set_tracking` and tracking's readout in `get_session`; verified by the "Tracking from an MCP client" test (truth plus noise in virtual time, as feat-15's long runs) and "Track refused"

## 6. Page, docs and check

- [ ] 6.1 Show "Moved by an MCP client" in the guidance block and top bar, and Setup's MCP section; verified by the "An MCP client's GoTo on the page", "Setup's MCP section" and "Only this machine" flows added to the Playwright `simulate` project
- [ ] 6.2 Add the new views to the night pixel check; verified by the "Night mode" scenario in the existing night check's run, with no existing test or golden edited
- [ ] 6.3 Document the MCP endpoint in the finder README (the URLs, `--mcp-allow-host`, `--no-mcp`, that it reaches the LAN only with `--listen 0.0.0.0:8080`, that portproxy or mirrored mode carries it under WSL2, and that the Windows firewall rule should be scoped to the local subnet because portproxy hides the peer address); verified by review
- [ ] 6.4 Run an Impeccable pass on the new views and update `DESIGN.md`; verified by the pass's notes in the PR
- [ ] 6.5 Run `openspec validate feat-16-mcp --strict` and confirm `openspec status --change feat-16-mcp` reaches 4/4; verified by their output
