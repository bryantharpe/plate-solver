// Test setup: jsdom has no layout, so the chart's ResizeObserver is a stub; it has no modal
// dialogs, so showModal and close just set `open`; and each test starts from an empty page
// and empty storage.

import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

class NoResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= NoResizeObserver as unknown as typeof ResizeObserver;

HTMLDialogElement.prototype.showModal ??= function (this: HTMLDialogElement) {
  this.open = true;
};
HTMLDialogElement.prototype.close ??= function (this: HTMLDialogElement) {
  this.open = false;
  this.dispatchEvent(new Event("close"));
};

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});
