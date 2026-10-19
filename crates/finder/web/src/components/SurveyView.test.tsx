import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ORION2 } from "../test/fixtures";
import { footprint, SurveyView } from "./SurveyView";

const pointing = ORION2.pointing!;

describe("the survey view", () => {
  it("loads nothing from the internet until asked", () => {
    render(<SurveyView pointing={pointing} />);
    expect(document.querySelector('script[src*="aladin"]')).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Show the survey image" }));
    const script = document.querySelector<HTMLScriptElement>('script[src*="aladin"]');
    expect(script?.src).toBe("https://aladin.cds.unistra.fr/AladinLite/api/v3/3.9.2-beta/aladin.js");
  });

  it("says so, and that the chart does not need it, when there is no connection", async () => {
    render(<SurveyView pointing={pointing} />);
    fireEvent.click(screen.getByRole("button", { name: "Show the survey image" }));
    document.querySelector<HTMLScriptElement>('script[src*="aladin"]')?.dispatchEvent(new Event("error"));
    expect(await screen.findByText(/No connection to the survey/)).toBeTruthy();
  });

  it("outlines the frame's field on the sky, turned by its roll", () => {
    const p = { ...pointing, ra_deg: 90, dec_deg: 0, fov_deg: 10, fov_vertical_deg: 6, north_deg: 0 };
    const corners = footprint(p);
    const ras = corners.map((c) => c[0]);
    const decs = corners.map((c) => c[1]);
    expect(Math.max(...ras) - Math.min(...ras)).toBeCloseTo(10);
    expect(Math.max(...decs) - Math.min(...decs)).toBeCloseTo(6);
    // North up: the image's right edge is west, at lower RA.
    expect(corners[1]?.[0]).toBeCloseTo(85);
    // North to the right: the frame's width lies along Dec.
    const turned = footprint({ ...p, north_deg: 90 });
    expect(Math.max(...turned.map((c) => c[1])) - Math.min(...turned.map((c) => c[1]))).toBeCloseTo(10);
  });
});
