import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { ORION2, SIMILAR_M42 } from "../test/fixtures";
import { NextTargets } from "./NextTargets";
import { SetupPage } from "./SetupPage";

// Oracles: US6's exact ranges and the saved M42 suggestions at 40° N, 75° W.
const SESSION = { site: { lat_deg: 40, lon_deg: -75 }, min_alt_deg: 15, target: null };
function open(session: { site: typeof SESSION.site | null; min_alt_deg: number } = SESSION) {
  const put = vi.fn(async () => {});
  render(<SetupPage session={session} put={put} onBack={() => {}} />);
  return put;
}

beforeEach(() => window.history.replaceState(null, "", "/"));
afterEach(() => {
  window.history.replaceState(null, "", "/");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("Setup", () => {
  it("saves a minimum altitude of 30° and acknowledges it", async () => {
    const put = open();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await waitFor(() => expect(put).toHaveBeenCalledExactlyOnceWith({ min_alt_deg: 30 }));
    expect(await screen.findByText("Minimum altitude saved.")).toBeTruthy();
  });

  it.each(["90", "-1", "", "NaN"])("refuses minimum altitude %j with the 0–80° range", (value) => {
    const put = open();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    expect(put).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toContain("0–80°");
  });

  it.each(["0", "80"])("accepts the minimum altitude boundary %s exactly", async (value) => {
    const put = open();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await waitFor(() => expect(put).toHaveBeenCalledWith({ min_alt_deg: Number(value) }));
  });

  it.each([
    ["95", "0"],
    ["0", "181"],
    ["", "0"],
  ])("refuses site %s, %s and explains both ranges", (lat, lon) => {
    const put = open();
    fireEvent.change(screen.getByLabelText("Latitude"), { target: { value: lat } });
    fireEvent.change(screen.getByLabelText("Longitude"), { target: { value: lon } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    expect(put).not.toHaveBeenCalled();
    expect(screen.getByRole("status").textContent).toMatch(/−90 to 90.*−180 to 180.*east positive/);
  });

  it("saves typed coordinates with the session's wire names and forgets them", async () => {
    const put = open();
    fireEvent.change(screen.getByLabelText("Latitude"), { target: { value: "-90" } });
    fireEvent.change(screen.getByLabelText("Longitude"), { target: { value: "180" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    await screen.findByText("Site saved.");
    expect(put).toHaveBeenCalledWith({ site: { lat_deg: -90, lon_deg: 180 } });
    expect(window.localStorage.getItem("finder.site")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Forget the site" }));
    await waitFor(() => expect(put).toHaveBeenLastCalledWith({ site: null }));
  });

  it("fills coordinates from the device without saving until asked", async () => {
    vi.stubGlobal("isSecureContext", true);
    vi.stubGlobal("navigator", {
      geolocation: {
        getCurrentPosition: (ok: (p: unknown) => void) => ok({ coords: { latitude: 51.4779, longitude: -0.0015 } }),
      },
    });
    const put = open({ site: null, min_alt_deg: 15 });
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("51.478");
    expect((screen.getByLabelText("Longitude") as HTMLInputElement).value).toBe("-0.002");
    expect(put).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Save site" }));
    await waitFor(() => expect(put).toHaveBeenCalledWith({ site: { lat_deg: 51.478, lon_deg: -0.002 } }));
  });

  it("explains when the device cannot share its location", () => {
    vi.stubGlobal("isSecureContext", false);
    open();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect(screen.getByRole("status").textContent).toMatch(/HTTPS.*Enter it by hand/);
  });

  it("explains a denied location request", () => {
    vi.stubGlobal("isSecureContext", true);
    vi.stubGlobal("navigator", {
      geolocation: {
        getCurrentPosition: (_ok: unknown, fail: (e: { message: string }) => void) =>
          fail({ message: "Permission denied" }),
      },
    });
    open();
    fireEvent.click(screen.getByRole("button", { name: "Use this device's location" }));
    expect(screen.getByRole("status").textContent).toContain("No location: Permission denied. Enter it by hand.");
  });

  it("does not claim a failed edit was saved", async () => {
    const put = vi.fn(async () => {
      throw new Error("Session could not be saved");
    });
    render(<SetupPage session={SESSION} put={put} onBack={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    expect(await screen.findByText("Session could not be saved")).toBeTruthy();
    expect(screen.queryByText("Minimum altitude saved.")).toBeNull();
  });

  it("uses pushState to open Setup and return to the chart with a shared 30° minimum", async () => {
    let session = SESSION;
    const fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/session") {
        if (init?.method === "PUT") session = { ...session, ...JSON.parse(String(init.body)) };
        return new Response(JSON.stringify(session));
      }
      return new Response(
        JSON.stringify(
          url === "/api/solve"
            ? ORION2
            : url.startsWith("/api/similar")
              ? SIMILAR_M42
              : url === "/api/samples"
                ? []
                : { version: "test", database: { catalog: "fixture", stars: 1 } },
        ),
      );
    });
    vi.stubGlobal("fetch", fetch);
    const push = vi.spyOn(window.history, "pushState");
    window.localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
    render(<App />);
    await screen.findByRole("heading", { name: "Orion" });
    fireEvent.click(screen.getByRole("link", { name: "Setup" }));
    expect(window.location.pathname).toBe("/setup");
    expect(push).toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
    fireEvent.change(screen.getByRole("textbox", { name: "Minimum altitude" }), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save minimum altitude" }));
    await screen.findByText("Minimum altitude saved.");
    expect(session.min_alt_deg).toBe(30);
    fireEvent.click(screen.getByRole("link", { name: "Back to console" }));
    expect(window.location.pathname).toBe("/");
    expect(push).toHaveBeenCalledTimes(2);
    await screen.findByText("Like M42: other diffuse nebulae");
    const down = SIMILAR_M42.suggestions.filter((s) => s.up === false);
    expect(down.length).toBeGreaterThan(0);
    expect(screen.getAllByText("below 30°")).toHaveLength(down.length);
    expect(screen.queryByText("below 15°")).toBeNull();

    window.history.replaceState(null, "", "/setup");
    act(() => window.dispatchEvent(new PopStateEvent("popstate")));
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
  });

  it("opens /setup directly and fills in the session after loading", async () => {
    window.history.replaceState(null, "", "/setup");
    vi.stubGlobal(
      "fetch",
      async (input: RequestInfo | URL) =>
        new Response(
          JSON.stringify(
            String(input) === "/api/session"
              ? { ...SESSION, min_alt_deg: 30 }
              : String(input) === "/api/samples"
                ? []
                : { version: "test", database: { catalog: "fixture", stars: 1 } },
          ),
        ),
    );
    render(<App />);
    expect(screen.getByRole("heading", { name: "Setup" })).toBeTruthy();
    await waitFor(() => expect((screen.getByLabelText("Latitude") as HTMLInputElement).value).toBe("40"));
    expect((screen.getByRole("textbox", { name: "Minimum altitude" }) as HTMLInputElement).value).toBe("30");
  });

  it("labels every down M42 suggestion below the supplied 30° minimum in readable ink", async () => {
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
    for (const suggestion of SIMILAR_M42.suggestions.filter((s) => s.up === false)) {
      const row = screen.getAllByRole("listitem").find((r) => r.textContent?.startsWith(suggestion.id))!;
      expect(row.textContent).toMatch(/below 30°$/);
      expect(row.className).toContain("text-ink-2");
      expect(row.querySelector("svg")).toBeTruthy();
    }
  });
});
