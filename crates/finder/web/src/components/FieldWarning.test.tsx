import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SetupPage } from "./SetupPage";

// Oracle: FR-037, inclusive 1° margins and 30 s, exact displayed degrees.
function open(fov_deg: number | null, unsolved_s = 0) {
  return render(
    <SetupPage
      session={{ site: null, field: { fov_deg, db_min_fov_deg: 10, db_max_fov_deg: 30, unsolved_s } }}
      put={vi.fn()}
      onBack={() => {}}
    />,
  );
}
describe("Setup's camera field and database range", () => {
  it("shows 12° beside 10–30° without a warning", () => {
    open(12);
    expect(screen.getByText(/Camera field: 12°/)).toBeTruthy();
    expect(screen.getByText(/Database range: 10–30°/)).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it.each([10.5, 9, 10, 11, 29, 30, 31])("warns at field %s° near or outside the range", (field) => {
    open(field);
    expect(screen.getByText(`Camera field: ${field}°`)).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toMatch(/database built for the camera's field may be needed/);
  });

  it.each([11.01, 28.99])("does not warn just beyond the 1° margin at %s°", (field) => {
    open(field);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it.each([null, 12])("warns after 30 s of unsolved frames even with last solved field %s", (field) => {
    open(field, 30);
    const warning = screen.getByRole("alert").textContent!;
    expect(warning).toContain("10–30°");
    expect(warning).toMatch(/at least 10 stars/);
    expect(warning).toMatch(/30 s/);
    expect(warning).toMatch(/field may be outside/);
    expect(warning).toMatch(/matching database may be needed/);
  });

  it("does not infer an out-of-range field before 30 s or before a frame", () => {
    const view = open(null, 29.99);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByText(/Camera field: not solved yet/)).toBeTruthy();
    view.rerender(<SetupPage session={{ site: null }} put={vi.fn()} onBack={() => {}} />);
    expect(screen.queryByText(/Database range:/)).toBeNull();
  });
});
