import { defineConfig } from "@playwright/test";

// End to end: the real `finder` binary, its database and sample frames, serving the built
// page (dist/), driven in Chromium. Set FINDER_URL to test a server that is already
// running; otherwise a release build is started here (the first run compiles it).
const url = process.env.FINDER_URL ?? "http://127.0.0.1:8097";

export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: { baseURL: url, browserName: "chromium" },
  webServer: process.env.FINDER_URL
    ? undefined
    : {
        command: "cargo run --release --locked -p finder -- --listen 127.0.0.1:8097",
        cwd: "../../..",
        url: `${url}/api/health`,
        timeout: 20 * 60_000,
        reuseExistingServer: !process.env.CI,
      },
});
