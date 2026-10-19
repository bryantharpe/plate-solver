// The real sky at the solved position: Aladin Lite (CDS) showing a survey image of the same
// field, outlined. Online only, and loaded on request: at a dark site there is often no
// network, and the script is large.

import { useEffect, useRef, useState } from "react";
import type { Pointing } from "../api";
import { Survey } from "./Icons";

/** Aladin Lite, pinned. */
const ALADIN_SRC = "https://aladin.cds.unistra.fr/AladinLite/api/v3/3.9.2-beta/aladin.js";

interface AladinInstance {
  gotoRaDec: (ra: number, dec: number) => void;
  setFoV: (fov: number) => void;
  addOverlay: (overlay: unknown) => void;
  removeOverlays?: () => void;
}
interface AladinGlobal {
  init: Promise<void>;
  aladin: (el: HTMLElement, options: Record<string, unknown>) => AladinInstance;
  graphicOverlay: (options: Record<string, unknown>) => { add: (shape: unknown) => void };
  polygon: (points: [number, number][], options?: Record<string, unknown>) => unknown;
}
declare global {
  interface Window {
    A?: AladinGlobal;
  }
}

let loading: Promise<AladinGlobal> | null = null;
function loadAladin(): Promise<AladinGlobal> {
  loading ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = ALADIN_SRC;
    script.async = true;
    script.onload = () => (window.A ? window.A.init.then(() => resolve(window.A!)) : reject(new Error("no Aladin")));
    script.onerror = () => {
      loading = null;
      reject(new Error("offline"));
    };
    document.head.appendChild(script);
  });
  return loading;
}

/** The frame's corners on the sky, approximately: a box of the field's size, rotated by roll. */
function footprint(p: Pointing): [number, number][] {
  const w = p.fov_deg / 2;
  const h = p.fov_vertical_deg / 2;
  const north = (p.north_deg * Math.PI) / 180;
  const cosDec = Math.cos((p.dec_deg * Math.PI) / 180);
  const corners: [number, number][] = [
    [-w, -h],
    [w, -h],
    [w, h],
    [-w, h],
  ];
  return corners.map(([x, y]) => {
    // Image offsets (right, up) to sky offsets (east, north): north sits at `north_deg`
    // clockwise from up, east a quarter turn counterclockwise from it.
    const up = y * Math.cos(north) + x * Math.sin(north);
    const left = -(x * Math.cos(north) - y * Math.sin(north));
    return [p.ra_deg + left / Math.max(cosDec, 0.05), p.dec_deg + up] as [number, number];
  });
}

export function SurveyView({ pointing }: { pointing: Pointing }) {
  const [state, setState] = useState<"idle" | "loading" | "shown" | "offline">("idle");
  const holder = useRef<HTMLDivElement>(null);
  const view = useRef<{ aladin: AladinInstance; A: AladinGlobal } | null>(null);

  const draw = (A: AladinGlobal, aladin: AladinInstance, p: Pointing) => {
    aladin.removeOverlays?.();
    const overlay = A.graphicOverlay({ color: "#ff5040", lineWidth: 2 });
    aladin.addOverlay(overlay);
    overlay.add(A.polygon(footprint(p)));
    aladin.gotoRaDec(p.ra_deg, p.dec_deg);
    aladin.setFoV(Math.max(p.fov_deg, p.fov_vertical_deg) * 1.6);
  };

  const latest = useRef(pointing);
  useEffect(() => {
    latest.current = pointing;
    const v = view.current;
    if (v) draw(v.A, v.aladin, pointing);
  }, [pointing]);

  const show = () => {
    setState("loading");
    loadAladin()
      .then((A) => {
        if (!holder.current) return;
        const aladin = A.aladin(holder.current, {
          survey: "P/DSS2/color",
          fov: pointing.fov_deg * 1.6,
          target: `${pointing.ra_deg} ${pointing.dec_deg}`,
          showReticle: false,
          showLayersControl: false,
          showGotoControl: false,
          showFrame: false,
          showCooGrid: false,
        });
        view.current = { aladin, A };
        draw(A, aladin, pointing);
        // When the survey's description arrives, Aladin moves to the survey's own starting
        // field of view; put this frame's back once it has.
        for (const delay of [800, 2500]) {
          window.setTimeout(() => view.current?.aladin === aladin && draw(A, aladin, latest.current), delay);
        }
        setState("shown");
      })
      .catch(() => setState("offline"));
  };

  return (
    <section aria-labelledby="survey-title" className="space-y-3">
      <h2 id="survey-title" className="section-title">
        The same field in a sky survey
      </h2>
      <div
        ref={holder}
        className={`photo relative aspect-[4/3] w-full overflow-hidden border border-ink bg-photo ${state === "idle" || state === "offline" ? "hidden" : ""}`}
      />
      {state !== "shown" && (
        <div className="flex flex-wrap items-center gap-3">
          <button type="button" className="button" onClick={show} disabled={state === "loading"}>
            <Survey size={18} />
            {state === "loading" ? "Loading the survey…" : "Show the survey image"}
          </button>
          <p className="quiet">
            {state === "offline"
              ? "No connection to the survey (CDS). It needs the internet; the chart above does not."
              : "Downloads Aladin Lite and a DSS image from CDS; needs the internet."}
          </p>
        </div>
      )}
    </section>
  );
}
