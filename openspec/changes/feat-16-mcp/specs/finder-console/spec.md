## ADDED Requirements

### Requirement: Motion by an MCP client on the page

While the session's `motion.by` is `mcp`, the guidance block and the top bar SHALL say the
telescope is being moved by an MCP client, naming the tool, beside a Stop that stays enabled.
Setup SHALL show an MCP section with the endpoint's URLs as the console sees them and the
allowed host names.
**Refs**: observing-session "The source of motion"; finder-console "GoTo and Stop on the console"; DESIGN.md; docs/research/2026-10-25-mcp-server.md §6

#### Scenario: An AI's GoTo on the page
- **WHEN** `goto_target` is called over MCP on `finder --simulate` with the page open
- **THEN** within 1 s the page reads "Moved by an MCP client · goto_target" with Stop enabled,
  and pressing Stop halts the GoTo (oracle: Playwright in the `simulate` project; the
  simulator's axis state, exact)

#### Scenario: Setup's MCP section
- **WHEN** Setup is opened on a console started with `--mcp-allow-host surface.local`
- **THEN** it lists `http://localhost:<port>/mcp`, an address for each non-loopback interface,
  and `surface.local`, and says that under WSL's NAT the phone needs the README's portproxy or
  mirrored-mode setup (oracle: the host's interface list, exact)

#### Scenario: Night mode
- **WHEN** the MCP note and Setup's MCP section are shown in night mode
- **THEN** they pass the night pixel check: no pixel outside the red-only palette (tolerance 0
  pixels)
