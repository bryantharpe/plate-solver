import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SiteDialog } from "./SiteDialog";

afterEach(() => vi.unstubAllGlobals());

function open(site: { lat: number; lon: number } | null = null) {
  const onSave = vi.fn();
  const onClose = vi.fn();
  render(<SiteDialog open site={site} onClose={onClose} onSave={onSave} />);
  const [lat, lon] = screen.getAllByRole("textbox") as [HTMLInputElement, HTMLInputElement];
  return { onSave, onClose, lat, lon };
}

describe("the site dialog", () => {
  it("saves a latitude and longitude, east positive", () => {
    const { onSave, onClose, lat, lon } = open();
    fireEvent.change(lat, { target: { value: "40" } });
    fireEvent.change(lon, { target: { value: "-75.5" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    expect(onSave).toHaveBeenCalledWith({ lat: 40, lon: -75.5 });
    expect(onClose).toHaveBeenCalled();
  });

  it("refuses a place that is not on Earth, and says what is allowed", () => {
    const { onSave, lat, lon } = open();
    fireEvent.change(lat, { target: { value: "95" } });
    fireEvent.change(lon, { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toMatch(/−90 to 90.*−180 to 180/);
  });

  it("fills in the device's location when the browser shares it", () => {
    vi.stubGlobal("isSecureContext", true);
    vi.stubGlobal("navigator", {
      geolocation: {
        getCurrentPosition: (ok: (p: { coords: { latitude: number; longitude: number } }) => void) =>
          ok({ coords: { latitude: 51.4779, longitude: -0.0015 } }),
      },
    });
    const { lat, lon } = open();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect([lat.value, lon.value]).toEqual(["51.478", "-0.002"]);
  });

  it("explains when the browser will not share a location over plain HTTP", () => {
    vi.stubGlobal("isSecureContext", false);
    open();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect(screen.getByRole("status").textContent).toMatch(/HTTPS.*Enter it by hand/);
  });

  it("forgets a site it has", () => {
    const { onSave } = open({ lat: 40, lon: -75 });
    fireEvent.click(screen.getByRole("button", { name: "Forget the site" }));
    expect(onSave).toHaveBeenCalledWith(null);
  });
});
