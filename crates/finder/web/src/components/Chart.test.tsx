import { fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import type { SolveReport } from "../api";
import { ORION2, ROOF, VIRGO } from "../test/fixtures";
import { Chart, DEFAULT_LAYERS, type Layers } from "./Chart";

function Harness({ report }: { report: SolveReport }) {
  const [layers, setLayers] = useState<Layers>(DEFAULT_LAYERS);
  return (
    <Chart report={report} imageUrl="/frame.jpg" label="frame" busy={false} layers={layers} onLayers={setLayers} />
  );
}

const chips = () =>
  within(screen.getByRole("group", { name: "Show on the chart" }))
    .getAllByRole("button")
    .map((b) => [b.textContent, b.getAttribute("aria-pressed")]);

describe("the chart", () => {
  it("keys every kind of mark in its legend, with counts", () => {
    render(<Harness report={ORION2} />);
    expect(chips()).toEqual([
      ["Grid12", "true"],
      ["Matched30", "true"],
      ["Not detected25", "true"],
      ["Deep sky3", "true"],
      ["Detections76", "false"],
    ]);
  });

  it("draws the deep-sky key from the symbols actually on the chart", () => {
    const { container, unmount } = render(<Harness report={VIRGO} />);
    const key = () => container.querySelectorAll('[role="group"] button:nth-child(4) svg');
    expect([...key()].map((s) => s.getAttribute("class"))).toEqual(["icon-galaxy"]);
    unmount();
    const orion = render(<Harness report={ORION2} />);
    expect(
      [...orion.container.querySelectorAll('[role="group"] button:nth-child(4) svg')].map((s) =>
        s.getAttribute("class"),
      ),
    ).toEqual(["icon-nebula"]);
  });

  it("draws matched and not-detected stars as separate layers, each toggled by its chip", () => {
    const { container } = render(<Harness report={ORION2} />);
    const count = (selector: string) => container.querySelectorAll(selector).length;
    expect(count(".mark-matched")).toBe(30);
    expect(count(".mark-expected")).toBe(25);
    expect(count(".mark-detected")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: /Matched/ }));
    expect(count(".mark-matched")).toBe(0);
    expect(count(".mark-expected")).toBe(25);

    fireEvent.click(screen.getByRole("button", { name: /Not detected/ }));
    expect(count(".mark-expected")).toBe(0);

    fireEvent.click(screen.getByRole("button", { name: /Detections/ }));
    expect(count(".mark-detected")).toBe(76);
  });

  it("draws each Messier object with a label, and names it when tapped", () => {
    const { container } = render(<Harness report={ORION2} />);
    for (const id of ["M42", "M78", "M43"]) {
      const g = container.querySelector(`[data-object="${id}"]`);
      expect(g?.querySelector(".object-label")?.textContent, id).toBe(id);
    }
    fireEvent.click(container.querySelector('[data-object="M42"]') as Element);
    const caption = container.querySelector("figcaption")?.textContent ?? "";
    expect(caption).toContain("Great Orion Nebula");
    expect(caption).toContain("Nebula with cluster");
    expect(caption).toContain("90′ across");
  });

  it("draws the RA/Dec grid with its labels in the margin", () => {
    const { container } = render(<Harness report={ORION2} />);
    expect(container.querySelectorAll(".grid-line").length).toBeGreaterThanOrEqual(ORION2.grid.length);
    const labels = [...container.querySelectorAll(".edge-label")].map((t) => t.textContent);
    expect(labels).toContain("−4°");
    expect(labels.some((l) => /^\dh( \d\dm)?$/.test(l ?? ""))).toBe(true);
  });

  it("shows only the detected stars, fixed on, when a frame does not solve", () => {
    const { container } = render(<Harness report={ROOF} />);
    expect(chips()).toEqual([["Detections5", "true"]]);
    expect(screen.getByRole("button", { name: /Detections/ }).hasAttribute("disabled")).toBe(true);
    expect(container.querySelectorAll(".mark-detected").length).toBe(5);
    expect(container.querySelector(".grid-line")).toBeNull();
  });
});
