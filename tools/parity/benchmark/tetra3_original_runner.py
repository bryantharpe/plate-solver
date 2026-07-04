#!/usr/bin/env python3
"""Runs original tetra3 (``reference-solutions/tetra3``) and prints one JSON
blob with N-iteration timing plus a representative detect/solve result.

Must be invoked with the ``tools/parity/.venv-tetra3-orig`` interpreter -
original tetra3 cannot share a Python process with cedar-solve (both install
a top-level ``tetra3`` package; see
``openspec/changes/feat-09-eval-harness/design.md``).

All warmup + N iterations run inside this ONE process invocation so
interpreter/import/database-load startup cost stays out of the timed region -
consistent with how the harness's two Rust gRPC servers are also long-lived,
not re-spawned per call.

Run:
    tools/parity/.venv-tetra3-orig/bin/python \\
        tools/parity/benchmark/tetra3_original_runner.py \\
        --mode solve --image <path> --db-path <path> \\
        --n-iterations 5 --warmup 1
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any, Dict, Optional


def _parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--mode", choices=["detect", "solve"], required=True)
    p.add_argument("--image", required=True, type=Path)
    p.add_argument("--db-path", required=True, type=Path)
    p.add_argument("--sigma", type=float, default=4.0)
    p.add_argument("--n-iterations", type=int, required=True)
    p.add_argument("--warmup", type=int, required=True)
    p.add_argument("--fov-estimate", type=float, default=None)
    p.add_argument("--fov-max-error", type=float, default=None)
    p.add_argument("--match-radius", type=float, default=0.01)
    p.add_argument("--match-threshold", type=float, default=1e-5)
    p.add_argument("--distortion", type=float, default=0.0)
    p.add_argument("--timeout-ms", type=int, default=5000)
    return p.parse_args()


def _run_detect(args: argparse.Namespace) -> Dict[str, Any]:
    import tetra3
    from PIL import Image

    wall_times = []
    centroids_yx = None
    with Image.open(args.image) as img:
        for i in range(args.warmup + args.n_iterations):
            t0 = time.perf_counter()
            centroids_yx = tetra3.get_centroids_from_image(img, sigma=args.sigma)
            wall = time.perf_counter() - t0
            if i >= args.warmup:
                wall_times.append(wall)

    centroids_list = (
        [[float(y), float(x)] for (y, x) in centroids_yx.tolist()] if centroids_yx is not None else []
    )
    return {
        "system": "tetra3_original",
        "wall_clock_s": wall_times,
        # Original tetra3's bare centroid extractor self-reports no timing.
        "algorithm_s": [None] * len(wall_times),
        "noise_estimate": None,
        "hot_pixel_count": None,
        "centroids_yx": centroids_list,
    }


def _run_solve(args: argparse.Namespace) -> Dict[str, Any]:
    import tetra3
    from PIL import Image

    t3 = tetra3.Tetra3(load_database=None)
    t3.load_database(args.db_path)

    wall_times = []
    t_extract_list = []
    t_solve_list = []
    last: Optional[Dict[str, Any]] = None
    with Image.open(args.image) as img:
        for i in range(args.warmup + args.n_iterations):
            t0 = time.perf_counter()
            sol = t3.solve_from_image(
                img,
                sigma=args.sigma,
                fov_estimate=args.fov_estimate,
                fov_max_error=args.fov_max_error,
                match_radius=args.match_radius,
                match_threshold=args.match_threshold,
                solve_timeout=args.timeout_ms,
                distortion=args.distortion,
                return_matches=True,
            )
            wall = time.perf_counter() - t0
            if i >= args.warmup:
                wall_times.append(wall)
                t_extract_list.append(sol.get("T_extract"))
                t_solve_list.append(sol.get("T_solve"))
            last = sol

    # Original tetra3 predates cedar-solve's explicit 'status' key (see
    # design.md) - RA is None exactly when no match was found, but it cannot
    # distinguish NO_MATCH/TIMEOUT/TOO_FEW, so anything unsolved is reported
    # as NO_MATCH. It also has no 'P90E'/'MAXE' keys (older API, RMSE only).
    solved = last is not None and last.get("RA") is not None
    status = "MATCH_FOUND" if solved else "NO_MATCH"
    matched_cat_ids = (
        [int(x) for x in last["matched_catID"]]
        if last is not None and last.get("matched_catID") is not None
        else None
    )
    return {
        "system": "tetra3_original",
        "wall_clock_s": wall_times,
        "t_extract_ms": t_extract_list,
        "t_solve_ms": t_solve_list,
        "status": status,
        "ra_deg": last.get("RA") if last else None,
        "dec_deg": last.get("Dec") if last else None,
        "roll_deg": last.get("Roll") if last else None,
        "fov_deg": last.get("FOV") if last else None,
        "matches": last.get("Matches") if last else None,
        "matched_cat_ids": matched_cat_ids,
        "rmse_arcsec": last.get("RMSE") if last else None,
        "p90e_arcsec": last.get("P90E") if last else None,
        "maxe_arcsec": last.get("MAXE") if last else None,
    }


def main() -> int:
    args = _parse_args()
    if not args.image.is_file():
        print(f"FAIL: image not found at {args.image}", file=sys.stderr)
        return 1
    if not args.db_path.is_file():
        print(f"FAIL: database not found at {args.db_path}", file=sys.stderr)
        return 1

    payload = _run_detect(args) if args.mode == "detect" else _run_solve(args)
    print(json.dumps(payload))
    return 0


if __name__ == "__main__":
    sys.exit(main())
