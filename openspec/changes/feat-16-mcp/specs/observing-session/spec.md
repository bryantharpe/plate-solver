## ADDED Requirements

### Requirement: The source of motion

The session SHALL hold, while a GoTo, a direct axis move or tracking runs, `motion` with `by`
(`page` or `mcp`), the action or tool that started it, and when it started, and SHALL clear it
when the motion ends. A session file without `motion` SHALL load as having none.
**Refs**: mcp-server "Motion runs at once"; observing-session "Same-origin writes"

#### Scenario: Started over MCP
- **WHEN** `goto_target` is called over MCP on `finder --simulate`
- **THEN** `GET /api/session` reports `motion.by` as `mcp` and `motion.tool` as `goto_target`
  until the GoTo ends, and no `motion` after (oracle: the session, polled every 0.5 s of virtual
  time)

#### Scenario: Started on the page
- **WHEN** GoTo is pressed on the page for the same target
- **THEN** `motion.by` is `page` while it runs (oracle: the session, polled every 0.5 s of
  virtual time)

#### Scenario: An older session file
- **WHEN** the console loads a session file written before this change
- **THEN** it loads without error and reports no `motion`, with every other field equal to the
  saved file's (exact)
