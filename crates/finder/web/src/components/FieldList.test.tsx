import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ORION2, VIRGO } from "../test/fixtures";
import { FieldList, FieldRoster } from "./FieldList";

describe("what is in the field", () => {
  it("rosters the objects and the brightest named stars in a line", () => {
    const { container } = render(<FieldRoster report={ORION2} />);
    expect(container.textContent).toBe("M42M78M43Alnilam, Alnitak, Mintaka");
  });

  it("points to the full list when the roster cannot hold everything", () => {
    render(<FieldRoster report={VIRGO} />);
    const more = screen.getByRole("link", { name: /more/ });
    const objects = VIRGO.objects.length;
    const named = VIRGO.stars.filter((s) => s.matched && s.name).length;
    expect(more.textContent).toBe(`${objects - 6 + named - Math.min(named, 3)} more`);
    expect(more.getAttribute("href")).toBe("#field-title");
  });

  it("lists every object brightest first with what it is, and selects one", () => {
    const select = vi.fn();
    render(<FieldList report={ORION2} selected="M42" onSelect={select} />);
    const rows = screen.getAllByRole("button");
    expect(rows.map((r) => r.textContent)).toEqual([
      "M42Great Orion NebulaNebula with clustermag 4.0",
      "M78NGC 2068Reflection nebulamag 8.0",
      "M43Mairan's NebulaEmission nebulamag 9.0",
    ]);
    expect(rows[0]?.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(rows[1] as HTMLElement);
    expect(select).toHaveBeenCalledWith("M78");
    // Named stars as the roster gives them: brightest first, after a ring.
    expect(screen.getByText("Alnilam, Alnitak, Mintaka")).toBeTruthy();
  });
});
