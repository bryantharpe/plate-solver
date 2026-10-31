import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LiveReport, ObservingSession } from "../api";
import { ORION2 } from "../test/fixtures";
import { useSession } from "../useSession";
import { SetupPage } from "./SetupPage";

// Oracle: plan.md's session/live wire shapes and simulator camera, exact values.
const SESSION: ObservingSession = {
  site: null,
  node: {
    address: "scope:50051",
    token_set: true,
    connected: true,
    last_frame_age_s: 0,
    offset_s: 1000,
    camera: {
      model: "Simulated IMX296",
      width: 1456,
      height: 1088,
      exposure_ms: { min: 1, max: 1000 },
      gain: { min: 1, max: 16 },
    },
    stars: 30,
    out_of_range: [],
  },
  camera_settings: { exposure_ms: "auto", gain: "auto", sigma: 8 },
};
class LiveSource extends EventTarget {
  static instances: LiveSource[] = [];
  close = vi.fn();
  constructor(public url: string) {
    super();
    LiveSource.instances.push(this);
  }
  emit(stars: number, frame_id: number) {
    const event: LiveReport = {
      frame_id,
      time: 1768528800 + frame_id,
      pointing_seq: null,
      preview_seq: null,
      stars,
      report: { ...ORION2, name: null },
    };
    this.dispatchEvent(new MessageEvent("live", { data: JSON.stringify(event) }));
  }
}
beforeEach(() => {
  LiveSource.instances = [];
  vi.stubGlobal("EventSource", LiveSource);
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
const open = (session = SESSION) => render(<SetupPage session={session} put={vi.fn()} onBack={() => {}} />);
const change = (name: string, value: string) => fireEvent.change(screen.getByLabelText(name), { target: { value } });

describe("Setup's scope node and camera", () => {
  it("reads token_set on another device without fetching, displaying or sending a token value", async () => {
    const read = { ...SESSION, node: { ...SESSION.node!, connected: false } };
    const answers: unknown[] = [];
    const requests: unknown[] = [];
    const fetch = vi.fn(async (url: string, init?: RequestInit) => {
      requests.push({ url, init });
      answers.push(read);
      return new Response(JSON.stringify(read));
    });
    vi.stubGlobal("fetch", fetch);
    function OtherDevice() {
      const { session, put } = useSession();
      return <SetupPage session={session} put={put} onBack={() => {}} />;
    }
    render(<OtherDevice />);
    expect(await screen.findByText(/a token is set/i)).toBeTruthy();
    const token = screen.getByLabelText("Token") as HTMLInputElement;
    expect(token.type).toBe("password");
    expect(token.value).toBe("");
    expect(JSON.stringify({ requests, answers })).not.toContain("test-token");
    for (const answer of answers) expect(answer).not.toHaveProperty("node.token");
    expect(fetch).toHaveBeenCalledExactlyOnceWith("/api/session");
  });

  it("shows connection, model and stars from the session and follows every live event and session update", () => {
    const view = open();
    expect(screen.getByText("Connected")).toBeTruthy();
    expect(screen.getByText("Simulated IMX296")).toBeTruthy();
    expect(screen.getByText("30 stars")).toBeTruthy();
    const stream = LiveSource.instances[0]!;
    expect(stream.url).toBe("/api/live");
    act(() => stream.emit(34, 41));
    expect(screen.getByText("34 stars")).toBeTruthy();
    act(() => stream.emit(27, 42));
    expect(screen.getByText("27 stars")).toBeTruthy();
    const disconnected = {
      ...SESSION,
      node: { ...SESSION.node!, connected: false, stars: 27, camera: { ...SESSION.node!.camera!, model: "IMX296" } },
    };
    view.rerender(<SetupPage session={disconnected} put={vi.fn()} onBack={() => {}} />);
    expect(screen.getByText("Disconnected")).toBeTruthy();
    expect(screen.getByText("IMX296")).toBeTruthy();
    expect(stream.close).toHaveBeenCalledTimes(1);
  });

  it.each([
    ["exposure_ms", "Exposure", 2000, 1, 1000, "ms"],
    ["gain", "Gain", 20, 1, 16, ""],
  ] as const)("explains stored %s outside the camera's range", (setting, label, stored, min, max, unit) => {
    open({ ...SESSION, node: { ...SESSION.node!, out_of_range: [{ setting, stored, min, max }] } });
    const warning = screen.getByRole("alert").textContent!;
    expect(warning).toContain(label);
    expect(warning).toContain(String(stored));
    expect(warning).toContain(`${min}–${max}${unit ? ` ${unit}` : ""}`);
    expect(warning).toMatch(/outside the camera's range/);
  });

  it.each([
    ["250", "2", 250, 2],
    ["auto", "2", "auto", 2],
    ["250", "auto", 250, "auto"],
    ["auto", "auto", "auto", "auto"],
  ])("saves exposure %s and gain %s with pairing and sigma in one PUT", async (exposure, gain, e, g) => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(SESSION)));
    vi.stubGlobal("fetch", fetch);
    open();
    change("Scope node address", "finder.local:50051");
    change("Token", "test-token");
    change("Exposure", String(exposure));
    change("Gain", String(gain));
    change("Detection threshold", "6");
    act(() => LiveSource.instances[0]!.emit(33, 42));
    fireEvent.click(screen.getByRole("button", { name: "Save scope node and camera" }));
    expect(await screen.findByText("Scope node and camera saved.")).toBeTruthy();
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, init] = fetch.mock.calls[0]! as unknown as [string, RequestInit];
    expect(url).toBe("/api/session");
    expect(init.method).toBe("PUT");
    expect(init.headers).toEqual({ "Content-Type": "application/json" });
    expect(JSON.parse(String(init.body))).toEqual({
      node: { address: "finder.local:50051", token: "test-token" },
      camera_settings: { exposure_ms: e, gain: g, sigma: 6 },
    });
    expect((screen.getByLabelText("Token") as HTMLInputElement).value).toBe("");
  });

  it("keeps the set token when saved empty and uses cedar's 8 sigma for automatic threshold", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify(SESSION)));
    vi.stubGlobal("fetch", fetch);
    open();
    change("Detection threshold", "auto");
    fireEvent.click(screen.getByRole("button", { name: "Save scope node and camera" }));
    await screen.findByText("Scope node and camera saved.");
    const [, init] = fetch.mock.calls[0]! as unknown as [string, RequestInit];
    expect(JSON.parse(String(init.body))).toEqual({
      node: { address: "scope:50051" },
      camera_settings: { exposure_ms: "auto", gain: "auto", sigma: 8 },
    });
  });

  it("shows a save refusal without losing the entered values", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ error: "gain outside range 1–16" }), { status: 400 })),
    );
    open();
    change("Gain", "20");
    fireEvent.click(screen.getByRole("button", { name: "Save scope node and camera" }));
    expect(await screen.findByText("gain outside range 1–16")).toBeTruthy();
    expect((screen.getByLabelText("Gain") as HTMLInputElement).value).toBe("20");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save scope node and camera" }).hasAttribute("disabled")).toBe(false),
    );
  });
});
