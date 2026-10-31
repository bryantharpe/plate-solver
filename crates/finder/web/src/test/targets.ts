// UI oracles: OpenNGC's pinned rows, the saved orion2 report, and explicit session
// answers. Display tests consume these numbers; they do not recalculate guidance.
import type { Target } from "../api";
import type { ObservingSession } from "../useSession";
import { ORION2 } from "./fixtures";

export const M31: Target = {
  id: "NGC 224", messier: "M31", common_names: ["Andromeda Galaxy"], code: "G", kind: "Galaxy",
  family: "galaxy", ra_deg: 10.68479, dec_deg: 41.26906, constellation: "And",
  major_arcmin: 177.83, minor_arcmin: 69.66, pa_deg: 35, mag: 3.44,
};
export const M78: Target = {
  id: "NGC 2068", messier: "M78", common_names: [], code: "RfN", kind: "Reflection nebula",
  family: "nebula", ra_deg: 86.69092, dec_deg: 0.07931, constellation: "Ori",
  major_arcmin: 4.5, minor_arcmin: null, pa_deg: null, mag: 8,
};
export const M8: Target = {
  id: "NGC 6523", messier: "M8", common_names: ["Lagoon Nebula"], code: "Neb", kind: "Nebula",
  family: "nebula", ra_deg: 270.92196, dec_deg: -24.38017, constellation: "Sgr",
  major_arcmin: 45, minor_arcmin: 30, pa_deg: null, mag: 5.8,
};
export const M75: Target = {
  id: "NGC 6864", messier: "M75", common_names: [], code: "GCl", kind: "Globular cluster",
  family: "globular_cluster", ra_deg: 301.52017, dec_deg: -21.92222, constellation: "Sgr",
  major_arcmin: 3.6, minor_arcmin: null, pa_deg: null, mag: 8.26,
};
export const TIME = Date.parse("2026-01-16T02:00:00Z") / 1000;
const mark = ORION2.objects.find((o) => o.id === "M78")!;
export const SESSION: ObservingSession = {
  site: { lat_deg: 40, lon_deg: -75 }, min_alt_deg: 15, target: M78, time: TIME,
  pointing: { seq: 7, time: TIME - 120, ra_deg: ORION2.pointing!.ra_deg, dec_deg: ORION2.pointing!.dec_deg },
  guidance: { class: "pointable", turn_deg: 33.6, tilt_deg: 12.4,
    placement: { on_frame: true, x: mark.x, y: mark.y, axes: [mark.radius_px, mark.minor_px], angle: mark.angle_deg } },
};
