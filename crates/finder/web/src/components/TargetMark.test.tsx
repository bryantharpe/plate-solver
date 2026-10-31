import { fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { SolveReport } from "../api";
import { ORION2 } from "../test/fixtures";
import { M31, M78, SESSION } from "../test/targets";
import type { ObservingSession } from "../useSession";
import { Chart, DEFAULT_LAYERS } from "./Chart";
import { TargetMark } from "./TargetMark";

// Oracle: saved orion2 report geometry. Position tolerance 0.5 px, bearing 1°.
// The five legend chips and counts are exact, including with deep sky switched off.
function Harness({
  session = SESSION,
  report = { ...ORION2, pointing_seq: 7 },
}: {
  session?: ObservingSession;
  report?: SolveReport;
}) {
  const [layers, setLayers] = useState(DEFAULT_LAYERS);
  return (
    <Chart
      report={report}
      imageUrl="/frame.jpg"
      label="orion2"
      busy={false}
      layers={layers}
      onLayers={setLayers}
      session={session}
    />
  );
}
function translation(element: Element) {
  const numbers = element
    .getAttribute("transform")!
    .match(/-?\d+(?:\.\d+)?/g)!
    .map(Number);
  return { x: numbers[0]!, y: numbers[1]! };
}
const chips = () =>
  within(screen.getByRole("group", { name: "Show on the chart" }))
    .getAllByRole("button")
    .map((b) => b.textContent);

describe("the session target on the chart", () => {
  it("centres a reticle and family symbol on M78 and keeps the five legend chips unchanged", () => {
    const { container, rerender } = render(<Harness session={{ ...SESSION, target: null }} />);
    const original = chips();
    rerender(<Harness />);
    expect(chips()).toEqual(original);
    expect(original).toEqual(["Grid12", "Matched30", "Unseen25", "Deep sky3", "Centroids67"]);
    const target = container.querySelector('[data-target="M78"]')!;
    const mark = ORION2.objects.find((o) => o.id === "M78")!;
    const at = translation(target);
    expect(Math.hypot(at.x - mark.x, at.y - mark.y)).toBeLessThanOrEqual(0.5);
    expect(target.querySelector(".target-reticle")).toBeTruthy();
    expect(target.querySelector(".target-symbol rect")).toBeTruthy();
    expect(target.textContent).toContain("M78");
    fireEvent.click(screen.getByRole("button", { name: /Deep sky/ }));
    expect(container.querySelector('[data-object="M78"]')).toBeNull();
    expect(container.querySelector('[data-target="M78"]')).toBeTruthy();
    expect(chips()).toEqual(original);
  });

  it("draws an edge arrow toward M79 within 1°, labelled with its whole-degree distance", () => {
    const mark = ORION2.nearby.find((o) => o.id === "M79")!;
    const session: ObservingSession = {
      ...SESSION,
      target: { ...M78, id: "NGC 1904", messier: "M79", family: "globular_cluster" },
      guidance: {
        class: "pointable",
        placement: { on_frame: false, direction_deg: mark.direction_deg, distance_deg: mark.separation_deg },
      },
    };
    const { container } = render(<Harness session={session} />);
    const target = container.querySelector('[data-target="M79"]')!;
    const arrow = target.querySelector("svg g")!;
    const bearing = Number(arrow.getAttribute("transform")!.match(/rotate\(([^)]+)\)/)![1]);
    expect(Math.abs(bearing - mark.direction_deg)).toBeLessThanOrEqual(1);
    expect(target.textContent).toContain("M79 · 21°");
    const at = translation(target);
    expect(at.x).toBeGreaterThan(0);
    expect(at.y).toBeGreaterThan(0);
    expect(at.x).toBeLessThan(ORION2.width);
    expect(at.y).toBeLessThan(ORION2.height);
    // The arrow lies on the centre-to-target ray, at most 24 CSS px from an edge.
    const positionBearing =
      ((Math.atan2(at.x - ORION2.width / 2, ORION2.height / 2 - at.y) * 180) / Math.PI + 360) % 360;
    expect(Math.abs(positionBearing - mark.direction_deg)).toBeLessThanOrEqual(1);
    expect(Math.min(at.x, at.y, ORION2.width - at.x, ORION2.height - at.y)).toBeLessThanOrEqual(
      (24 * ORION2.width) / 320,
    );
    expect(chips()).toHaveLength(5);
  });

  it.each([undefined, 6, 8])("draws no target when the report sequence is %s", (seq) => {
    const { container } = render(<Harness report={{ ...ORION2, pointing_seq: seq }} />);
    expect(container.querySelector("[data-target]")).toBeNull();
  });

  it("draws no target without a session pointing, placement or target", () => {
    for (const session of [
      { ...SESSION, pointing: null },
      { ...SESSION, guidance: undefined },
      { ...SESSION, target: null },
    ]) {
      const { container, unmount } = render(<Harness session={session} />);
      expect(container.querySelector("[data-target]")).toBeNull();
      unmount();
    }
  });

  it("uses an oriented galaxy ellipse and the supplied axes inside the reticle", () => {
    const { container } = render(
      <svg>
        <TargetMark
          target={M31}
          placement={{ on_frame: true, x: 300, y: 200, axes: [40, 18], angle: 35 }}
          W={800}
          H={600}
          u={1}
        />
      </svg>,
    );
    const symbol = container.querySelector(".target-symbol ellipse")!;
    expect(symbol.getAttribute("rx")).toBe("40");
    expect(symbol.getAttribute("ry")).toBe("18");
    expect(symbol.getAttribute("transform")).toBe("rotate(-55)");
  });
});
