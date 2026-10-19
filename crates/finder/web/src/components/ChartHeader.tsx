// The chart's margin header: where the frame points, set the way an atlas titles a chart.

import type { ReactNode } from "react";
import type { SolveReport } from "../api";
import { arcsec, field, ms } from "../format";
import { Alert, Check } from "./Icons";

/** Digits large, unit marks (h m s, ° ′ ″) small: `05h 40m 46s`. */
function Coordinate({ text }: { text: string }) {
  const parts = text.match(/[−+-]?\d+(?:\.\d+)?|[^\d\s−+-]+|\s+/g) ?? [text];
  return (
    <span className="whitespace-nowrap">
      {parts.map((p, i) =>
        /\d/.test(p) ? (
          <span key={i}>{p}</span>
        ) : /\s/.test(p) ? (
          <span key={i} className="inline-block w-[0.18em]" />
        ) : (
          <span key={i} className="ml-[0.04em] text-[0.5em] font-medium text-ink-2 align-[0.62em]">
            {p}
          </span>
        ),
      )}
    </span>
  );
}

function Frame({ children }: { children: ReactNode }) {
  // The atlas's double rule around a chart's title block.
  return (
    <section aria-labelledby="pointing-title" className="border border-ink p-[3px]">
      <div className="border border-ink px-4 pt-3 pb-4 sm:px-5">{children}</div>
    </section>
  );
}

interface HeaderProps {
  report: SolveReport;
  label: string;
  /** The threshold the frame was solved at, and a way to solve it again at another. */
  sigma: number;
  onRetry: (sigma: number) => void;
}

export function ChartHeader({ report, label, sigma, onRetry }: HeaderProps) {
  const p = report.pointing;
  const t = report.timing_ms;
  if (!p) {
    const lower = sigma > 5 ? 5 : sigma > 3 ? 3 : null;
    return (
      <Frame>
        <h1 id="pointing-title" className="chart-title text-[1.05rem]">
          No match
        </h1>
        <p className="mt-2 flex gap-2 text-[0.9375rem] text-ink">
          <span className="mt-0.5 shrink-0 text-alert">
            <Alert size={18} />
          </span>
          <span>{report.hint}</span>
        </p>
        <p className="mt-2 text-sm text-ink-2">
          {label}: {report.detected.length} stars detected at {sigma}σ, {report.used} tried · {ms(t.detect + t.solve)}
        </p>
        {lower !== null && (
          <button type="button" className="button mt-3" onClick={() => onRetry(lower)}>
            Try again at {lower}σ
          </button>
        )}
      </Frame>
    );
  }
  return (
    <Frame>
      <div className="flex items-baseline justify-between gap-3">
        <h1 id="pointing-title" className="chart-title text-[1.05rem] sm:text-[1.2rem]">
          {p.constellation.name}
        </h1>
        <span className="text-xs font-semibold tracking-[0.18em] text-ink-2">J2000</span>
      </div>
      <dl className="mt-2 grid grid-cols-[2.6rem_1fr] items-baseline gap-x-2 leading-none">
        <dt className="text-sm font-semibold tracking-[0.12em] text-ink-2 uppercase">RA</dt>
        <dd className="text-[2.35rem] font-[680] tracking-[-0.02em] sm:text-[2.75rem]">
          <Coordinate text={p.ra_text} />
        </dd>
        <dt className="mt-2 text-sm font-semibold tracking-[0.12em] text-ink-2 uppercase">Dec</dt>
        <dd className="mt-2 text-[2.35rem] font-[680] tracking-[-0.02em] sm:text-[2.75rem]">
          <Coordinate text={p.dec_text} />
        </dd>
      </dl>
      <p className="mt-4 border-t border-rule pt-3 text-[0.9375rem]">
        Field {field(p.fov_deg, p.fov_vertical_deg)} · roll {Math.round(p.roll_deg)}° · {p.arcsec_per_px.toFixed(0)}″/px
      </p>
      <p className="mt-1 flex items-center gap-1.5 text-[0.9375rem] text-ink-2">
        <span className="text-nebula">
          <Check size={18} />
        </span>
        Solved in {ms(t.detect + t.solve)} · {p.matched} of {report.detected.length} stars matched ·{" "}
        {arcsec(p.rmse_arcsec)} RMS
      </p>
    </Frame>
  );
}

/** The header while a frame is being solved, or before the first one. */
export function ChartHeaderPlaceholder({ busy, label }: { busy: boolean; label: string | null }) {
  return (
    <Frame>
      <h1 id="pointing-title" className="chart-title text-[1.05rem]">
        {busy ? "Charting…" : "No frame yet"}
      </h1>
      {busy ? (
        <div className="mt-3 space-y-3" aria-hidden="true">
          <div className="skeleton h-9 w-4/5" />
          <div className="skeleton h-9 w-3/4" />
          <div className="skeleton h-4 w-2/3" />
        </div>
      ) : (
        <p className="mt-2 text-[0.9375rem] text-ink-2">
          Choose a frame and Finder charts it: where it points, what is in it, and what to look at next.
        </p>
      )}
      {busy && label && <p className="mt-3 text-sm text-ink-2">{label}</p>}
    </Frame>
  );
}
