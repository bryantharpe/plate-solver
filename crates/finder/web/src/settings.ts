// Per-device preferences, kept in localStorage. Storage can be missing or throw (private
// windows, blocked site data), so every read and write is guarded and the page works
// without it.

import { useCallback, useEffect, useState } from "react";

export interface Site {
  lat: number;
  lon: number;
}

const keys = { night: "finder.night", site: "finder.site", frame: "finder.frame" } as const;

function read<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

function write(key: string, value: unknown) {
  try {
    if (value === null) window.localStorage.removeItem(key);
    else window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Not stored: the setting lasts until the page reloads.
  }
}

function usePersistent<T>(key: string, fallback: T): [T, (value: T) => void] {
  const [value, setValue] = useState<T>(() => read(key, fallback));
  const set = useCallback(
    (next: T) => {
      setValue(next);
      write(key, next);
    },
    [key],
  );
  return [value, set];
}

const prefersDark = () => {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    return false;
  }
};

/**
 * Night mode: red and black only. Until the user picks, it follows the device's dark
 * setting. It sets `data-theme` on the root element (index.html does the same before the
 * first paint, so a night page never flashes white) and the browser's theme colour.
 */
export function useNight(): [boolean, (on: boolean) => void] {
  const [stored, setStored] = usePersistent<boolean | null>(keys.night, null);
  const night = stored ?? prefersDark();
  useEffect(() => {
    document.documentElement.dataset.theme = night ? "night" : "day";
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", night ? "#000000" : "#f8fafc");
  }, [night]);
  return [night, setStored];
}

/** The observing site, used to say what is up. `null` until the user sets one. */
export function useSite(): [Site | null, (site: Site | null) => void] {
  return usePersistent<Site | null>(keys.site, null);
}

/** The last sample solved, so a reload comes back to it. */
export function useLastSample(): [string | null, (name: string | null) => void] {
  return usePersistent<string | null>(keys.frame, null);
}

export function isValidSite(lat: number, lon: number): boolean {
  return Number.isFinite(lat) && Number.isFinite(lon) && Math.abs(lat) <= 90 && Math.abs(lon) <= 180;
}
