// Real reports from `finder` (POST /api/solve with a sample, GET /api/similar), saved with
// coordinates rounded to a thousandth of a pixel. orion2 solves with three Messier objects;
// virgo is a wide frame with fifteen galaxies; roof does not solve.

import type { SimilarReport, SolveReport } from "../api";
import orion2 from "./fixtures/orion2.json";
import roof from "./fixtures/roof.json";
import similarM42 from "./fixtures/similar-m42.json";
import virgo from "./fixtures/virgo.json";

export const ORION2 = orion2 as unknown as SolveReport;
export const VIRGO = virgo as unknown as SolveReport;
export const ROOF = roof as unknown as SolveReport;
/** Like M42, for 40° N 75° W at 9 pm EST on 2026-01-15: M78 up, three summer nebulae down. */
export const SIMILAR_M42 = similarM42 as unknown as SimilarReport;
