import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { ORION2, SIMILAR_M42 } from "../test/fixtures";
import { NextTargets } from "./NextTargets";
import { SetupPage } from "./SetupPage";

afterEach(() => {
  window.history.replaceState(null, "", "/");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const INITIAL = { site: null, min_alt_deg: 15 };
function setup(site: { lat_deg: number; lon_deg: number } | null = null) {
  const onSave = vi.fn(async () => {});
  const view = render(<SetupPage session={{ ...INITIAL, site }} onSave={onSave} onBack={() => {}} />);
  return { onSave, ...view };
}

// Exact values and messages below come from FR-027–029 and the existing site dialog.
describe("Setup", () => {
  it("saves a minimum altitude of 30 degrees", async () => {
    const { onSave } = setup();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await screen.findByText("Minimum altitude saved.");
    expect(onSave.mock.calls).toEqual([[{ min_alt_deg: 30 }]]);
  });

  it.each(["90", "-1", "", "not a number", "Infinity"])("refuses minimum altitude %s and says 0–80°", (value) => {
    const { onSave } = setup();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toContain("0–80°");
  });

  it.each(["0", "80"])("accepts the minimum altitude boundary %s", async (value) => {
    const { onSave } = setup();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await screen.findByText("Minimum altitude saved.");
    expect(onSave).toHaveBeenCalledWith({ min_alt_deg: Number(value) });
  });

  it.each([
    ["95", "0"],
    ["40", "181"],
    ["", "-75"],
  ])("refuses site %s, %s and gives the ranges", (lat, lon) => {
    const { onSave } = setup();
    fireEvent.change(screen.getByLabelText("Latitude"), { target: { value: lat } });
    fireEvent.change(screen.getByLabelText("Longitude"), { target: { value: lon } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toMatch(/−90 to 90.*−180 to 180.*east positive/);
  });

  it("saves and forgets the shared site without keeping a browser copy", async () => {
    const { onSave, rerender } = setup();
    fireEvent.change(screen.getByLabelText("Latitude"), { target: { value: "40" } });
    fireEvent.change(screen.getByLabelText("Longitude"), { target: { value: "-75" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    await screen.findByText("Site saved.");
    expect(onSave).toHaveBeenCalledWith({ site: { lat_deg: 40, lon_deg: -75 } });
    expect(window.localStorage.getItem("finder.site")).toBeNull();
    rerender(
      <SetupPage session={{ ...INITIAL, site: { lat_deg: 40, lon_deg: -75 } }} onSave={onSave} onBack={() => {}} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Forget the site" }));
    await waitFor(() => expect(onSave).toHaveBeenCalledWith({ site: null }));
  });

  it("fills the device's coordinates and saves them only when asked", async () => {
    vi.stubGlobal("isSecureContext", true);
    vi.stubGlobal("navigator", {
      geolocation: {
        getCurrentPosition: (ok: (p: { coords: { latitude: number; longitude: number } }) => void) =>
          ok({ coords: { latitude: 51.4779, longitude: -0.0015 } }),
      },
    });
    const { onSave } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("51.478");
    expect((screen.getByLabelText("Longitude") as HTMLInputElement).value).toBe("-0.002");
    expect(onSave).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    await screen.findByText("Site saved.");
    expect(onSave).toHaveBeenCalledWith({ site: { lat_deg: 51.478, lon_deg: -0.002 } });
  });

  it("explains when device location is unavailable or refused", () => {
    vi.stubGlobal("isSecureContext", false);
    setup();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect(screen.getByRole("status").textContent).toMatch(/HTTPS.*Enter it by hand/);
    vi.stubGlobal("isSecureContext", true);
    vi.stubGlobal("navigator", {
      geolocation: {
        getCurrentPosition: (_ok: unknown, fail: (e: { message: string }) => void) =>
          fail({ message: "Permission denied" }),
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect(screen.getByRole("status").textContent).toMatch(/Permission denied.*Enter it by hand/);
  });

  it("shows a failed save instead of claiming it was saved", async () => {
    render(
      <SetupPage
        session={INITIAL}
        onSave={async () => {
          throw new Error("Console unavailable");
        }}
        onBack={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await screen.findByText("Console unavailable");
    expect(screen.queryByText("Minimum altitude saved.")).toBeNull();
  });

  it("receives session changes while leaving an unfinished edit alone on an unchanged poll", () => {
    const { onSave, rerender } = setup({ lat_deg: 40, lon_deg: -75 });
    fireEvent.change(screen.getByLabelText("Latitude"), { target: { value: "41" } });
    rerender(
      <SetupPage session={{ ...INITIAL, site: { lat_deg: 40, lon_deg: -75 } }} onSave={onSave} onBack={() => {}} />,
    );
    expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("41");
    rerender(
      <SetupPage session={{ site: { lat_deg: 50, lon_deg: -1 }, min_alt_deg: 30 }} onSave={onSave} onBack={() => {}} />,
    );
    expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("50");
    expect((screen.getByRole("textbox", { name: "Minimum altitude" }) as HTMLInputElement).value).toBe("30");
  });
});

function serveSession() {
  let session = { ...INITIAL, site: { lat_deg: 40, lon_deg: -75 } };
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    if (url === "/api/session") {
      if (init?.method === "PUT") session = { ...session, ...JSON.parse(String(init.body)) };
      return new Response(JSON.stringify(session));
    }
    const body = url.startsWith("/api/samples")
      ? [{ name: "orion2.jpg", label: "Orion2" }]
      : url.startsWith("/api/solve")
        ? ORION2
        : url.startsWith("/api/similar")
          ? SIMILAR_M42
          : {
              status: "ok",
              version: "0.1.0",
              database: { catalog: "hip_main", stars: 42212, patterns: 1, min_fov: 10, max_fov: 30 },
              samples: 21,
            };
    return new Response(JSON.stringify(body));
  });
  vi.stubGlobal("fetch", fetch);
  vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
  return { fetch, session: () => session };
}

describe("Setup routing and suggestions", () => {
  it("uses pushState from the top bar and returns to the same chart with the saved minimum", async () => {
    const api = serveSession();
    window.localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
    const push = vi.spyOn(window.history, "pushState");
    render(<App />);
    await screen.findByText("Like M42: other diffuse nebulae");
    fireEvent.click(screen.getByRole("link", { name: "Setup" }));
    expect(push).toHaveBeenCalledWith(null, "", "/setup");
    expect(window.location.pathname).toBe("/setup");
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await screen.findByText("Minimum altitude saved.");
    expect(api.session().min_alt_deg).toBe(30);
    fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
    expect(push).toHaveBeenLastCalledWith(null, "", "/");
    expect(window.location.pathname).toBe("/");
    await screen.findByText("Like M42: other diffuse nebulae");
    expect(screen.getAllByText("below 30°")).toHaveLength(SIMILAR_M42.suggestions.filter((s) => s.up === false).length);
    expect(screen.queryByText("below 15°")).toBeNull();
    expect(api.fetch.mock.calls.filter(([url]) => url === "/api/solve")).toHaveLength(1);
  });

  it("opens /setup directly and follows popstate back to the console", async () => {
    serveSession();
    window.history.replaceState(null, "", "/setup");
    render(<App />);
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
    await waitFor(() => expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("40"));
    act(() => {
      window.history.replaceState(null, "", "/");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByRole("heading", { name: "Chart a frame" })).toBeTruthy();
  });

  it("labels every down suggestion below 30° and keeps the horizon mark in readable ink", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify(SIMILAR_M42)));
    render(
      <NextTargets
        reference={ORION2.objects[0]!}
        nearby={[]}
        site={{ lat: 40, lon: -75 }}
        minAlt={30}
        onSetSite={() => {}}
      />,
    );
    await screen.findByText("Like M42: other diffuse nebulae");
    const down = SIMILAR_M42.suggestions.filter((s) => s.up === false);
    expect(down.length).toBeGreaterThan(0);
    for (const suggestion of down) {
      const row = screen.getAllByRole("listitem").find((r) => within(r).queryByText(suggestion.id))!;
      expect(within(row).getByText("below 30°")).toBeTruthy();
      expect(row.className).toContain("text-ink-2");
      expect(row.querySelector(".text-right svg")).toBeTruthy();
    }
  });
});
