#!/usr/bin/env python3
"""Merge benchmark result JSONs into the comparison tables in RESULTS.md.

Usage:
    python tools/benchmark/report.py docs/benchmarks/results-*.json
"""

import json
import math
import statistics
import sys
from pathlib import Path


def ang_sep_deg(ra1, dec1, ra2, dec2):
    ra1, dec1, ra2, dec2 = map(math.radians, (ra1, dec1, ra2, dec2))
    c = math.sin(dec1) * math.sin(dec2) + math.cos(dec1) * math.cos(dec2) * math.cos(ra1 - ra2)
    return math.degrees(math.acos(max(-1.0, min(1.0, c))))


def main() -> None:
    by_solver = {}
    for path in sys.argv[1:]:
        for row in json.load(open(path)):
            by_solver.setdefault(row["solver"], {})[row["image"]] = row

    solvers = sorted(by_solver)
    baseline = "tetra3"
    images = sorted(by_solver[baseline])

    print("## Per-solver summary\n")
    print("| solver | solved | median wall (ms) | min–max wall (ms) | median RMSE (arcsec) |")
    print("|---|---|---|---|---|")
    for s in solvers:
        rows = [by_solver[s][i] for i in images if i in by_solver[s]]
        solved = [r for r in rows if r.get("ra") is not None]
        walls = [statistics.median(r["wall_ms"]) for r in solved]
        rmses = [r["rmse"] for r in solved if r.get("rmse") is not None]
        print(
            f"| {s} | {len(solved)}/{len(images)} "
            f"| {statistics.median(walls):.1f} "
            f"| {min(walls):.1f}–{max(walls):.1f} "
            f"| {statistics.median(rmses):.1f} |"
        )

    print("\n## Agreement with tetra3 (boresight separation / |ΔFOV|, per image)\n")
    others = [s for s in solvers if s != baseline]
    print("| image | " + " | ".join(others) + " |")
    print("|---|" + "---|" * len(others))
    worst = {s: 0.0 for s in others}
    for img in images:
        base = by_solver[baseline][img]
        cells = []
        for s in others:
            r = by_solver[s].get(img)
            if not r or r.get("ra") is None or base.get("ra") is None:
                cells.append("—")
                continue
            sep = ang_sep_deg(base["ra"], base["dec"], r["ra"], r["dec"])
            dfov = abs(base["fov"] - r["fov"])
            worst[s] = max(worst[s], sep)
            cells.append(f"{sep * 3600:.1f}″ / {dfov * 3600:.1f}″")
        print(f"| {img.removeprefix('2019-07-29T204726_').removesuffix('_Try1.jpg')} | " + " | ".join(cells) + " |")
    print()
    for s in others:
        print(f"Worst-case boresight separation vs tetra3 for {s}: {worst[s] * 3600:.1f}″")

    print("\n## Per-image wall time, median of 5 warm runs (ms)\n")
    print("| image | " + " | ".join(solvers) + " |")
    print("|---|" + "---|" * len(solvers))
    for img in images:
        cells = []
        for s in solvers:
            r = by_solver[s].get(img)
            cells.append(f"{statistics.median(r['wall_ms']):.1f}" if r and r.get("ra") is not None else "—")
        print(f"| {img.removeprefix('2019-07-29T204726_').removesuffix('_Try1.jpg')} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
