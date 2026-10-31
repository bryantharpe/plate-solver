import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { ORION2, SIMILAR_M42 } from "./test/fixtures";
import { M31, SESSION } from "./test/targets";
import type { Target } from "./api";

// Oracles: the saved report, session contract, and FR-033's exact DOM reading order.
beforeEach(() => window.history.replaceState(null, "", "/"));
afterEach(() => vi.unstubAllGlobals());
const before = (a: Element, b: Element) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
function serve(initial: Target | null = SESSION.target ?? null) {
  let target = initial;
  vi.stubGlobal("fetch", async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url === "/api/session") {
      if (init?.method === "PUT") target = JSON.parse(String(init.body)).target === null ? null : M31;
      return new Response(JSON.stringify({ ...SESSION, target }));
    }
    const body = url.startsWith("/api/targets?")
      ? [{ ...M31, class: "pointable" }]
      : url === "/api/solve"
        ? { ...ORION2, pointing_seq: 7 }
        : url.startsWith("/api/similar")
          ? SIMILAR_M42
          : url === "/api/samples"
            ? [{ name: "orion2.jpg", label: "Orion2" }]
            : {
                status: "ok",
                version: "0.1.0",
                database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
                samples: 1,
              };
    return new Response(JSON.stringify(body));
  });
  window.localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
}

describe("the page with a target", () => {
  it("reads guidance, title, chart and legend, then the field roster, field list and the rest", async () => {
    serve();
    const { container } = render(<App />);
    const title = await screen.findByRole("heading", { name: "Orion" });
    const guidance = await screen.findByRole("region", { name: /Target/ });
    const chart = container.querySelector("figure.chart")!;
    const legend = screen.getByRole("group", { name: "Show on the chart" });
    const [roster] = screen.getAllByText("Alnilam, Alnitak, Mintaka");
    const field = screen.getByRole("heading", { name: "In the field" });
    expect(before(guidance, title)).toBe(true);
    expect(before(title, chart)).toBe(true);
    expect(before(chart.querySelector("svg")!, legend)).toBe(true);
    expect(before(legend, roster!)).toBe(true);
    expect(before(roster!, field)).toBe(true);
    expect(before(field, screen.getByRole("heading", { name: "What to look at next" }))).toBe(true);
    expect(container.querySelector('[data-target="M78"]')).toBeTruthy();
  });

  it("opens the sheet from the top bar, chooses M31, then clears it and restores the no-target layout", async () => {
    serve(null);
    const { container } = render(<App />);
    await screen.findByRole("heading", { name: "Orion" });
    fireEvent.click(screen.getByRole("button", { name: "Choose target" }));
    const sheet = screen.getByRole("dialog", { name: "Choose a target" });
    fireEvent.change(within(sheet).getByRole("searchbox"), { target: { value: "andro" } });
    fireEvent.click(await within(sheet).findByRole("button", { name: /Andromeda Galaxy/ }));
    const guidance = await screen.findByRole("region", { name: /Target/ });
    expect(guidance.textContent).toContain("M31");
    expect(guidance.textContent).toContain("Andromeda Galaxy");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Choose a target" })).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Choose target" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear target" }));
    await waitFor(() => expect(screen.queryByRole("region", { name: /Target/ })).toBeNull());
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Choose a target" })).toBeNull());
    const chart = container.querySelector("figure.chart")!;
    const [roster] = screen.getAllByText("Alnilam, Alnitak, Mintaka");
    expect(before(roster!, chart)).toBe(true);
    expect(container.querySelector("[data-target]")).toBeNull();
  });
});
