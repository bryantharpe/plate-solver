import { describe, expect, it } from "vitest";
import { arcsec, compass, field, mag, ms, objectTitle, scaleBar } from "./format";

describe("formatting", () => {
  it("writes residuals in arcseconds, or arcminutes when large", () => {
    expect(arcsec(12.4)).toBe("12″");
    expect(arcsec(150)).toBe("2.5′");
    expect(arcsec(null)).toBe("—");
  });

  it("writes timings to a sensible precision", () => {
    expect(ms(0.42)).toBe("0.42 ms");
    expect(ms(5.61)).toBe("5.6 ms");
    expect(ms(41.4)).toBe("41 ms");
  });

  it("writes magnitudes, fields and compass points", () => {
    expect(mag(4)).toBe("4.0");
    expect(mag(null)).toBe("—");
    expect(field(10.18, 6.83)).toBe("10.2° × 6.8°");
    expect(compass(0)).toBe("N");
    expect(compass(159.5)).toBe("SSE");
    expect(compass(-90)).toBe("W");
  });

  it("titles an object by its name, else its designation", () => {
    expect(objectTitle({ id: "M42", name: "Great Orion Nebula", designation: "NGC 1976" })).toBe("Great Orion Nebula");
    expect(objectTitle({ id: "M78", name: null, designation: "NGC 2068" })).toBe("NGC 2068");
    expect(objectTitle({ id: "M40", name: null, designation: null })).toBe("M40");
  });

  it("picks a round scale bar near the length asked for", () => {
    // 72″ a pixel over about 110 px is about 2.2°: the bar is 2°, 100 px.
    expect(scaleBar(72, 110)).toEqual({ px: 100, label: "2°" });
    // 2″ a pixel over about 20 px is 40″: the nearest round length is 30″.
    expect(scaleBar(2, 20)).toEqual({ px: 15, label: "30″" });
  });
});
