import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LiveReport, ObservingSession } from "../api";
import { ORION2 } from "../test/fixtures";
import { SetupPage } from "./SetupPage";

// Oracle: spec's full-frame coordinate contract. A 364×272 displayed preview
// at (20, 40) is one quarter of 1456×1088: tap (195, 145) means (700, 420).
const SESSION: ObservingSession = {
  site: null,
  aim: { x: 700, y: 420 },
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
const FRAME: LiveReport = {
  frame_id: 42,
  time: 1768528800,
  pointing_seq: null,
  preview_seq: 42,
  stars: 30,
  report: { ...ORION2, name: null, width: 1456, height: 1088 },
};
class LiveSource extends EventTarget {
  static current: LiveSource;
  close = vi.fn();
  constructor() {
    super();
    LiveSource.current = this;
  }
  emit(frame = FRAME) {
    this.dispatchEvent(new MessageEvent("live", { data: JSON.stringify(frame) }));
  }
}
beforeEach(() => vi.stubGlobal("EventSource", LiveSource));
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function open() {
  render(<SetupPage session={SESSION} put={vi.fn()} onBack={() => {}} />);
  act(() => LiveSource.current.emit());
  const image = screen.getByRole("img", { name: /preview.*42/i });
  vi.spyOn(image, "getBoundingClientRect").mockReturnValue({
    x: 20,
    y: 40,
    left: 20,
    top: 40,
    width: 364,
    height: 272,
    right: 384,
    bottom: 312,
    toJSON: () => ({}),
  });
  fireEvent.load(image);
  return image;
}
describe("Setup's aim-point tap", () => {
  it("posts the shown preview's scaled pixel and frame id, even after newer frames arrive", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(SESSION)));
    vi.stubGlobal("fetch", fetch);
    const image = open();
    expect(image.closest(".photo")).toBeTruthy();
    expect(image.getAttribute("src")).toBe("/api/live/preview.jpg?seq=42");
    act(() =>
      LiveSource.current.emit({
        ...FRAME,
        frame_id: 43,
        preview_seq: null,
        report: { ...FRAME.report, width: 800, height: 600 },
      }),
    );
    fireEvent.click(image, { clientX: 195, clientY: 145 });
    expect(await screen.findByText("Aim point set.")).toBeTruthy();
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/aim", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ frame_id: 42, x: 700, y: 420 }),
    });
    expect(screen.getByText(/Aim point:.*700.*420/)).toBeTruthy();
  });

  it.each([
    [422, "no star is close enough to the tap"],
    [409, "the console no longer holds that frame"],
  ])("shows why status %s set nothing and retains the old aim point", async (status, error) => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error }), { status })),
    );
    const image = open();
    fireEvent.click(image, { clientX: 195, clientY: 145 });
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toBe(error);
    expect(screen.getByText(/Aim point:.*700.*420/)).toBeTruthy();
  });

  it("resets immediately from DELETE's session answer", async () => {
    const { aim: _aim, ...reset } = SESSION;
    const fetch = vi.fn(async () => new Response(JSON.stringify(reset)));
    vi.stubGlobal("fetch", fetch);
    open();
    fireEvent.click(screen.getByRole("button", { name: "Reset aim point" }));
    expect(await screen.findByText(/Aim point: frame centre/)).toBeTruthy();
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/aim", { method: "DELETE" });
    expect(screen.queryByText(/Aim point:.*700.*420/)).toBeNull();
  });

  it("does not post a tap when a preview fails to load", async () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    const image = open();
    fireEvent.error(image);
    fireEvent.click(image, { clientX: 195, clientY: 145 });
    expect(fetch).not.toHaveBeenCalled();
    expect(screen.getByText(/preview unavailable/i)).toBeTruthy();
  });

  it("can reset a saved aim point before any preview arrives", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ site: null })));
    vi.stubGlobal("fetch", fetch);
    render(<SetupPage session={{ site: null, aim: SESSION.aim }} put={vi.fn()} onBack={() => {}} />);
    expect(screen.queryByRole("img")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Reset aim point" }));
    expect(await screen.findByText(/Aim point: frame centre/)).toBeTruthy();
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
