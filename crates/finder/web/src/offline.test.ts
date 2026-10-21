// The console works with no internet: everything it loads comes from `finder` itself. The
// one exception is the survey view, which fetches Aladin Lite from CDS when asked.

import { describe, expect, it } from "vitest";
import { readWebFile } from "./test/files";

const scripts = import.meta.glob<string>(["./**/*.{ts,tsx}", "!./**/*.test.{ts,tsx}", "!./test/**"], {
  query: "?raw",
  import: "default",
  eager: true,
});
// The test runner hands CSS imports over empty, so the stylesheet is read from disk.
const sources: Record<string, string> = { ...scripts, "./index.css": readWebFile("src/index.css") };
const html = readWebFile("index.html");

describe("offline", () => {
  it("references no other host, except the survey view's Aladin Lite", () => {
    const external: string[] = [];
    for (const [path, text] of [...Object.entries(sources), ["index.html", html] as const]) {
      for (const m of text.matchAll(/https?:\/\/[^\s"'`)]+/g)) external.push(`${path}: ${m[0]}`);
    }
    expect(external).toEqual([
      "./components/SurveyView.tsx: https://aladin.cds.unistra.fr/AladinLite/api/v3/3.9.2-beta/aladin.js",
    ]);
  });

  it("serves its own fonts", () => {
    const css = sources["./index.css"] ?? "";
    expect(css).toContain('@import "@fontsource-variable/archivo/wdth.css"');
    expect(css).not.toMatch(/fonts\.googleapis|fonts\.gstatic/);
  });
});
