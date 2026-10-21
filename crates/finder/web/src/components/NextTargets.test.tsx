import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ORION2, SIMILAR_M42 } from "../test/fixtures";
import { NextTargets } from "./NextTargets";

afterEach(() => vi.unstubAllGlobals());

const M42 = ORION2.objects[0]!;

describe("what to look at next", () => {
  it("suggests objects like the one in the field, with what is up at the site", async () => {
    const fetch = vi.fn(async (_url: string) => new Response(JSON.stringify(SIMILAR_M42)));
    vi.stubGlobal("fetch", fetch);
    render(<NextTargets reference={M42} nearby={ORION2.nearby} site={{ lat: 40, lon: -75 }} onSetSite={() => {}} />);
    expect(await screen.findByText("Like M42: other diffuse nebulae")).toBeTruthy();
    const url = fetch.mock.calls[0]?.[0];
    expect(url).toMatch(/^\/api\/similar\?id=M42&limit=6&lat=40&lon=-75&time=\d+/);

    const rows = screen.getAllByRole("listitem");
    const m78 = rows.find((r) => r.textContent?.startsWith("M78"));
    expect(m78?.textContent).toMatch(/47° up [NESW]{1,3}$/);
  });

  it("keeps a row for an object that is down readable, and marks it below the horizon", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify(SIMILAR_M42)));
    render(<NextTargets reference={M42} nearby={[]} site={{ lat: 40, lon: -75 }} onSetSite={() => {}} />);
    await screen.findByText("Like M42: other diffuse nebulae");
    const lagoon = screen.getAllByRole("listitem").find((r) => r.textContent?.startsWith("M8"));
    expect(lagoon?.textContent).toMatch(/below 15°$/);
    // Readable text: the row is set in ink-2 (AA on paper), only its symbol in the faint ink.
    expect(lagoon?.className).toContain("text-ink-2");
    expect(lagoon?.className).not.toContain("text-ink-3");
    expect(lagoon?.querySelector(".text-ink-3 svg")).toBeTruthy();
  });

  it("asks for the site when it has none, and lists what is just off the frame", async () => {
    vi.stubGlobal("fetch", async () => new Response(JSON.stringify({ ...SIMILAR_M42, observer: null })));
    render(<NextTargets reference={M42} nearby={ORION2.nearby} site={null} onSetSite={() => {}} />);
    expect(screen.getByRole("button", { name: "Set your site" })).toBeTruthy();
    expect(screen.getByText("Just off the frame")).toBeTruthy();
    for (const o of ORION2.nearby) expect(screen.getAllByText(o.id).length).toBeGreaterThan(0);
    expect(screen.getAllByRole("img", { name: /from up on the frame/ })).toHaveLength(ORION2.nearby.length);
  });
});
