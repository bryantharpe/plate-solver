// Number and angle formatting for the page.

export const deg = (v: number, digits = 1) => `${v.toFixed(digits)}°`;

export function arcsec(v: number | null | undefined): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  return v >= 100 ? `${(v / 60).toFixed(1)}′` : `${Math.round(v)}″`;
}

export function ms(v: number): string {
  if (v < 1) return `${v.toFixed(2)} ms`;
  if (v < 10) return `${v.toFixed(1)} ms`;
  return `${Math.round(v)} ms`;
}

export const mag = (v: number | null | undefined) =>
  v === null || v === undefined || !Number.isFinite(v) ? "—" : v.toFixed(1);

/** Field of view as `10.2° × 6.8°`. */
export const field = (w: number, h: number) => `${w.toFixed(1)}° × ${h.toFixed(1)}°`;

const POINTS = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];

export const compass = (azimuth: number) => POINTS[Math.round((((azimuth % 360) + 360) % 360) / 22.5) % 16] ?? "N";

/** `M42 · Great Orion Nebula`, or the designation when there is no common name. */
export function objectTitle(o: { id: string; name: string | null; designation: string | null }): string {
  return o.name ?? o.designation ?? o.id;
}

/** A readable angular length for a scale bar spanning about `targetPx` pixels. */
export function scaleBar(arcsecPerPx: number, targetPx: number): { px: number; label: string } {
  const steps = [5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 18000, 36000];
  const want = arcsecPerPx * targetPx;
  const step = steps.reduce((best, s) => (Math.abs(s - want) < Math.abs(best - want) ? s : best));
  const label = step >= 3600 ? `${step / 3600}°` : step >= 60 ? `${step / 60}′` : `${step}″`;
  return { px: step / arcsecPerPx, label };
}
