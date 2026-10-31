import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LiveReport, ObservingSession } from "./api";
import { ORION2 } from "./test/fixtures";
import { SESSION, TIME } from "./test/targets";
import { useLive } from "./useLive";
import { useSession } from "./useSession";

// Oracles: the stream wire contract, the saved Orion report, and FR-035's strict
// >5 s boundary. The browser clock deliberately differs from the console clock.
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

const CONNECTED: ObservingSession = {
  ...SESSION,
  time: TIME,
  pointing: { ...SESSION.pointing!, source: "live", time: TIME },
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
const LIVE: LiveReport = {
  frame_id: 42,
  time: TIME,
  pointing_seq: 8,
  preview_seq: 42,
  stars: ORION2.detected.length,
  report: { ...ORION2, name: null, pointing_seq: 8 },
  guidance: SESSION.guidance,
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2040-01-01T00:00:00Z"));
  FakeEventSource.instances = [];
  vi.stubGlobal("EventSource", FakeEventSource);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("live reports", () => {
  it("opens no stream without a node or while disconnected", () => {
    const { rerender } = renderHook((session) => useLive(session), { initialProps: SESSION });
    rerender({ ...CONNECTED, node: { ...CONNECTED.node!, connected: false } });
    expect(FakeEventSource.instances).toHaveLength(0);
  });

  it("opens one stream on connection, keeps it over polls, and closes on disconnect and unmount", () => {
    const { rerender, unmount } = renderHook((session) => useLive(session), { initialProps: SESSION });
    rerender(CONNECTED);
    const stream = FakeEventSource.instances[0]!;
    expect(FakeEventSource.instances).toHaveLength(1);
    expect(stream.url).toBe("/api/live");
    rerender({ ...CONNECTED, time: TIME + 1 });
    expect(FakeEventSource.instances).toHaveLength(1);
    expect(stream.close).not.toHaveBeenCalled();
    rerender({ ...CONNECTED, node: { ...CONNECTED.node!, connected: false } });
    expect(stream.close).toHaveBeenCalledTimes(1);
    rerender(CONNECTED);
    expect(FakeEventSource.instances).toHaveLength(2);
    unmount();
    expect(FakeEventSource.instances[1]!.close).toHaveBeenCalledTimes(1);
  });

  it("keeps the report and marks stale within 250 ms after 5 s, then clears on the next solve", () => {
    const { result } = renderHook(() => useLive(CONNECTED));
    const stream = FakeEventSource.instances[0]!;
    act(() => stream.emit(LIVE));
    expect(result.current.live).toEqual(LIVE);
    expect(result.current.stale).toBe(false);
    act(() => vi.advanceTimersByTime(5000));
    expect(result.current.stale).toBe(false);
    act(() => vi.advanceTimersByTime(250));
    expect(result.current.stale).toBe(true);
    expect(result.current.session.guidance?.stale).toBe(true);
    expect(result.current.session.guidance?.turn_deg).toBeUndefined();
    expect(result.current.session.guidance?.tilt_deg).toBeUndefined();
    act(() => stream.emit({ ...LIVE, frame_id: 43, time: TIME + 5.25, pointing_seq: 9 }));
    expect(result.current.stale).toBe(false);
    expect(result.current.session.guidance?.turn_deg).toBe(33.6);
  });

  it("uses a poll's console time and a live pointing even before the first event", () => {
    const { result, rerender } = renderHook((session) => useLive(session), { initialProps: CONNECTED });
    act(() => vi.advanceTimersByTime(250));
    rerender({ ...CONNECTED, time: TIME + 5.1 });
    expect(result.current.stale).toBe(true);
  });

  it("does not let a failed solve or an older poll refresh the newest pointing", () => {
    const { result, rerender } = renderHook((session) => useLive(session), { initialProps: CONNECTED });
    const stream = FakeEventSource.instances[0]!;
    act(() => stream.emit(LIVE));
    act(() => vi.advanceTimersByTime(5250));
    act(() =>
      stream.emit({
        ...LIVE,
        frame_id: 43,
        time: TIME + 5.25,
        pointing_seq: null,
        report: { ...ORION2, name: null, pointing: null, status: "no_match" },
        guidance: { class: "pointable", stale: true },
      }),
    );
    rerender({ ...CONNECTED, time: TIME + 5.25 });
    expect(result.current.stale).toBe(true);
    expect(result.current.session.pointing?.seq).toBe(8);
    expect(result.current.session.guidance?.turn_deg).toBeUndefined();
  });

  it("never ages an uploaded pointing, including one newer than the stream", () => {
    const { result, rerender } = renderHook((session) => useLive(session), { initialProps: CONNECTED });
    act(() => FakeEventSource.instances[0]!.emit(LIVE));
    rerender({ ...CONNECTED, pointing: { ...CONNECTED.pointing!, seq: 9, source: "upload" } });
    act(() => vi.advanceTimersByTime(60000));
    expect(result.current.stale).toBe(false);
    expect(result.current.session.pointing?.source).toBe("upload");
  });

  it("preserves optional live session fields on reads without adding missing or null fields", async () => {
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    const wire: ObservingSession = {
      ...CONNECTED,
      camera_settings: { exposure_ms: "auto", gain: 2, sigma: 8 },
      aim: { x: 700, y: 420 },
      field: { fov_deg: 12, db_min_fov_deg: 10, db_max_fov_deg: 30, unsolved_s: 0 },
    };
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify(wire)));
    const { result, unmount } = renderHook(() => useSession());
    await act(async () => {});
    expect(result.current.session).toEqual(wire);
    unmount();
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ site: null, node: null })));
    const empty = renderHook(() => useSession());
    await act(async () => {});
    for (const key of ["node", "camera_settings", "aim", "field"] as const) {
      expect(empty.result.current.session[key]).toBeUndefined();
      expect(Object.hasOwn(empty.result.current.session, key)).toBe(false);
    }
  });
});
