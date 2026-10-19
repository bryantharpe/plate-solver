import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { isValidSite, useLastSample, useNight, useSite } from "./settings";

beforeEach(() => {
  document.documentElement.dataset.theme = "day";
  document.head.innerHTML = '<meta name="theme-color" content="#f7f8f9">';
});
afterEach(() => vi.restoreAllMocks());

describe("night mode", () => {
  it("follows the device until chosen, then keeps the choice", () => {
    const { result, unmount } = renderHook(() => useNight());
    expect(result.current[0]).toBe(false);
    expect(document.documentElement.dataset.theme).toBe("day");

    act(() => result.current[1](true));
    expect(document.documentElement.dataset.theme).toBe("night");
    expect(document.querySelector('meta[name="theme-color"]')?.getAttribute("content")).toBe("#000000");
    expect(window.localStorage.getItem("finder.night")).toBe("true");
    unmount();

    // A reload comes back in night mode.
    const again = renderHook(() => useNight());
    expect(again.result.current[0]).toBe(true);
  });

  it("follows a dark device setting before the user chooses", () => {
    vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("dark") }));
    const { result } = renderHook(() => useNight());
    expect(result.current[0]).toBe(true);
    vi.unstubAllGlobals();
  });

  it("still works when storage is blocked", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    const { result } = renderHook(() => useNight());
    expect(result.current[0]).toBe(false);
    act(() => result.current[1](true));
    expect(result.current[0]).toBe(true);
    expect(document.documentElement.dataset.theme).toBe("night");
  });
});

describe("site and last frame", () => {
  it("remembers the observing site and forgets it when cleared", () => {
    const { result } = renderHook(() => useSite());
    expect(result.current[0]).toBeNull();
    act(() => result.current[1]({ lat: 40, lon: -75 }));
    expect(renderHook(() => useSite()).result.current[0]).toEqual({ lat: 40, lon: -75 });
    act(() => result.current[1](null));
    expect(window.localStorage.getItem("finder.site")).toBeNull();
  });

  it("remembers the last sample charted", () => {
    const { result } = renderHook(() => useLastSample());
    act(() => result.current[1]("orion2.jpg"));
    expect(renderHook(() => useLastSample()).result.current[0]).toBe("orion2.jpg");
  });

  it("accepts only real latitudes and longitudes", () => {
    expect(isValidSite(40, -75)).toBe(true);
    expect(isValidSite(-90, 180)).toBe(true);
    expect(isValidSite(91, 0)).toBe(false);
    expect(isValidSite(0, -181)).toBe(false);
    expect(isValidSite(Number.NaN, 0)).toBe(false);
  });
});
