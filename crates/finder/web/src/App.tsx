import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { getHealth, type FrameSource, type Health, type SolveSettings } from "./api";
import { Chart, DEFAULT_LAYERS, type Layers } from "./components/Chart";
import { ChartHeader, ChartHeaderPlaceholder } from "./components/ChartHeader";
import { FieldList, FieldRoster } from "./components/FieldList";
import { FramePicker, SampleGrid } from "./components/FramePicker";
import { Upload } from "./components/Icons";
import { NextTargets } from "./components/NextTargets";
import { SiteDialog } from "./components/SiteDialog";
import { SurveyView } from "./components/SurveyView";
import { TopBar } from "./components/TopBar";
import { useLastSample, useNight, useSite } from "./settings";
import { useSolve } from "./useSolve";

const FIRST_SAMPLE = "orion2.jpg";

export function App() {
  const [night, setNight] = useNight();
  const [site, setSite] = useSite();
  const [lastSample, setLastSample] = useLastSample();
  const { state, run } = useSolve();
  const [picker, setPicker] = useState(false);
  const [siteOpen, setSiteOpen] = useState(false);
  const [layers, setLayers] = useState<Layers>(DEFAULT_LAYERS);
  const [selected, setSelected] = useState<string | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [settings, setSettings] = useState<SolveSettings>({});

  useEffect(() => {
    getHealth()
      .then(setHealth)
      .catch(() => setHealth(null));
  }, []);

  // Come back to the last sample charted.
  useEffect(() => {
    if (lastSample) run({ kind: "sample", name: lastSample }, settings);
    // Only on first load: `run` and `lastSample` are read once, deliberately.
  }, []);

  const open = (source: FrameSource, solveWith: SolveSettings = settings) => {
    setSelected(null);
    if (source.kind === "sample") setLastSample(source.name);
    run(source, solveWith);
  };

  const report = state.phase === "done" ? state.report : null;
  const reference = useMemo(() => {
    const objects = report?.objects ?? [];
    return objects.find((o) => o.id === selected) ?? objects.find((o) => o.inside) ?? objects[0] ?? null;
  }, [report, selected]);

  const source = state.phase === "idle" ? null : state.source;
  const label = state.phase === "idle" ? null : state.label;
  const busy = state.phase === "solving";

  // The frame's shape sets how wide the page is on a big screen (see .page-width); keep
  // the last one while the next frame solves.
  const [aspect, setAspect] = useState(4 / 3);
  useEffect(() => {
    if (report) setAspect(report.width / report.height);
  }, [report]);

  return (
    <div className="contents" style={{ "--chart-aspect": aspect } as CSSProperties}>
      <TopBar
        source={source}
        label={label}
        site={site}
        night={night}
        onFrame={() => setPicker(true)}
        onSite={() => setSiteOpen(true)}
        onNight={setNight}
      />
      {/*
        One column on a phone, in reading order: where it points and what is in it, the
        chart, then the rest. On a wide screen the chart takes the right column and stays
        in view while the left one scrolls.
      */}
      <main className="page-width grid gap-x-10 gap-y-4 px-4 pt-3 pb-16 sm:gap-y-5 sm:px-5 sm:pt-4 lg:grid-cols-[minmax(22rem,30rem)_minmax(0,1fr)] lg:grid-rows-[auto_auto_1fr_auto] lg:gap-y-8 lg:px-10 lg:pt-8">
        {state.phase !== "idle" && (
          <div className="space-y-3 sm:space-y-4 lg:col-start-1 lg:row-start-1">
            {report ? (
              <ChartHeader
                report={report}
                label={label ?? ""}
                sigma={settings.sigma ?? 8}
                onRetry={(sigma) => {
                  const next = { ...settings, sigma };
                  setSettings(next);
                  if (source) open(source, next);
                }}
              />
            ) : (
              <ChartHeaderPlaceholder busy={busy} label={label} />
            )}
            {report?.pointing && <FieldRoster report={report} />}
            {state.phase === "error" && (
              <ErrorNote message={state.message} onRetry={() => run(state.source, settings)} />
            )}
          </div>
        )}

        {state.phase === "idle" && (
          <div className="lg:col-span-2 lg:row-start-1">
            <FirstRun onPick={(name) => open({ kind: "sample", name })} onUpload={() => setPicker(true)} />
          </div>
        )}

        {state.phase !== "idle" && (
          <div className="lg:col-start-2 lg:row-span-3 lg:row-start-1">
            <div className="lg:sticky lg:top-24">
              <Chart
                report={report}
                imageUrl={state.imageUrl}
                label={state.label}
                busy={busy}
                layers={layers}
                onLayers={setLayers}
              />
            </div>
          </div>
        )}

        {report?.pointing && (
          <div className="pt-3 lg:col-start-1 lg:row-start-2 lg:pt-0">
            <FieldList report={report} selected={reference?.id ?? null} onSelect={setSelected} />
          </div>
        )}

        {report?.pointing && (
          <div className="space-y-10 pt-5 lg:col-start-1 lg:row-start-3 lg:pt-0">
            <NextTargets reference={reference} nearby={report.nearby} site={site} onSetSite={() => setSiteOpen(true)} />
            <SurveyView pointing={report.pointing} />
          </div>
        )}

        <footer className="pt-5 text-sm text-ink-2 lg:col-span-2 lg:row-start-4 lg:pt-0">
          {health
            ? `Database: ${health.database.catalog}, ${health.database.stars.toLocaleString("en-US")} stars, fields ${health.database.min_fov}–${health.database.max_fov}° · finder ${health.version}`
            : "Finder"}
        </footer>
      </main>

      <FramePicker
        open={picker}
        current={source?.kind === "sample" ? source.name : null}
        settings={settings}
        onSettings={setSettings}
        onClose={() => setPicker(false)}
        onPick={(source) => open(source)}
      />
      <SiteDialog open={siteOpen} site={site} onClose={() => setSiteOpen(false)} onSave={setSite} />
    </div>
  );
}

function FirstRun({ onPick, onUpload }: { onPick: (name: string) => void; onUpload: () => void }) {
  return (
    <section aria-labelledby="first-title" className="space-y-5 border border-ink p-[3px]">
      <div className="space-y-5 border border-ink p-4 sm:p-6">
        <h1 id="first-title" className="chart-title text-[1.3rem]">
          Chart a frame
        </h1>
        <p className="max-w-[60ch] text-ink-2">
          Finder finds the stars in a frame, works out where it points with no hint, and draws the sky's grid, the
          matched catalog stars and the Messier objects over it. Start with one of the sample frames, or your own.
        </p>
        <div className="flex flex-wrap gap-3">
          <button type="button" className="button button-primary" onClick={() => onPick(FIRST_SAMPLE)}>
            Chart the Orion sample
          </button>
          <button type="button" className="button" onClick={onUpload}>
            <Upload size={18} />
            Open your own frame
          </button>
        </div>
        <SampleGrid current={null} onPick={onPick} heading="h2" />
      </div>
    </section>
  );
}

function ErrorNote({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div role="alert" className="space-y-3 border-t-2 border-alert pt-3">
      <p className="font-semibold text-alert">That frame did not go through.</p>
      <p className="text-ink-2">{message}</p>
      <button type="button" className="button" onClick={onRetry}>
        Try again
      </button>
    </div>
  );
}
