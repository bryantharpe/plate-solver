import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M31, M8, M75, M78, SESSION, TIME } from "../test/targets";
import { useSession } from "../useSession";
import { GuidanceBlock } from "./GuidanceBlock";

// Oracles: explicit wire answers at 40° N, 75° W, 2026-01-16 02:00 UT.
// Whole degrees and minute text are exact; the clock and device zone are external.
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
const block = () => screen.getByRole("region", { name: /Target/ });
const noDirections = () => expect(block().textContent).not.toMatch(/turn |tilt /);

function SharedGuidance() {
  const { session } = useSession();
  return <GuidanceBlock session={session} />;
}

describe("the guidance block", () => {
  it("names M78 and its class and reads the rounded turn, tilt and pointing age", () => {
    vi.useFakeTimers();
    vi.setSystemTime(TIME * 1000);
    render(<GuidanceBlock session={SESSION} />);
    expect(block().textContent).toContain("M78");
    expect(block().textContent).toContain("Reflection nebula");
    expect(block().textContent).toContain("Pointable");
    expect(screen.getByText("turn clockwise 34° · tilt up 12°")).toBeTruthy();
    expect(screen.getByText(/Pointing 2 min ago/)).toBeTruthy();
    expect(block().className).toContain("text-ink");
    expect(block().querySelector('[class*="icon-"], [class*="object-"]')).toBeNull();
  });

  it("uses anticlockwise and down for negative directions", () => {
    render(
      <GuidanceBlock session={{ ...SESSION, guidance: { class: "pointable", turn_deg: -33.6, tilt_deg: -12.4 } }} />,
    );
    expect(screen.getByText("turn anticlockwise 34° · tilt down 12°")).toBeTruthy();
  });

  it("prompts for a solved frame when pointable without a pointing", () => {
    render(<GuidanceBlock session={{ ...SESSION, pointing: null, guidance: { class: "pointable" } }} />);
    expect(screen.getByText(/Solve a frame/)).toBeTruthy();
    noDirections();
  });

  it("says M8 is down and renders supplied rise, transit and set in the device zone to the minute", () => {
    // Deliberately non-UTC: catches a hard-coded UTC formatter. The browser's local
    // formatter is supplied here with a fixed America/New_York device zone.
    const native = Date.prototype.toLocaleTimeString;
    vi.spyOn(Date.prototype, "toLocaleTimeString").mockImplementation(function (this: Date, _locale, options) {
      expect(options).toEqual(expect.objectContaining({ hour: "2-digit", minute: "2-digit" }));
      expect(options?.timeZone).toBeUndefined();
      return native.call(this, "en-GB", { ...options, timeZone: "America/New_York" });
    });
    render(
      <GuidanceBlock
        session={{
          ...SESSION,
          target: M8,
          guidance: {
            class: "below_horizon",
            rise: Date.parse("2026-01-16T11:36:42Z") / 1000,
            transit: Date.parse("2026-01-16T15:13:18Z") / 1000,
            set: Date.parse("2026-01-16T18:50:54Z") / 1000,
          },
        }}
      />,
    );
    expect(block().textContent).toMatch(/M8[\s\S]*below the horizon/i);
    const times = [...block().querySelectorAll("time")].map((t) => t.textContent);
    expect(times).toEqual(["06:36", "10:13", "13:50"]);
    for (const label of ["Rises", "Transits", "Sets"]) expect(block().textContent).toContain(label);
    noDirections();
  });

  it("handles too low and a target that never reaches the minimum altitude", () => {
    render(
      <GuidanceBlock
        session={{ ...SESSION, target: M8, guidance: { class: "too_low", never: true, transit: TIME + 3600 } }}
      />,
    );
    expect(block().textContent).toMatch(/below the minimum altitude/i);
    expect(block().textContent).toMatch(/never reaches 15°/i);
    expect(block().querySelectorAll("time")).toHaveLength(1);
    noDirections();
  });

  it("refuses M75 within 15° of the Sun and shows no directions or times", () => {
    render(
      <GuidanceBlock
        session={{ ...SESSION, target: M75, guidance: { class: "sun_guard", turn_deg: 3, tilt_deg: 4, rise: TIME } }}
      />,
    );
    expect(block().textContent).toContain("The console will not point within 15° of the Sun.");
    noDirections();
    expect(block().querySelector("time")).toBeNull();
  });

  it("asks for a site and links to Setup, without directions or times", () => {
    render(
      <GuidanceBlock
        session={{
          ...SESSION,
          site: null,
          target: { ...M78, id: "NGC 1976", messier: "M42" },
          guidance: { class: "no_site" },
        }}
      />,
    );
    expect(block().textContent).toMatch(/site is needed/i);
    expect(screen.getByRole("link", { name: /Set your site/ }).getAttribute("href")).toBe("/setup");
    noDirections();
    expect(block().querySelector("time")).toBeNull();
  });

  it("names another client's M31 within 5 s through the session poll", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(TIME * 1000);
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    let target = SESSION.target;
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ ...SESSION, target })));
    render(<SharedGuidance />);
    await act(async () => {});
    expect(block().textContent).toContain("M78");
    target = M31;
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(block().textContent).toContain("M31");
    expect(block().textContent).toContain("Andromeda Galaxy");
    expect(block().textContent).not.toContain("M78");
  });
});
