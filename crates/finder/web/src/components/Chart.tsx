// The chart: the frame with the solution printed over it the way a star atlas prints a
// chart. The RA/Dec grid is labelled in the paper margin, matched stars are ringed and
// named, and deep-sky objects are drawn in the atlas's symbols. Everything is drawn in the
// frame's own pixel coordinates, so each mark sits where the server put it.

import { useId, useLayoutEffect, useRef, useState, type CSSProperties, type RefObject } from "react";
import type { Edge, GridLine, ObjectMark, SolveReport, StarMark } from "../api";
import { mag, objectTitle, scaleBar } from "../format";
import { FamilyShape } from "./Symbol";

export interface Layers {
  grid: boolean;
  catalog: boolean;
  objects: boolean;
  detected: boolean;
}

type Picked = { kind: "star"; star: StarMark } | { kind: "object"; object: ObjectMark } | null;

interface Props {
  report: SolveReport | null;
  imageUrl: string;
  label: string;
  busy: boolean;
  layers: Layers;
  onLayers: (layers: Layers) => void;
}

/** Paper margin around the frame for the grid's labels, in CSS pixels. */
const MARGIN = 30;
/** Labels' on-screen size, in CSS pixels. */
const TEXT = 12;

function useWidth(ref: RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    setWidth(el.getBoundingClientRect().width);
    const observer = new ResizeObserver(([entry]) => entry && setWidth(entry.contentRect.width));
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

export function Chart({ report, imageUrl, label, busy, layers, onLayers }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const width = useWidth(box);
  const [picked, setPicked] = useState<Picked>(null);
  const clip = useId();

  if (!report) {
    return (
      <figure className="chart" aria-busy={busy}>
        <div ref={box}>
          <div className="relative border border-ink" style={{ margin: MARGIN }}>
            <img src={imageUrl} alt={`Frame ${label}`} className="photo block w-full opacity-60" />
            {busy && (
              <p className="absolute inset-x-0 bottom-0 m-0 bg-photo/80 px-3 py-2 text-sm font-medium text-on-photo">
                Charting {label}…
              </p>
            )}
          </div>
        </div>
      </figure>
    );
  }

  const W = report.width;
  const H = report.height;
  // Image pixels per CSS pixel, so strokes and labels keep their on-screen size.
  const u = width > 2 * MARGIN + 40 ? W / (width - 2 * MARGIN) : W / 320;
  const m = MARGIN * u;
  const solved = report.pointing !== null;
  // With nothing solved, the detected stars are the whole story: show them.
  const showDetected = layers.detected || !solved;
  const key = `${report.name}:${report.pointing?.ra_deg ?? "none"}`;
  const spots = layoutLabels(report, layers, W, H, u);

  return (
    <figure
      className="chart lg:mx-auto lg:max-w-[calc((100dvh-11rem)*var(--chart-aspect))]"
      style={{ "--chart-aspect": W / H } as CSSProperties}
    >
      <div ref={box}>
        <svg
          key={key}
          viewBox={`${-m} ${-m} ${W + 2 * m} ${H + 2 * m}`}
          className="block h-auto w-full select-none"
          role="img"
          aria-label={solved ? `Chart of ${label} with the solution drawn on it` : `Frame ${label}`}
          onClick={() => setPicked(null)}
        >
          <defs>
            <clipPath id={clip}>
              <rect width={W} height={H} />
            </clipPath>
          </defs>
          <rect
            x={-m + 1.5 * u}
            y={-m + 1.5 * u}
            width={W + 2 * m - 3 * u}
            height={H + 2 * m - 3 * u}
            fill="none"
            stroke="var(--color-ink)"
            strokeWidth={u}
          />
          <image href={imageUrl} width={W} height={H} preserveAspectRatio="none" className="photo" />
          <rect width={W} height={H} fill="none" stroke="var(--color-ink)" strokeWidth={1.4 * u} />

          {layers.grid && solved && <Grid lines={report.grid} W={W} H={H} u={u} clip={clip} />}

          <g clipPath={`url(#${clip})`} fontSize={TEXT * u}>
            {showDetected &&
              report.detected.map(([x, y], i) => (
                <circle key={i} cx={x} cy={y} r={3 * u} strokeWidth={u} className="mark-detected" />
              ))}
            {layers.objects &&
              report.objects.map((o, i) => (
                <ObjectSymbol
                  key={o.id}
                  object={o}
                  u={u}
                  spot={spots.get(`o:${o.id}`)}
                  delay={450 + i * 60}
                  picked={picked?.kind === "object" && picked.object.id === o.id}
                  onPick={() => setPicked({ kind: "object", object: o })}
                />
              ))}
            {layers.catalog &&
              report.stars.map((s) => (
                <StarSymbol
                  key={s.id}
                  star={s}
                  u={u}
                  spot={spots.get(`s:${s.id}`)}
                  picked={picked?.kind === "star" && picked.star.id === s.id}
                  onPick={() => setPicked({ kind: "star", star: s })}
                />
              ))}
            {solved && <Furniture report={report} u={u} />}
          </g>
        </svg>
      </div>
      <figcaption className="mx-[30px] mt-3 min-h-11">
        {picked ? (
          <PickedLine picked={picked} />
        ) : (
          <Legend report={report} layers={{ ...layers, detected: showDetected }} fixed={!solved} onLayers={onLayers} />
        )}
      </figcaption>
    </figure>
  );
}

function toPath(points: [number, number][]): string {
  return points.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`).join("");
}

/** The grid lines, clipped to the frame, with their labels in the margin at its edges. */
function Grid({ lines, W, H, u, clip }: { lines: GridLine[]; W: number; H: number; u: number; clip: string }) {
  const gap = 6 * u;
  const labels = placeLabels(lines, W, H, u);
  return (
    <g>
      <g clipPath={`url(#${clip})`}>
        {lines.map((line, i) =>
          line.paths.map((path, j) => (
            <path
              key={`${i}.${j}`}
              d={toPath(path)}
              pathLength={1}
              className="grid-line draw"
              strokeWidth={u}
              style={{ animationDelay: `${i * 40}ms` }}
            />
          )),
        )}
      </g>
      <g fontSize={11 * u} className="ink-in" style={{ animationDelay: "250ms" }}>
        {labels.map(({ x, y, edge, text }, i) => {
          const tick =
            edge === "top"
              ? `M${x} 0V${-4 * u}`
              : edge === "bottom"
                ? `M${x} ${H}V${H + 4 * u}`
                : edge === "left"
                  ? `M0 ${y}H${-4 * u}`
                  : `M${W} ${y}H${W + 4 * u}`;
          const [tx, ty, rotate, baseline]: [number, number, number, "auto" | "hanging"] =
            edge === "top"
              ? [x, -gap, 0, "auto"]
              : edge === "bottom"
                ? [x, H + gap, 0, "hanging"]
                : edge === "left"
                  ? [-gap, y, -90, "auto"]
                  : [W + gap, y, 90, "auto"];
          return (
            <g key={i}>
              <path d={tick} stroke="var(--color-ink)" strokeWidth={u} />
              <text
                x={tx}
                y={ty}
                textAnchor="middle"
                dominantBaseline={baseline}
                transform={rotate ? `rotate(${rotate} ${tx} ${ty})` : undefined}
                className="edge-label"
              >
                {text}
              </text>
            </g>
          );
        })}
      </g>
    </g>
  );
}

/** One label per grid line end, dropping any that would crowd a neighbour or a corner. */
function placeLabels(lines: GridLine[], W: number, H: number, u: number) {
  const spacing = 52 * u;
  const corner = 16 * u;
  const all = lines.flatMap((line) => line.ticks.map((t) => ({ ...t, text: line.label })));
  const kept: { x: number; y: number; edge: Edge; text: string }[] = [];
  for (const edge of ["top", "right", "bottom", "left"] as const) {
    const along = (t: { x: number; y: number }) => (edge === "top" || edge === "bottom" ? t.x : t.y);
    const length = edge === "top" || edge === "bottom" ? W : H;
    let last = -Infinity;
    for (const t of all.filter((t) => t.edge === edge).sort((a, b) => along(a) - along(b))) {
      const at = along(t);
      if (at < corner || at > length - corner || at - last < spacing) continue;
      kept.push(t);
      last = at;
    }
  }
  return kept;
}

interface LabelSpot {
  x: number;
  y: number;
  anchor: "start" | "middle" | "end";
}

/**
 * Where each object and star label goes: the first of eight spots around its mark that
 * stays inside the frame and clear of the labels placed before it (objects first, then
 * stars brightest first) and of the chart's furniture. Objects always get a label; a star
 * whose name has no room goes without.
 */
function layoutLabels(report: SolveReport, layers: Layers, W: number, H: number, u: number) {
  type Box = { x0: number; y0: number; x1: number; y1: number };
  const taken: Box[] = [
    { x0: W - 72 * u, y0: 0, x1: W, y1: 72 * u }, // north arrow
    { x0: 0, y0: H - 40 * u, x1: 120 * u, y1: H }, // scale bar
  ];
  const spots = new Map<string, LabelSpot>();
  const clear = (b: Box) => !taken.some((t) => b.x0 < t.x1 && b.x1 > t.x0 && b.y0 < t.y1 && b.y1 > t.y0);
  const fits = (b: Box) => b.x0 >= 3 * u && b.x1 <= W - 3 * u && b.y0 >= 3 * u && b.y1 <= H - 3 * u;
  const place = (key: string, text: string, x: number, y: number, r: number, size: number, always: boolean) => {
    const w = text.length * size * 0.58;
    const d = r + 4 * u;
    const e = d * 0.72;
    const candidates: LabelSpot[] = [
      { x: x + d, y: y + size * 0.35, anchor: "start" },
      { x: x - d, y: y + size * 0.35, anchor: "end" },
      { x, y: y - d, anchor: "middle" },
      { x, y: y + d + size * 0.8, anchor: "middle" },
      { x: x + e, y: y - e, anchor: "start" },
      { x: x - e, y: y - e, anchor: "end" },
      { x: x + e, y: y + e + size * 0.7, anchor: "start" },
      { x: x - e, y: y + e + size * 0.7, anchor: "end" },
    ];
    const box = (c: LabelSpot): Box => {
      const x0 = c.anchor === "start" ? c.x : c.anchor === "end" ? c.x - w : c.x - w / 2;
      return { x0, y0: c.y - size * 0.8, x1: x0 + w, y1: c.y + size * 0.25 };
    };
    const spot =
      candidates.find((c) => fits(box(c)) && clear(box(c))) ??
      (always ? (candidates.find((c) => fits(box(c))) ?? candidates[0]) : undefined);
    if (!spot) return;
    taken.push(box(spot));
    spots.set(key, spot);
  };
  if (layers.objects) {
    for (const o of report.objects) place(`o:${o.id}`, o.id, o.x, o.y, Math.max(o.radius_px, 9 * u), 13 * u, true);
  }
  if (layers.catalog) {
    const named = report.stars.filter((s) => s.name).sort((a, b) => a.mag - b.mag);
    for (const s of named) place(`s:${s.id}`, s.name ?? "", s.x, s.y, 7.5 * u, 12 * u, false);
  }
  return spots;
}

function ObjectSymbol({
  object: o,
  u,
  spot,
  delay,
  picked,
  onPick,
}: {
  object: ObjectMark;
  u: number;
  spot: LabelSpot | undefined;
  delay: number;
  picked: boolean;
  onPick: () => void;
}) {
  const r = Math.max(o.radius_px, 9 * u);
  const galaxy = o.family === "galaxy";
  const ry = galaxy ? Math.max(Math.min(o.minor_px, r), 4 * u) : r;
  return (
    <g
      transform={`translate(${o.x} ${o.y})`}
      className={`object-${o.family} ink-in cursor-pointer`}
      style={{ animationDelay: `${delay}ms` }}
      data-picked={picked}
      onClick={(e) => {
        e.stopPropagation();
        onPick();
      }}
    >
      <title>{`${o.id} ${objectTitle(o)}`}</title>
      {galaxy ? (
        <ellipse
          rx={r}
          ry={ry}
          transform={`rotate(${o.angle_deg - 90})`}
          fill="none"
          stroke="currentColor"
          strokeWidth={1.6}
          vectorEffect="non-scaling-stroke"
          className="shape"
        />
      ) : (
        <g className="shape">
          <FamilyShape family={o.family} r={r} />
        </g>
      )}
      <circle r={Math.max(r, 18 * u)} className="hit" />
      {spot && (
        <text
          x={spot.x - o.x}
          y={spot.y - o.y}
          textAnchor={spot.anchor}
          fontSize={13 * u}
          strokeWidth={3 * u}
          className="object-label"
        >
          {o.id}
        </text>
      )}
    </g>
  );
}

function StarSymbol({
  star: s,
  u,
  spot,
  picked,
  onPick,
}: {
  star: StarMark;
  u: number;
  spot: LabelSpot | undefined;
  picked: boolean;
  onPick: () => void;
}) {
  return (
    <g
      className={`${s.matched ? "mark-matched" : "mark-expected"} ink-in`}
      style={{ animationDelay: s.matched ? "350ms" : "600ms" }}
      data-picked={picked}
      onClick={(e) => {
        e.stopPropagation();
        onPick();
      }}
    >
      <circle cx={s.x} cy={s.y} r={(s.matched ? 7.5 : 4.5) * u} strokeWidth={1.3 * u} className="ring" />
      <circle cx={s.x} cy={s.y} r={16 * u} className="hit" />
      {s.name && spot && (
        <text x={spot.x} y={spot.y} textAnchor={spot.anchor} strokeWidth={3 * u} className="mark-label">
          {s.name}
        </text>
      )}
    </g>
  );
}

/** The chart's furniture: centre reticle, north and east, and a scale bar. */
function Furniture({ report, u }: { report: SolveReport; u: number }) {
  const p = report.pointing;
  if (!p) return null;
  const W = report.width;
  const H = report.height;
  const bar = scaleBar(p.arcsec_per_px, Math.min(W, H) * 0.22);
  const pad = 16 * u;
  const arm = 24 * u;
  const ax = W - pad - arm;
  const ay = pad + arm;
  const at = (angle: number, length: number) => {
    const a = (angle * Math.PI) / 180;
    return [ax + Math.sin(a) * length, ay - Math.cos(a) * length] as const;
  };
  // On a frame of the sky, east is a quarter turn counterclockwise from north.
  const east = p.north_deg - 90;
  const [nx, ny] = at(p.north_deg, arm);
  const [ex, ey] = at(east, arm * 0.7);
  const [nlx, nly] = at(p.north_deg, arm + 9 * u);
  const [elx, ely] = at(east, arm * 0.7 + 9 * u);
  const cx = W / 2;
  const cy = H / 2;
  const by = H - pad;
  return (
    <g className="furniture ink-in" strokeWidth={1.3 * u} style={{ animationDelay: "300ms" }}>
      <path
        d={`M${cx - 11 * u} ${cy}H${cx - 4 * u}M${cx + 4 * u} ${cy}H${cx + 11 * u}M${cx} ${cy - 11 * u}V${cy - 4 * u}M${cx} ${cy + 4 * u}V${cy + 11 * u}`}
        fill="none"
      />
      <path d={`M${ax} ${ay}L${nx} ${ny}M${ax} ${ay}L${ex} ${ey}`} fill="none" />
      <text x={nlx} y={nly} textAnchor="middle" dominantBaseline="middle" fontSize={11 * u}>
        N
      </text>
      <text x={elx} y={ely} textAnchor="middle" dominantBaseline="middle" fontSize={11 * u}>
        E
      </text>
      <path d={`M${pad} ${by - 5 * u}V${by}H${pad + bar.px}V${by - 5 * u}`} fill="none" />
      <text x={pad + bar.px / 2} y={by - 8 * u} textAnchor="middle" fontSize={11 * u}>
        {bar.label}
      </text>
    </g>
  );
}

function Swatch({ layer }: { layer: keyof Layers }) {
  return (
    <svg viewBox="-8 -8 16 16" width="14" height="14" aria-hidden="true" className="swatch">
      {layer === "grid" && (
        <path d="M-7 -3H7M-7 3H7M-3 -7V7M3 -7V7" stroke="currentColor" strokeWidth="1.2" fill="none" />
      )}
      {layer === "catalog" && <circle r="5" stroke="currentColor" strokeWidth="1.4" fill="none" />}
      {layer === "objects" && (
        <rect x="-5" y="-5" width="10" height="10" stroke="currentColor" strokeWidth="1.4" fill="none" />
      )}
      {layer === "detected" && <circle r="2.5" fill="currentColor" />}
    </svg>
  );
}

function Legend({
  report,
  layers,
  fixed,
  onLayers,
}: {
  report: SolveReport;
  layers: Layers;
  /** Unsolved: the detected stars are all there is, so they stay on. */
  fixed: boolean;
  onLayers: (l: Layers) => void;
}) {
  const items: { key: keyof Layers; label: string; count: number }[] = [
    { key: "grid", label: "Grid", count: report.grid.length },
    { key: "catalog", label: "Stars", count: report.stars.length },
    { key: "objects", label: "Deep sky", count: report.objects.length },
    { key: "detected", label: "Detected", count: report.detected.length },
  ];
  return (
    <div className="flex flex-wrap gap-2" role="group" aria-label="Show on the chart">
      {items
        .filter((item) => item.count > 0 || item.key === "detected")
        .map((item) => (
          <button
            key={item.key}
            type="button"
            className="chip"
            aria-pressed={layers[item.key]}
            disabled={fixed && item.key === "detected"}
            onClick={() => onLayers({ ...layers, [item.key]: !layers[item.key] })}
          >
            <Swatch layer={item.key} />
            {item.label}
            <span className="chip-count">{item.count}</span>
          </button>
        ))}
    </div>
  );
}

function PickedLine({ picked }: { picked: NonNullable<Picked> }) {
  const fact = "text-ink-2";
  if (picked.kind === "star") {
    const s = picked.star;
    return (
      <p className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-[0.9375rem]">
        <strong className="font-semibold">{s.name ?? s.id}</strong>
        {s.name && <span className={fact}>{s.id}</span>}
        <span className={fact}>mag {mag(s.mag)}</span>
        <span className={fact}>
          {s.matched
            ? `Matched; the solution puts it ${s.error_px?.toFixed(2) ?? "—"} px from where it was found`
            : "In the catalog but not detected"}
        </span>
      </p>
    );
  }
  const o = picked.object;
  return (
    <p className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-[0.9375rem]">
      <strong className="font-semibold">{o.id}</strong>
      <span>{objectTitle(o)}</span>
      <span className={fact}>{o.kind}</span>
      <span className={fact}>mag {mag(o.mag)}</span>
      {o.major_arcmin !== null && <span className={fact}>{Math.round(o.major_arcmin)}′ across</span>}
    </p>
  );
}
