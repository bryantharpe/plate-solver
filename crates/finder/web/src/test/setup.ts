// Test setup: jsdom has no layout, so the chart's ResizeObserver is a stub, and each test
// starts from an empty page and empty storage.

import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

class NoResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= NoResizeObserver as unknown as typeof ResizeObserver;

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});
