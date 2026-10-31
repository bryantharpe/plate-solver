import { act, cleanup, render, renderHook, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { useSession } from "./useSession";

const HEALTH = {
  status: "ok",
  version: "0.1.0",
  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
  samples: 21,
};
const EMPTY = { site: null, min_alt_deg: 15, target: null, pointing: null, time: 0 };
const response = (body: unknown) => new Response(JSON.stringify(body));

beforeEach(() => {
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("the observing session (the API contract is the oracle; times are exact)", () => {
  it("gets on mount and every 5 s while visible, pauses hidden, and cleans up", async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(async (_url: string) => response(EMPTY));
    vi.stubGlobal("fetch", fetch);
    const { unmount } = renderHook(() => useSession());
    await act(async () => {});
    expect(fetch.mock.calls).toEqual([["/api/session"]]);
    await act(() => vi.advanceTimersByTimeAsync(4999));
    expect(fetch).toHaveBeenCalledTimes(1);
    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(fetch).toHaveBeenCalledTimes(2);
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(fetch).toHaveBeenCalledTimes(3);

    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    await act(() => vi.advanceTimersByTimeAsync(15000));
    expect(fetch).toHaveBeenCalledTimes(3);
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    await act(async () => document.dispatchEvent(new Event("visibilitychange")));
    expect(fetch).toHaveBeenCalledTimes(4);
    await act(() => vi.advanceTimersByTimeAsync(5000));
    expect(fetch).toHaveBeenCalledTimes(5);
    unmount();
    await vi.advanceTimersByTimeAsync(5000);
    document.dispatchEvent(new Event("visibilitychange"));
    expect(fetch).toHaveBeenCalledTimes(5);
  });

  it("waits for visibility when mounted in a hidden page", async () => {
    vi.useFakeTimers();
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    const fetch = vi.fn(async () => response(EMPTY));
    vi.stubGlobal("fetch", fetch);
    renderHook(() => useSession());
    await act(() => vi.advanceTimersByTimeAsync(10000));
    expect(fetch).not.toHaveBeenCalled();
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    await act(async () => document.dispatchEvent(new Event("visibilitychange")));
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("puts JSON and refetches exactly once, even when hidden", async () => {
    const fetch = vi.fn(async (_url: string, _init?: RequestInit) => response(EMPTY));
    vi.stubGlobal("fetch", fetch);
    const { result } = renderHook(() => useSession());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    fetch.mockClear();
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    const edit = { site: { lat_deg: 40, lon_deg: -75 }, min_alt_deg: 30 };
    await act(() => result.current.put(edit));
    expect(fetch.mock.calls).toEqual([
      ["/api/session", { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify(edit) }],
      ["/api/session"],
    ]);
  });

  it("reads App.test's health response as an empty session", async () => {
    vi.stubGlobal("fetch", async () => response(HEALTH));
    const { result } = renderHook(() => useSession());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.session.site).toBeNull();
    expect(result.current.session.min_alt_deg).toBeUndefined();
    expect(result.current.error).toBeNull();
  });

  it("reports a refused edit without refetching or replacing the session", async () => {
    const fetch = vi.fn(async (_url: string, init?: RequestInit) =>
      init?.method === "PUT"
        ? new Response(JSON.stringify({ error: "Site refused" }), { status: 403 })
        : response(EMPTY),
    );
    vi.stubGlobal("fetch", fetch);
    const { result } = renderHook(() => useSession());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    await act(async () => {
      await expect(result.current.put({ site: null })).rejects.toThrow("Site refused");
    });
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(result.current.session.site).toBeNull();
  });

  it("adopts the earlier console's 40 N, 75 W site once, then leaves later sessions alone", async () => {
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    let session = { ...EMPTY, site: null as null | { lat_deg: number; lon_deg: number } };
    const writes: unknown[] = [];
    vi.stubGlobal("fetch", async (url: string, init?: RequestInit) => {
      if (url === "/api/session") {
        if (init?.method === "PUT") {
          const edit = JSON.parse(String(init.body));
          writes.push(edit);
          session = { ...session, ...edit };
        }
        return response(session);
      }
      return response(url.startsWith("/api/samples") ? [{ name: "orion2.jpg", label: "Orion2" }] : HEALTH);
    });
    const first = render(createElement(App));
    await waitFor(() => expect(window.localStorage.getItem("finder.site")).toBeNull());
    expect(session.site).toEqual({ lat_deg: 40, lon_deg: -75 });
    expect(writes).toEqual([{ site: { lat_deg: 40, lon_deg: -75 } }]);
    first.unmount();
    session = { ...session, site: { lat_deg: 51, lon_deg: -1 } };
    render(createElement(App));
    await screen.findByRole("button", { name: "Site 51.0° N" });
    expect(writes).toHaveLength(1);
    expect(session.site).toEqual({ lat_deg: 51, lon_deg: -1 });
  });

  it("keeps an existing session site instead of adopting the browser's site", async () => {
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    const fetch = vi.fn(async (url: string) =>
      response(
        url === "/api/session"
          ? { ...EMPTY, site: { lat_deg: -33, lon_deg: 151 } }
          : url.startsWith("/api/samples")
            ? [{ name: "orion2.jpg", label: "Orion2" }]
            : HEALTH,
      ),
    );
    vi.stubGlobal("fetch", fetch);
    render(createElement(App));
    await screen.findByRole("button", { name: "Site 33.0° S" });
    expect(fetch.mock.calls.filter(([url]) => url === "/api/session")).toHaveLength(1);
    await waitFor(() => expect(window.localStorage.getItem("finder.site")).toBeNull());
  });

  it("keeps the browser's site if the initial session read fails", async () => {
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    const fetch = vi.fn(async (url: string) =>
      url === "/api/session"
        ? new Response("unavailable", { status: 503 })
        : response(url.startsWith("/api/samples") ? [{ name: "orion2.jpg", label: "Orion2" }] : HEALTH),
    );
    vi.stubGlobal("fetch", fetch);
    render(createElement(App));
    await screen.findByText(/Database: hip_main/);
    expect(window.localStorage.getItem("finder.site")).toBe(JSON.stringify({ lat: 40, lon: -75 }));
    expect(fetch.mock.calls.filter(([url]) => url === "/api/session")).toHaveLength(1);
  });
});
