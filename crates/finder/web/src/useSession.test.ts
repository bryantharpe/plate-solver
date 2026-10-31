import { act, cleanup, render, renderHook, screen, waitFor } from "@testing-library/react";
import { createElement, StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { useSession, type ObservingSession } from "./useSession";

// Oracles: the session wire contract and the earlier console's finder.site record.
// Polling uses an injected fake clock; request counts and coordinates are exact.
const EMPTY = { site: null, min_alt_deg: 15, target: null };
const HEALTH = {
  status: "ok",
  version: "0.1.0",
  database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
  samples: 21,
};

function serve(initial: ObservingSession = EMPTY) {
  let session = initial;
  const fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input) === "/api/session") {
      if (init?.method === "PUT") session = { ...session, ...JSON.parse(String(init.body)) };
      return new Response(JSON.stringify(session));
    }
    return new Response(JSON.stringify(String(input) === "/api/samples" ? [] : HEALTH));
  });
  vi.stubGlobal("fetch", fetch);
  return { fetch, session: () => session };
}

beforeEach(() => {
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  window.history.replaceState(null, "", "/");
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("the observing session", () => {
  it("reads on mount and every 5 s only while visible, resumes, and cleans up", async () => {
    vi.useFakeTimers();
    const { fetch } = serve();
    const { unmount } = renderHook(() => useSession());
    await act(async () => {});
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/session");
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
    await act(() => vi.advanceTimersByTimeAsync(5000));
    document.dispatchEvent(new Event("visibilitychange"));
    expect(fetch).toHaveBeenCalledTimes(5);
  });

  it("does not start polling on a hidden page", async () => {
    vi.useFakeTimers();
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    const { fetch } = serve();
    renderHook(() => useSession());
    await act(() => vi.advanceTimersByTimeAsync(10000));
    expect(fetch).not.toHaveBeenCalled();
  });

  it("puts JSON and refetches exactly once after the edit", async () => {
    const { fetch } = serve();
    const { result } = renderHook(() => useSession());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    await act(() => result.current.put({ min_alt_deg: 30 }));
    expect(fetch.mock.calls).toEqual([
      ["/api/session"],
      ["/api/session", { method: "PUT", headers: { "Content-Type": "application/json" }, body: '{"min_alt_deg":30}' }],
      ["/api/session"],
    ]);
    expect(result.current.session.min_alt_deg).toBe(30);
  });

  it("reads App.test's health response as an empty session", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify(HEALTH)));
    const { result } = renderHook(() => useSession());
    await waitFor(() => expect(result.current.loaded).toBe(true));
    expect(result.current.session).toEqual({ site: null, min_alt_deg: undefined, target: null });
  });

  it("adopts 40° N, 75° W once, clears the browser copy, and leaves a later session alone", async () => {
    const api = serve();
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    const first = render(createElement(StrictMode, null, createElement(App)));
    await waitFor(() => expect(api.session().site).toEqual({ lat_deg: 40, lon_deg: -75 }));
    await waitFor(() => expect(window.localStorage.getItem("finder.site")).toBeNull());
    expect(api.fetch.mock.calls.filter(([, init]) => init?.method === "PUT")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Site 40.0° N" })).toBeTruthy();
    first.unmount();

    const later = serve({ ...EMPTY, site: { lat_deg: 51, lon_deg: 0 } });
    render(createElement(App));
    await screen.findByRole("button", { name: "Site 51.0° N" });
    expect(later.fetch.mock.calls.filter(([, init]) => init?.method === "PUT")).toHaveLength(0);
    expect(later.session().site).toEqual({ lat_deg: 51, lon_deg: 0 });
  });

  it("keeps an existing session site and discards the obsolete browser copy", async () => {
    const api = serve({ ...EMPTY, site: { lat_deg: 51, lon_deg: 0 } });
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    render(createElement(App));
    await screen.findByRole("button", { name: "Site 51.0° N" });
    await waitFor(() => expect(window.localStorage.getItem("finder.site")).toBeNull());
    expect(api.fetch.mock.calls.filter(([, init]) => init?.method === "PUT")).toHaveLength(0);
  });

  it("keeps the browser site if adoption fails and reports the failed write", async () => {
    const api = serve();
    const fetch = api.fetch.getMockImplementation()!;
    api.fetch.mockImplementation((input, init) =>
      init?.method === "PUT"
        ? Promise.resolve(new Response('{"error":"Session could not be saved"}', { status: 500 }))
        : fetch(input, init),
    );
    window.localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    render(createElement(App));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Session could not be saved");
    expect(JSON.parse(window.localStorage.getItem("finder.site")!)).toEqual({ lat: 40, lon: -75 });
    expect(api.session().site).toBeNull();
  });
});
