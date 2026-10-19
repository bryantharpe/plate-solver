import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { ORION2, SIMILAR_M42 } from "./test/fixtures";

afterEach(() => vi.unstubAllGlobals());

/** The console's API, answered from fixtures. */
function serve() {
  const calls: string[] = [];
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    calls.push(url);
    const body = url.startsWith("/api/solve")
      ? ORION2
      : url.startsWith("/api/similar")
        ? SIMILAR_M42
        : url.startsWith("/api/samples")
          ? [{ name: "orion2.jpg", label: "Orion2" }]
          : {
              status: "ok",
              version: "0.1.0",
              database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
              samples: 21,
            };
    return new Response(JSON.stringify(body));
  });
  return calls;
}

const before = (a: Element, b: Element) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;

describe("the console", () => {
  it("opens on a first-run card that offers a sample or your own frame", () => {
    serve();
    render(<App />);
    expect(screen.getByRole("heading", { name: "Chart a frame" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Chart the Orion sample" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Open your own frame" })).toBeTruthy();
  });

  it("comes back to the last frame and reads, in order: where it points, what is in it, the chart, the rest", async () => {
    window.localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
    const calls = serve();
    const { container } = render(<App />);
    const pointing = await screen.findByRole("heading", { name: "Orion" });
    expect(calls).toContain("/api/solve");

    // The roster under the card and the full list after the chart name the same stars alike.
    const [roster, listed] = screen.getAllByText("Alnilam, Alnitak, Mintaka") as [HTMLElement, HTMLElement];
    const chart = container.querySelector("figure.chart") as Element;
    const list = screen.getByRole("heading", { name: "In the field" });
    const next = screen.getByRole("heading", { name: "What to look at next" });
    expect(before(pointing, roster)).toBe(true);
    expect(before(roster, chart)).toBe(true);
    expect(before(chart, list)).toBe(true);
    expect(before(list, listed)).toBe(true);
    expect(before(list, next)).toBe(true);
  });
});
