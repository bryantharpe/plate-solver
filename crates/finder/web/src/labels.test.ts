import { describe, expect, it } from "vitest";
import type { ObjectMark, SolveReport, StarMark } from "./api";
import { furnitureGeometry } from "./components/Chart";
import {
  boxesOverlap,
  layoutLabels,
  outline,
  reach,
  ringRadius,
  type Box,
  type LabelInput,
  type LabelSpot,
} from "./labels";
import { ORION2, VIRGO } from "./test/fixtures";

/** Archivo's widths run a little over half the size; a generous estimate keeps tests honest. */
const measure = (text: string, size: number) => text.length * size * 0.62;

/** The chart's input at a given on-screen width (the frame plus its 30px paper margins). */
function input(report: SolveReport, cssWidth: number): LabelInput {
  const u = report.width / (cssWidth - 60);
  return {
    W: report.width,
    H: report.height,
    u,
    stars: report.stars,
    objects: report.objects,
    detected: [],
    furniture: furnitureGeometry(report, u).boxes,
    measure,
  };
}

const boxOf = (spot: LabelSpot): Box => {
  expect(spot.box).toBeDefined();
  return spot.box as Box;
};

/** The distance from a point to a box (zero inside it). */
const distance = (b: Box, x: number, y: number) =>
  Math.hypot(Math.max(b.x0 - x, 0, x - b.x1), Math.max(b.y0 - y, 0, y - b.y1));

function check(name: string, report: SolveReport, cssWidth: number) {
  const chart = input(report, cssWidth);
  const { u, W, H } = chart;
  const spots = layoutLabels(chart);
  const placed = [...spots.entries()];

  describe(`${name} at ${cssWidth}px`, () => {
    it("names every Messier object on the chart", () => {
      for (const o of report.objects) expect(spots.has(`o:${o.id}`), o.id).toBe(true);
    });

    it("keeps every label inside the frame and off the furniture", () => {
      for (const [key, spot] of placed) {
        const b = boxOf(spot);
        expect(b.x0 >= 0 && b.y0 >= 0 && b.x1 <= W && b.y1 <= H, key).toBe(true);
        for (const f of chart.furniture) expect(boxesOverlap(b, f), `${key} on the furniture`).toBe(false);
      }
    });

    it("never prints one label over another", () => {
      for (const [i, [a, sa]] of placed.entries()) {
        for (const [b, sb] of placed.slice(i + 1)) {
          expect(boxesOverlap(boxOf(sa), boxOf(sb)), `${a} and ${b}`).toBe(false);
        }
      }
    });

    it("never prints a label across its own symbol", () => {
      for (const o of report.objects) {
        const spot = spots.get(`o:${o.id}`);
        if (!spot) continue;
        const b = boxOf(spot);
        for (const [x, y] of outline(o, u)) expect(distance(b, x, y), `${o.id} crosses its outline`).toBeGreaterThan(0);
      }
      for (const s of report.stars) {
        const spot = spots.get(`s:${s.id}`);
        if (spot) expect(distance(boxOf(spot), s.x, s.y)).toBeGreaterThan(ringRadius(s) * u);
      }
    });

    it("draws a leader only from a label set off its mark, starting outside the mark", () => {
      for (const o of report.objects) {
        const spot = spots.get(`o:${o.id}`);
        if (!spot?.leader) continue;
        const { x1, y1, x2, y2 } = spot.leader;
        const along = Math.hypot(x1 - o.x, y1 - o.y);
        const [dx, dy] = [(x1 - o.x) / along, (y1 - o.y) / along];
        expect(along).toBeGreaterThan(reach(o, u, dx, dy));
        expect(Math.hypot(x2 - x1, y2 - y1)).toBeGreaterThan(0);
      }
    });
  });
  return { spots, chart };
}

describe("label layout on the Orion frame", () => {
  // The user's phone (412 wide, 16px gutters) and a laptop.
  for (const width of [380, 800]) {
    const { spots, chart } = check("orion2", ORION2, width);
    const { u } = chart;

    it(`at ${width}px names the belt stars and Saiph clear of every other ring`, () => {
      const named = ORION2.stars.filter((s) => s.name);
      for (const name of ["Alnitak", "Alnilam", "Mintaka", "Saiph"]) {
        const star = named.find((s) => s.name === name) as StarMark;
        const spot = spots.get(`s:${star.id}`);
        expect(spot, name).toBeDefined();
        const b = boxOf(spot as LabelSpot);
        for (const other of ORION2.stars) {
          if (other.id === star.id) continue;
          expect(distance(b, other.x, other.y), `${name} over ${other.id}`).toBeGreaterThan(ringRadius(other) * u);
        }
      }
    });

    it(`at ${width}px keeps object labels off the other stars' rings`, () => {
      for (const o of ORION2.objects) {
        const b = boxOf(spots.get(`o:${o.id}`) as LabelSpot);
        for (const s of ORION2.stars) {
          expect(distance(b, s.x, s.y), `${o.id} over ${s.id}`).toBeGreaterThan(ringRadius(s) * u);
        }
      }
    });
  }
});

describe("label layout on the crowded Virgo frame", () => {
  // Fifteen galaxies: crowded at phone width, so this checks the rules that always hold.
  check("virgo", VIRGO, 380);
  check("virgo", VIRGO, 960);
});

describe("label layout rules", () => {
  const star = (id: string, x: number, y: number, name: string | null = null): StarMark => ({
    id,
    x,
    y,
    name,
    mag: 2,
    matched: true,
    error_px: 0.2,
  });
  const nebula = (id: string, x: number, y: number, radius_px: number): ObjectMark => ({
    id,
    designation: null,
    name: null,
    code: "Neb",
    kind: "Nebula",
    family: "nebula",
    ra_deg: 0,
    dec_deg: 0,
    constellation: { abbr: "Ori", name: "Orion" },
    major_arcmin: null,
    minor_arcmin: null,
    pa_deg: null,
    mag: 5,
    x,
    y,
    radius_px,
    minor_px: radius_px,
    angle_deg: 0,
    separation_deg: 0,
    inside: true,
  });
  const base = { W: 400, H: 300, u: 1, detected: [] as [number, number][], furniture: [], measure };

  it("drops a star's name when every spot around it is covered", () => {
    const crowd: StarMark[] = [star("a", 200, 150, "Lonely")];
    for (let x = 120; x <= 280; x += 9) for (let y = 100; y <= 200; y += 9) crowd.push(star(`${x},${y}`, x, y));
    const spots = layoutLabels({ ...base, stars: crowd, objects: [] });
    expect(spots.has("s:a")).toBe(false);
  });

  it("sets a label off on a leader when the spots beside the mark are taken", () => {
    const ring: StarMark[] = [];
    for (let a = 0; a < 360; a += 20) {
      const t = (a * Math.PI) / 180;
      ring.push(star(`r${a}`, 200 + 24 * Math.cos(t), 150 + 24 * Math.sin(t)));
    }
    const spots = layoutLabels({ ...base, stars: [star("c", 200, 150, "Centre"), ...ring], objects: [] });
    expect(spots.get("s:c")?.leader).toBeDefined();
  });

  it("names an object whose centre lies off the frame inside the frame", () => {
    // Off the top-left corner, only its corner showing: the label is pulled into the frame.
    for (const o of [nebula("M99", -6, 150, 12), nebula("M98", -10, -10, 14)]) {
      const spot = layoutLabels({ ...base, stars: [], objects: [o] }).get(`o:${o.id}`) as LabelSpot;
      const b = boxOf(spot);
      expect(b.x0 >= 0 && b.y0 >= 0 && b.x1 <= base.W && b.y1 <= base.H, o.id).toBe(true);
    }
  });

  it("measures a symbol's reach by its shape", () => {
    const square = nebula("M1", 0, 0, 10);
    expect(reach(square, 1, 1, 0)).toBeCloseTo(10);
    expect(reach(square, 1, Math.SQRT1_2, Math.SQRT1_2)).toBeCloseTo(10 * Math.SQRT2);
    // A galaxy 20 by 6 with its major axis up the image.
    const galaxy: ObjectMark = { ...square, family: "galaxy", radius_px: 20, minor_px: 6, angle_deg: 0 };
    expect(reach(galaxy, 1, 0, -1)).toBeCloseTo(20);
    expect(reach(galaxy, 1, 1, 0)).toBeCloseTo(6);
    // A small object is drawn no smaller than a mark.
    expect(reach({ ...square, radius_px: 1 }, 1, 1, 0)).toBeCloseTo(9);
  });
});
