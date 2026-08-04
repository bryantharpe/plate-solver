#!/usr/bin/env python3
"""Benchmark a Python reference solver (tetra3 or cedar-solve) on the corpus.

Usage:
    python tools/benchmark/run_python.py references/tetra3 tetra3 > out.json
    python tools/benchmark/run_python.py references/cedar-solve cedar-solve > out.json

Each corpus image is solved once as warmup, then TIMED_RUNS times; wall-clock
per call (centroid extraction + solve) and the solver's own reported
T_solve/T_extract are recorded.
"""

import json
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CORPUS = sorted((REPO / "references/cedar-solve/examples/data/medium_fov").glob("2019-07-29T*.jpg"))
FOV_ESTIMATE = 11.0
TIMEOUT_MS = 10_000
TIMED_RUNS = 5


def main() -> None:
    solver_dir, label = sys.argv[1], sys.argv[2]
    sys.path.insert(0, str(REPO / solver_dir))

    # tetra3 (pinned commit) uses np.math, removed in numpy 2.x.
    import math

    import numpy

    if not hasattr(numpy, "math"):
        numpy.math = math

    from PIL import Image
    from tetra3 import Tetra3

    t3 = Tetra3("default_database")

    results = []
    for img_path in CORPUS:
        image = Image.open(img_path)
        # Warmup (JIT-less but primes caches / lazy imports).
        t3.solve_from_image(image, fov_estimate=FOV_ESTIMATE, solve_timeout=TIMEOUT_MS)

        walls, solution = [], None
        for _ in range(TIMED_RUNS):
            t0 = time.perf_counter()
            solution = t3.solve_from_image(
                image, fov_estimate=FOV_ESTIMATE, solve_timeout=TIMEOUT_MS
            )
            walls.append((time.perf_counter() - t0) * 1000.0)

        results.append(
            {
                "solver": label,
                "image": img_path.name,
                "ra": solution.get("RA"),
                "dec": solution.get("Dec"),
                "roll": solution.get("Roll"),
                "fov": solution.get("FOV"),
                "rmse": solution.get("RMSE"),
                "matches": solution.get("Matches"),
                "prob": solution.get("Prob"),
                "t_solve_reported_ms": solution.get("T_solve"),
                "t_extract_reported_ms": solution.get("T_extract"),
                "wall_ms": walls,
            }
        )
        print(f"{label} {img_path.name}: RA={solution.get('RA')}", file=sys.stderr)

    json.dump(results, sys.stdout, indent=1)


if __name__ == "__main__":
    main()
