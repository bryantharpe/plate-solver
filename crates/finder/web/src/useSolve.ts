// The solve workflow: which frame is up, its image, and its report.

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, sampleUrl, solve, type FrameSource, type SolveReport, type SolveSettings } from "./api";

export type SolveState =
  | { phase: "idle" }
  | { phase: "solving"; source: FrameSource; imageUrl: string; label: string }
  | { phase: "done"; source: FrameSource; imageUrl: string; label: string; report: SolveReport }
  | { phase: "error"; source: FrameSource; imageUrl: string; label: string; message: string };

function describe(source: FrameSource): { imageUrl: string; label: string } {
  return source.kind === "sample"
    ? { imageUrl: sampleUrl(source.name), label: source.name }
    : { imageUrl: URL.createObjectURL(source.file), label: source.file.name };
}

/** Solve frames one at a time; a newer request supersedes an older one still running. */
export function useSolve() {
  const [state, setState] = useState<SolveState>({ phase: "idle" });
  const latest = useRef(0);
  const objectUrl = useRef<string | null>(null);

  const run = useCallback(async (source: FrameSource, settings: SolveSettings = {}) => {
    const ticket = ++latest.current;
    const { imageUrl, label } = describe(source);
    if (objectUrl.current) URL.revokeObjectURL(objectUrl.current);
    objectUrl.current = source.kind === "file" ? imageUrl : null;
    setState({ phase: "solving", source, imageUrl, label });
    try {
      const report = await solve(source, settings);
      if (ticket === latest.current) setState({ phase: "done", source, imageUrl, label, report });
    } catch (e) {
      if (ticket !== latest.current) return;
      const message =
        e instanceof ApiError ? e.message : "The console could not reach the solver. Is `finder` still running?";
      setState({ phase: "error", source, imageUrl, label, message });
    }
  }, []);

  useEffect(
    () => () => {
      if (objectUrl.current) URL.revokeObjectURL(objectUrl.current);
    },
    [],
  );

  return { state, run };
}
