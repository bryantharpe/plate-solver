#!/usr/bin/env python3
"""Capture nearby_stars parity fixture from the reference tetra3 implementation.

Run:  tools/parity/.venv/bin/python tools/parity/capture_nearby_stars.py
Output: ps-db/tests/fixtures/nearby_stars.json
"""

import sys, math, json
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DB_PATH = str(REPO / "reference-solutions/cedar-solve/tetra3/data/default_database.npz")
OUT = REPO / "ps-db/tests/fixtures/nearby_stars.json"

cedar_root = REPO / "reference-solutions/cedar-solve"
sys.path.insert(0, str(cedar_root))
import tetra3  # noqa: E402

t3 = tetra3.Tetra3(load_database=DB_PATH)


def query(ra_rad, dec_rad, radius_rad):
    vec = [
        math.cos(ra_rad) * math.cos(dec_rad),
        math.sin(ra_rad) * math.cos(dec_rad),
        math.sin(dec_rad),
    ]
    max_dist = 2.0 * math.sin(radius_rad / 2.0)
    inds = t3._star_kd_tree.query_ball_point(vec, max_dist)
    inds_sorted = sorted(inds)  # star_table is brightest-first; sort preserves order
    stars = t3.star_table[inds_sorted]
    return {
        "query_vector": [float(x) for x in vec],
        "radius_rad": radius_rad,
        "max_chord_dist": max_dist,
        "num_nearby": len(inds_sorted),
        "indices": inds_sorted,
        "stars": [[float(x) for x in row] for row in stars],
    }


cases = [
    # (ra_rad, dec_rad, radius_rad, label)
    (0.5, 0.3, math.radians(5.0), "ra05_dec03_r5deg"),
    (1.0, -0.5, math.radians(3.0), "ra10_decneg05_r3deg"),
    (3.14, 1.2, math.radians(2.0), "ra_pi_dec12_r2deg"),
]

out = {"star_table_shape": list(t3.star_table.shape), "queries": []}
for ra, dec, r, label in cases:
    res = query(ra, dec, r)
    res["label"] = label
    out["queries"].append(res)
    print(f"  {label}: {res['num_nearby']} stars")

OUT.parent.mkdir(parents=True, exist_ok=True)
with open(OUT, "w") as f:
    json.dump(out, f, indent=2)

print(f"Wrote {OUT}")
