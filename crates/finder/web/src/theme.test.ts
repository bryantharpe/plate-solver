// The two editions' colours, checked from the stylesheet itself: night is red on black with
// nothing brighter than the ink, and day's text meets WCAG AA.

import { describe, expect, it } from "vitest";
import { readWebFile } from "./test/files";

const css = readWebFile("src/index.css");
const html = readWebFile("index.html");

/** The `--color-*` custom properties declared in the first block after `selector`. */
function tokens(selector: string): Map<string, string> {
  const start = css.indexOf(selector);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  const body = css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
  return new Map([...body.matchAll(/(--color-[\w-]+):\s*([^;]+);/g)].map((m) => [m[1] ?? "", (m[2] ?? "").trim()]));
}

/** sRGB channels 0–255 of a `#rrggbb` or `rgb(r g b / a)` colour. */
function rgb(value: string): [number, number, number] {
  const hex = value.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i);
  if (hex) return [1, 2, 3].map((i) => parseInt(hex[i] ?? "", 16)) as [number, number, number];
  const fn = value.match(/^rgb\((\d+) (\d+) (\d+)(?: \/ [\d.]+)?\)$/);
  if (fn) return [1, 2, 3].map((i) => Number(fn[i])) as [number, number, number];
  throw new Error(`not an sRGB colour: ${value}`);
}

/** Relative luminance (WCAG) of an `oklch(L% C H)` colour. */
function luminance(value: string): number {
  const m = value.match(/^oklch\(([\d.]+)% ([\d.]+) ([\d.]+)\)$/);
  if (!m) throw new Error(`not an oklch colour: ${value}`);
  const [L, C, h] = [Number(m[1]) / 100, Number(m[2]), (Number(m[3]) * Math.PI) / 180];
  const [a, b] = [C * Math.cos(h), C * Math.sin(h)];
  // OKLab to linear sRGB (Björn Ottosson's matrices).
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const mm = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const r = 4.0767416621 * l - 3.3077115913 * mm + 0.2309699292 * s;
  const g = -1.2684380046 * l + 2.6097574011 * mm - 0.3413193965 * s;
  const bl = -0.0041960863 * l - 0.7034186147 * mm + 1.707614701 * s;
  const clamp = (v: number) => Math.min(Math.max(v, 0), 1);
  return 0.2126 * clamp(r) + 0.7152 * clamp(g) + 0.0722 * clamp(bl);
}

const contrast = (a: string, b: string) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
};

describe("night edition", () => {
  const day = tokens("@theme {");
  const night = tokens('[data-theme="night"] {');

  it("redefines every colour the page uses", () => {
    for (const name of day.keys()) expect(night.has(name), name).toBe(true);
  });

  it("puts every colour on the red channel alone, none brighter than the ink", () => {
    const [ink] = rgb(night.get("--color-ink") ?? "");
    for (const [name, value] of night) {
      const [r, g, b] = rgb(value);
      expect([g, b], name).toEqual([0, 0]);
      expect(r, name).toBeLessThanOrEqual(ink);
    }
  });

  it("filters photographs into the red channel by luminance", () => {
    const matrix = html.match(/<filter id="red-only"[\s\S]*?values="([^"]+)"/)?.[1];
    const values = (matrix ?? "").trim().split(/\s+/).map(Number);
    expect(values).toHaveLength(20);
    // Rows: red, green, blue, alpha. Green and blue are zero; red takes Rec. 709 luminance.
    expect(values.slice(5, 15).every((v) => v === 0)).toBe(true);
    expect(values.slice(0, 3).reduce((a, b) => a + b, 0)).toBeCloseTo(1, 3);
    expect(css).toMatch(/\[data-theme="night"\] \.photo \{\s*filter: url\(#red-only\);/);
  });
});

describe("day edition", () => {
  const day = tokens("@theme {");
  const color = (name: string) => day.get(name) ?? "";

  it("sets body text and secondary text at WCAG AA on both paper tones", () => {
    for (const text of ["--color-ink", "--color-ink-2"]) {
      for (const ground of ["--color-paper", "--color-paper-2"]) {
        expect(contrast(color(text), color(ground)), `${text} on ${ground}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
});
