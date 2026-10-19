import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// `npm run dev` serves the page with hot reload and forwards the API to a running
// `cargo run -p finder` (port 8080). `npm run build` writes dist/, which the Rust binary
// embeds; commit dist/ with the source change. `npm test` runs the page's unit and
// component tests in jsdom; `npm run e2e` drives the real page in Chromium (e2e/).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: { "/api": "http://127.0.0.1:8080" },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test/setup.ts"],
  },
});
