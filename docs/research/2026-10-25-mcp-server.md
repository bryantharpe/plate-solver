# MCP: a Streamable HTTP server in the console, and the Rust SDK

Research notes gathered on 2026-10-25 for feat-16 (MCP). They cover the parts of the Model Context Protocol the console needs, how the specification says to secure an HTTP server, and whether the official Rust SDK fits this repository's toolchain and licence gate. Sources are linked inline, with the dates found.

**Outcome:** the console serves MCP over Streamable HTTP at `/mcp` on its own address and port, with no token, validating `Origin` (and `Host`, against DNS rebinding). The scope node gets no MCP: the console stays the only holder of its token, per [ADR-0001](../adr/0001-console-owns-pointing-scope-node-drives-axes.md). The official SDK, `rmcp`, declares Rust 1.88, builds on Rust 1.96, and it and every crate it pulls in are under licences `deny.toml` allows; `cargo deny check` against the console's whole tree is feat-16's first task.

## 1. Versions of the specification

- Dated revisions are published at modelcontextprotocol.io. The ones that matter here: [2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports), 2025-11-25, and the newest, 2026-07-28 (found 2026-10-25).
- **2025-06-18 and 2025-11-25** have protocol-level sessions over HTTP: a server MAY assign an `Mcp-Session-Id` at initialisation, and the client then sends it on every request ([transports §Session Management](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)).
- **2026-07-28** is stateless: "MCP is stateless and has no protocol-level sessions"; a server that needs state across requests mints its own handle and takes it back as a tool argument ([security best practices, State Handle Hijacking](https://modelcontextprotocol.io/specification/latest/basic/security_best_practices), 2026-10-25).
- The console needs no per-client state: everything lives in the one observing session. So it can run without session IDs, which both families of versions allow (in the older ones the session ID is a MAY).
- Clients send `MCP-Protocol-Version` on every HTTP request after initialisation; a server MUST answer 400 to an invalid or unsupported one ([transports §Protocol Version Header](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)).

## 2. Streamable HTTP

From [transports §Streamable HTTP](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports) (2026-10-25):
- One endpoint path (the MCP endpoint) takes POST and GET.
- Every JSON-RPC message from the client is a new POST. Its `Accept` lists both `application/json` and `text/event-stream`.
- For a request, the server answers either one JSON object (`application/json`) or an SSE stream; the client MUST handle both. For a notification or response, it answers 202 with no body.
- GET opens an optional server-to-client SSE stream; a server that offers none answers 405.
- The older HTTP+SSE transport (2024-11-05) is deprecated; servers MAY keep it for old clients.

**For the console:** plain JSON answers and 405 on GET. Nothing the console does needs server-initiated messages; a GoTo's progress is read with a tool call, like the page polls the session.

## 3. Security guidance for an HTTP server

The transport's own warning ([transports §Security Warning](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)), verbatim:

> 1. Servers **MUST** validate the `Origin` header on all incoming connections to prevent DNS rebinding attacks
> 2. When running locally, servers **SHOULD** bind only to localhost (127.0.0.1) rather than all network interfaces (0.0.0.0)
> 3. Servers **SHOULD** implement proper authentication for all connections

The [security best practices](https://modelcontextprotocol.io/specification/latest/basic/security_best_practices) (2026-10-25) add, for local servers on HTTP: restrict access, for instance with an authorisation token or a Unix socket; and never treat possession of a session ID or state handle as authentication.

**How that maps onto the owner's decision (open on the LAN, 2026-10-25):**
- **Origin (MUST).** Kept. It stops a web page on another site from driving the telescope through a browser on the LAN; real MCP clients (Claude Code, Claude Desktop, a phone app) send no `Origin` or their own.
- **DNS rebinding.** A rebinding page is served from the attacker's name, so its requests carry `Origin: http://evil.example:8080` *and* `Host: evil.example:8080`. feat-12's rule (`Origin` must match `Host`) lets that through. The console therefore also checks that the host in `Host` (and in any `Origin`) is an IP literal, `localhost`, or a name the owner listed. A browser on the LAN that reaches the console by its IP address or `.local` name still works.
- **Bind to localhost (SHOULD).** Deliberately not followed: the phone must reach the console over the LAN. The console already binds where the page does; the MCP endpoint adds no port.
- **Authentication (SHOULD).** Deliberately not followed, by the owner's decision: MCP clients on the house LAN need no token. The scope node's token stays with the console and is never given to an MCP client (that would be the "token passthrough" anti-pattern the best practices forbid).

## 4. Tools

From the [tools page](https://modelcontextprotocol.io/specification/2025-06-18/server/tools) of the specification:
- A server declares the `tools` capability; clients call `tools/list` and `tools/call`.
- Each tool has a `name`, a `description`, an `inputSchema` (JSON Schema) and, since 2025-06-18, an optional `outputSchema` with `structuredContent` in the result. Servers that return structured content should also return it as text for older clients.
- **Errors.** A failure inside a tool (a refusal, a bad argument the model can fix) is a result with `isError: true` and a message, so the model sees it. JSON-RPC errors are for protocol problems such as an unknown tool.
- **Annotations** are hints, not guarantees: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. Clients must not rely on them for safety, so the console's guards cannot depend on a client honouring them.
- **Human in the loop.** "For trust & safety and security, there **SHOULD** always be a human in the loop with the ability to deny tool invocations"; applications SHOULD show which tools are exposed, show when they are invoked, and present confirmation prompts. The owner has decided against a confirmation on the page (2026-10-25); a client may still ask its own user, and the page shows when an MCP client is moving the telescope.
- **Server duties** (§Security Considerations): servers MUST validate all tool inputs, implement proper access controls, rate limit tool invocations and sanitise tool outputs. Clients SHOULD time out tool calls and log tool usage.
- Annotations: "clients **MUST** consider tool annotations to be untrusted unless they come from trusted servers." (All checked against the [2025-06-18 tools page](https://modelcontextprotocol.io/specification/2025-06-18/server/tools), 2026-10-25.)

## 5. The Rust SDK: `rmcp`

- Official SDK, [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk), published as [`rmcp`](https://crates.io/crates/rmcp) with `rmcp-macros`.
- **Versions** (crates.io, 2026-10-25): 3.7.0 (2026-10-22), 3.6.0 (2026-10-20), 3.5.1 (2026-10-05), 3.5.0 (2026-09-28), 3.4.1 (2026-09-23). Releases are frequent; pin an exact version at least two weeks old (3.5.1 today) and move deliberately.
- **MSRV:** `rust_version` 1.88 on crates.io, below this repository's 1.96.
- **Features for a server over Streamable HTTP:** `server`, `macros` (the `#[tool]` attributes) and `transport-streamable-http-server` ([crates/rmcp/Cargo.toml](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/Cargo.toml), 2026-10-25). The server side is a `tower` service, so it mounts as a route in the console's existing axum router; axum itself is only a dev-dependency of `rmcp`.
- **What those features pull in:** tokio, tokio-util, tokio-stream, serde, serde_json, schemars 1 (with chrono), uuid, pastey, async-trait, sse-stream, tower-service, http, http-body, http-body-util, bytes, rand 0.10 and base64.

**Licences against `deny.toml`'s allow-list** (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0, BSL-1.0, CC0-1.0):
- `rmcp` and `rmcp-macros`: Apache-2.0. Allowed.
- tokio, tokio-util, tokio-stream, tower-service, bytes, http-body-util: MIT. Allowed.
- serde, serde_json, uuid, pastey, async-trait, http, http-body, rand, base64, chrono: MIT or Apache-2.0. Allowed.
- schemars: MIT. Allowed.
- **Checked on 2026-10-25:** a scratch crate depending on `rmcp = "=3.5.1"` with those three features (no default features) passed `cargo +1.96 check` on aarch64-unknown-linux-gnu in 10 s. `cargo metadata` over its tree (91 packages, all targets) gave only: MIT OR Apache-2.0 (and spellings of it), MIT, Apache-2.0, one `(MIT OR Apache-2.0) AND Unicode-3.0`, one `Unlicense OR MIT` and one `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. Each has an allowed choice, so none fails the licence gate. sse-stream 0.2.6 and schemars 1.2.3 resolved.
- Several of these are already in the console's tree (tokio, serde, http, bytes); `cargo deny`'s duplicate-version check may flag `rand` or `base64` if their major versions differ from what is already locked. That is a `[bans]` warning to settle when the dependency lands, not a licence problem.

## 6. Reaching the console from the LAN under WSL2

From the [runtime note §5](2026-10-21-scope-node-runtime.md#5-networking) and feat-12's same-origin design:
- Under WSL's default NAT, LAN devices reach the console only through `netsh interface portproxy` and a firewall rule; mirrored mode shares Windows' interfaces and needs a Hyper-V firewall rule for the console's port. The finder README documents mirrored mode for the phone.
- Because the MCP endpoint is a path on the console's existing port, whichever of the two the owner set up for the page also carries MCP, with no new rule. Clients on the Surface itself reach it at `localhost` (WSL forwards Windows' localhost to WSL in NAT mode, and mirrored mode shares it).
- A client through portproxy or mirrored mode keeps its own `Host` (the Surface's IP or name), so the `Host` check in §3 passes as long as that name is an IP literal or listed.
