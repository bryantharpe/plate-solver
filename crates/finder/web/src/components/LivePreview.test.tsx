import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LivePreview } from "./LivePreview";

// Oracle: plan.md's preview URL and DESIGN.md's .photo red-only filter.
describe("the live preview", () => {
  it("draws the named preview inside the red-only photo element and updates its sequence", () => {
    const { rerender } = render(<LivePreview seq={42} />);
    const image = screen.getByRole("img", { name: /preview/i });
    expect(image.getAttribute("src")).toBe("/api/live/preview.jpg?seq=42");
    expect(image.closest(".photo")).toBeTruthy();
    rerender(<LivePreview seq={43} />);
    expect(screen.getByRole("img").getAttribute("src")).toBe("/api/live/preview.jpg?seq=43");
  });

  it("requests no image until a preview exists, accepting sequence zero", () => {
    const { rerender } = render(<LivePreview seq={null} />);
    expect(screen.queryByRole("img")).toBeNull();
    rerender(<LivePreview seq={0} />);
    expect(screen.getByRole("img").getAttribute("src")).toBe("/api/live/preview.jpg?seq=0");
  });
});
