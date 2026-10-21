import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FramePicker } from "./FramePicker";

afterEach(() => vi.unstubAllGlobals());

const SAMPLES = [
  { name: "orion2.jpg", label: "Orion2" },
  { name: "virgo.jpg", label: "Virgo" },
];

function picker(current: string | null = null) {
  vi.stubGlobal("fetch", async () => new Response(JSON.stringify(SAMPLES)));
  const props = { onSettings: vi.fn(), onClose: vi.fn(), onPick: vi.fn() };
  render(<FramePicker open current={current} settings={{}} {...props} />);
  return props;
}

describe("the frame picker", () => {
  it("offers the server's samples with previews, and charts the one tapped", async () => {
    const { onPick, onClose } = picker("orion2.jpg");
    const virgo = await screen.findByRole("button", { name: "Virgo" });
    expect(screen.getByRole("button", { name: "Orion2" }).getAttribute("aria-current")).toBe("true");
    const preview = virgo.querySelector("img");
    expect(preview?.getAttribute("src")).toBe("/api/samples/virgo.jpg/thumb");
    expect(preview?.className).toContain("photo");
    fireEvent.click(virgo);
    expect(onPick).toHaveBeenCalledWith({ kind: "sample", name: "virgo.jpg" });
    expect(onClose).toHaveBeenCalled();
  });

  it("opens a frame from the device", async () => {
    const { onPick } = picker();
    const file = new File([new Uint8Array([1, 2, 3])], "mine.jpg", { type: "image/jpeg" });
    const input = document.querySelector<HTMLInputElement>('input[type="file"]') as HTMLInputElement;
    expect(input.accept).toBe("image/jpeg,image/png,image/bmp,.pgm,.ppm,.pnm");
    fireEvent.change(input, { target: { files: [file] } });
    expect(onPick).toHaveBeenCalledWith({ kind: "file", file });
  });

  it("takes a field of view hint and a detection threshold, or leaves the field unknown", async () => {
    const { onSettings } = picker();
    await screen.findByRole("button", { name: "Virgo" });
    const [fov, sigma] = screen.getAllByRole("textbox") as [HTMLInputElement, HTMLInputElement];
    fireEvent.change(fov, { target: { value: "12.5" } });
    expect(onSettings).toHaveBeenLastCalledWith({ fov: 12.5 });
    fireEvent.change(fov, { target: { value: "" } });
    expect(onSettings).toHaveBeenLastCalledWith({ fov: undefined });
    fireEvent.change(sigma, { target: { value: "5" } });
    expect(onSettings).toHaveBeenLastCalledWith({ sigma: 5 });
  });
});
