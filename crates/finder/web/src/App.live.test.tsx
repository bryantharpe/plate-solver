import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { LiveReport, ObservingSession } from "./api";
import { ORION2, SIMILAR_M42, VIRGO } from "./test/fixtures";
import { SESSION, TIME } from "./test/targets";

// Recorded SolveReport fields are the saved Orion/Virgo fixtures, wrapped exactly
// as plan.md's live event specifies. No chart or guidance component is mocked.
class FakeEventSource extends EventTarget {
  static instances: FakeEventSource[] = [];
  close = vi.fn();
  constructor(public url: string) {
    super();
    FakeEventSource.instances.push(this);
  }
  emit(report: LiveReport) {
    this.dispatchEvent(new MessageEvent("live", { data: JSON.stringify(report) }));
  }
}
const LIVE: LiveReport = {
  frame_id: 42,
  time: TIME,
  pointing_seq: 8,
  preview_seq: 42,
  stars: ORION2.detected.length,
  report: { ...ORION2, name: null, pointing_seq: 8 },
  guidance: SESSION.guidance,
};
const CONNECTED: ObservingSession = {
  ...SESSION,
  time: TIME,
  pointing: null,
  node: {
    address: "scope:50051",
    token_set: true,
    connected: true,
    last_frame_age_s: 0,
    offset_s: 1000,
    camera: null,
    stars: 30,
    out_of_range: [],
  },
};
function serve(initial: ObservingSession = CONNECTED) {
  let session = initial;
  const started = Date.now();
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    const body =
      url === "/api/session"
        ? { ...session, time: (session.time ?? TIME) + (Date.now() - started) / 1000 }
        : url === "/api/solve"
          ? { ...VIRGO, pointing_seq: 9 }
          : url.startsWith("/api/similar")
            ? SIMILAR_M42
            : url === "/api/samples"
              ? [{ name: "virgo.jpg", label: "Virgo" }]
              : {
                  status: "ok",
                  version: "0.1.0",
                  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
                  samples: 1,
                };
    return new Response(JSON.stringify(body));
  });
  return (next: ObservingSession) => (session = next);
}
const before = (a: Element, b: Element) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2040-01-01T00:00:00Z"));
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  window.history.replaceState(null, "", "/");
  FakeEventSource.instances = [];
  vi.stubGlobal("EventSource", FakeEventSource);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("the live console", () => {
  it.each([undefined, { ...CONNECTED.node!, connected: false }])("opens no stream with node %s", async (node) => {
    serve({ ...SESSION, node });
    render(<App />);
    await act(async () => {});
    expect(FakeEventSource.instances).toHaveLength(0);
  });

  it("opens and closes the stream as session polls connect and disconnect the node", async () => {
    const poll = serve({ ...CONNECTED, node: { ...CONNECTED.node!, connected: false } });
    render(<App />);
    await act(async () => {});
    expect(FakeEventSource.instances).toHaveLength(0);
    poll(CONNECTED);
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(FakeEventSource.instances).toHaveLength(1);
    expect(FakeEventSource.instances[0]!.url).toBe("/api/live");
    poll({ ...CONNECTED, node: { ...CONNECTED.node!, connected: false } });
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(FakeEventSource.instances[0]!.close).toHaveBeenCalledTimes(1);
  });

  it("draws each live report's coordinates and star marks, with its filtered preview below the legend", async () => {
    serve();
    const { container } = render(<App />);
    await act(async () => {});
    act(() => FakeEventSource.instances[0]!.emit(LIVE));
    const title = screen.getByRole("region", { name: "Orion" });
    const coordinates = title.querySelectorAll("dd");
    expect(coordinates[0]!.textContent).toBe(ORION2.pointing!.ra_text.replace(/\s/g, ""));
    expect(coordinates[1]!.textContent).toBe(ORION2.pointing!.dec_text.replace(/\s/g, ""));
    expect(container.querySelectorAll("[data-star]")).toHaveLength(ORION2.stars.length);
    for (const star of ORION2.stars) {
      const circle = container.querySelector(`[data-star="${star.id}"] circle`)!;
      expect(Number(circle.getAttribute("cx"))).toBe(star.x);
      expect(Number(circle.getAttribute("cy"))).toBe(star.y);
    }
    const image = screen.getByRole("img", { name: /preview/i });
    expect(image.getAttribute("src")).toBe("/api/live/preview.jpg?seq=42");
    expect(image.closest(".photo")).toBeTruthy();
    expect(before(screen.getByRole("group", { name: "Show on the chart" }), image)).toBe(true);
    expect(container.querySelector('[data-target="M78"]')).toBeTruthy();

    act(() =>
      FakeEventSource.instances[0]!.emit({
        ...LIVE,
        frame_id: 43,
        pointing_seq: 10,
        preview_seq: null,
        report: { ...VIRGO, name: null, pointing_seq: 10 },
      }),
    );
    expect(screen.getByRole("heading", { name: VIRGO.pointing!.constellation.name })).toBeTruthy();
    expect(container.querySelectorAll("[data-star]")).toHaveLength(VIRGO.stars.length);
    expect(screen.getByRole("img", { name: /preview/i }).getAttribute("src")).toBe("/api/live/preview.jpg?seq=42");
  });

  it("says pointing stale in guidance and chart within 1 s after 5 s, removes directions, and recovers", async () => {
    serve();
    render(<App />);
    await act(async () => {});
    const stream = FakeEventSource.instances[0]!;
    act(() => stream.emit(LIVE));
    const guidance = screen.getByRole("region", { name: /Target/ });
    const title = screen.getByRole("region", { name: "Orion" });
    expect(guidance.textContent).toContain("turn clockwise 34° · tilt up 12°");
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(within(guidance).queryByText(/pointing stale/i)).toBeNull();
    await act(() => vi.advanceTimersByTimeAsync(250));
    expect(within(guidance).getByText(/pointing stale/i)).toBeTruthy();
    expect(within(title).getByText(/pointing stale/i)).toBeTruthy();
    expect(guidance.textContent).not.toMatch(/turn |tilt /i);
    expect(title.textContent).not.toMatch(/turn |tilt /i);
    act(() => stream.emit({ ...LIVE, time: TIME + 5.25, pointing_seq: 10 }));
    expect(within(guidance).queryByText(/pointing stale/i)).toBeNull();
    expect(within(title).queryByText(/pointing stale/i)).toBeNull();
    expect(guidance.textContent).toContain("turn clockwise 34° · tilt up 12°");
  });

  it("keeps a newer uploaded chart until a newer live solve arrives", async () => {
    serve();
    render(<App />);
    await act(async () => {});
    const stream = FakeEventSource.instances[0]!;
    act(() => stream.emit(LIVE));
    fireEvent.click(screen.getByRole("button", { name: "Live frame 42" }));
    await act(async () => {});
    fireEvent.click(
      within(screen.getByRole("dialog", { name: "Choose a frame" })).getByRole("button", { name: "Virgo" }),
    );
    await act(async () => {});
    expect(screen.getByRole("heading", { name: VIRGO.pointing!.constellation.name })).toBeTruthy();
    act(() => stream.emit({ ...LIVE, frame_id: 41, pointing_seq: 8 }));
    expect(screen.getByRole("heading", { name: VIRGO.pointing!.constellation.name })).toBeTruthy();
    act(() => stream.emit({ ...LIVE, frame_id: 43, pointing_seq: 10, report: { ...LIVE.report, pointing_seq: 10 } }));
    expect(screen.getByRole("heading", { name: "Orion" })).toBeTruthy();
  });

  it("keeps the last solved pointing's age through an unsolved report", async () => {
    serve();
    render(<App />);
    await act(async () => {});
    const stream = FakeEventSource.instances[0]!;
    act(() => stream.emit(LIVE));
    await act(() => vi.advanceTimersByTimeAsync(1000));
    act(() =>
      stream.emit({
        ...LIVE,
        frame_id: 43,
        time: TIME + 1,
        pointing_seq: null,
        preview_seq: null,
        report: { ...LIVE.report, pointing: null, pointing_seq: undefined, status: "no_match" },
      }),
    );
    await act(() => vi.advanceTimersByTimeAsync(4000));
    await act(() => vi.advanceTimersByTimeAsync(250));
    expect(within(screen.getByRole("region", { name: /Target/ })).getByText(/pointing stale/i)).toBeTruthy();
    expect(within(screen.getByRole("region", { name: "No match" })).getByText(/pointing stale/i)).toBeTruthy();
    expect(screen.getByRole("region", { name: /Target/ }).textContent).not.toMatch(/turn |tilt /i);
  });
});
