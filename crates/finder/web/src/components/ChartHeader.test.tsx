import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ORION2, ROOF } from "../test/fixtures";
import { ChartHeader } from "./ChartHeader";

describe("the chart header", () => {
  it("says where the frame points: constellation, RA and Dec, field, roll, scale and fit", () => {
    render(<ChartHeader report={ORION2} label="orion2.jpg" sigma={8} onRetry={() => {}} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Orion");
    const card = screen.getByRole("region", { name: "Orion" });
    const text = (card.textContent ?? "").replace(/\s+/g, "");
    expect(text).toContain("RA05h40m46s");
    expect(text).toContain("Dec−04°14′46″");
    expect(card.textContent).toContain("Field 10.2° × 10.2°");
    expect(card.textContent).toContain("roll 60°");
    expect(card.textContent).toContain("72″/px");
    expect(card.textContent).toContain("30 of 67 stars matched");
    // The residual never breaks from its unit.
    expect(card.textContent).toContain("12″ RMS");
  });

  it("sets its facts as clauses that wrap only between them", () => {
    const { container } = render(<ChartHeader report={ORION2} label="orion2.jpg" sigma={8} onRetry={() => {}} />);
    const clauses = [...container.querySelectorAll(".clauses > span > span")].map((s) => s.textContent);
    expect(clauses).toEqual([
      "Field 10.2° × 10.2°",
      "roll 60°",
      "72″/px",
      expect.stringMatching(/^Solved in [\d.]+ ms$/),
      "30 of 67 stars matched",
      "12″ RMS",
    ]);
  });

  it("says what to try when a frame does not solve, and offers a lower threshold", () => {
    const retry = vi.fn();
    const { rerender } = render(<ChartHeader report={ROOF} label="roof.jpg" sigma={8} onRetry={retry} />);
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("No match");
    expect(screen.getByText(ROOF.hint ?? "")).toBeTruthy();
    expect(screen.getByText(/roof\.jpg: 5 stars detected at 8σ, 5 tried/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Try again at 5σ" }));
    expect(retry).toHaveBeenCalledWith(5);

    rerender(<ChartHeader report={ROOF} label="roof.jpg" sigma={5} onRetry={retry} />);
    fireEvent.click(screen.getByRole("button", { name: "Try again at 3σ" }));
    expect(retry).toHaveBeenLastCalledWith(3);

    rerender(<ChartHeader report={ROOF} label="roof.jpg" sigma={3} onRetry={retry} />);
    expect(screen.queryByRole("button")).toBeNull();
  });
});
