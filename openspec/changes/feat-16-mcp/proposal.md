## Why

The owner wants an AI (Claude on the Surface, or an app on the phone) to steer the telescope:
find a target, solve a frame, point the scope and track it. MCP is how those clients call tools.
This change gives the console an MCP server that passes its own actions and the scope link's
mount calls through to MCP clients on the house LAN, while every move still goes through the
console's guards, so an AI can never point the 16-inch aperture at the Sun.

## What Changes

- **An MCP endpoint** in the console: Streamable HTTP at `/mcp`, on the console's own address and
  port, so whatever lets the phone reach the page under WSL2 (portproxy or mirrored mode) also
  carries MCP. No token: it is open on the LAN, by the owner's decision. It validates `Origin`, as
  the MCP specification requires, and `Host`, against DNS rebinding.
- **Tools that pass the console's actions through**, landing with the change that made each
  action:
  - after feat-12: the session, search targets, the target's guidance (pointability and push-to
    directions), set the target, solve a frame, the latest pointing;
  - after feat-13: the scope link's health and the live pointing;
  - after feat-14: the mount calls (describe the axes, axis state, go to an axis position, move
    at a rate, stop all), and GoTo and Stop;
  - after feat-15: Track on and off, with its readout.
- **No confirmation.** A GoTo or axis move from an MCP client runs at once.
- **Safety is not passed through.** Every tool that can move the telescope goes through the
  page's guards: the Sun guard, the minimum altitude, the horizon, feat-14's motion and path
  checks, the cable rule, Stop wins and the watchdog. A rate move has a duration and stops at its
  end. Stop and stop all are always listed and never refused.
- The session records whether the current motion came from the page or an MCP client; the page
  says so, and Setup shows the MCP address.
- **No MCP on the scope node.** The Pi is controlled only over the existing `scope_node.v1` link,
  and the console stays the only holder of its token.

## Capabilities

### New Capabilities

- `mcp-server`: the console's MCP endpoint, its tools, and the guards on every tool that moves
  the telescope.

### Modified Capabilities

All introduced earlier in this set; each delta only ADDs:

- `observing-session`: the source of the current motion (the page or an MCP client).
- `finder-console`: the page's note when an MCP client moves the telescope, and Setup's MCP
  section.

## Impact

- **Code:** `finder::mcp` (the endpoint and tools, each a thin call into the same functions the
  HTTP routes use), a `/mcp` route in the console's router, the `--mcp-allow-host` and `--no-mcp`
  flags, and a field in the session. No change to the scope node, the proto or the link.
- **Dependencies:** `rmcp` (pinned, `server`, `macros` and `transport-streamable-http-server`, no
  default features). It builds on Rust 1.96 and its tree passes the licence allow-list
  (`docs/research/2026-10-25-mcp-server.md` §5); `cargo deny check` confirms it when it lands.
- **Security:** anyone on the house Wi-Fi can move the telescope through MCP, within the guards.
  That is the owner's decision; it is listed under Decisions for the owner in design.md, with
  the `Host` check and the rate-move duration as recommended defaults.
- **Tests:** Rust tests post JSON-RPC to `/mcp` through the router and check each tool against
  the HTTP route it passes through, and every motion tool against the simulator's truth in
  virtual time. Playwright adds an MCP-moved flow to the `simulate` project. No existing test,
  golden or CI file changes.
- **Out of scope:** MCP on the scope node; MCP resources and prompts; server-initiated messages
  (SSE); OAuth; the mobile app and SkySafari.
