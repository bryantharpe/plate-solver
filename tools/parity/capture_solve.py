#!/usr/bin/env python3
"""Capture golden solve output from the cedar-solve reference.

Uses the committed ``default_database.npz`` and one ``medium_fov`` example
image; records RA/Dec/Roll/FOV/matched_cat_ids as a JSON fixture consumed by
the SV6 reference-parity test in ps-solve.

Run:  tools/parity/.venv/bin/python tools/parity/capture_solve.py
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DB_PATH = ROOT / "reference-solutions/cedar-solve/tetra3/data/default_database.npz"
MEDIUM_FOV_DIR = ROOT / "reference-solutions/cedar-solve/examples/data/medium_fov"
OUTPUT = ROOT / "ps-solve/tests/fixtures/reference_solve.json"

IMAGE_NAME = "2019-07-29T204726_Alt40_Azi-135_Try1.jpg"


def main() -> int:
    import tetra3
    from PIL import Image

    if not DB_PATH.is_file():
        print(f"FAIL: DB not found at {DB_PATH}", file=sys.stderr)
        return 1

    img_path = MEDIUM_FOV_DIR / IMAGE_NAME
    if not img_path.is_file():
        print(f"FAIL: image not found at {img_path}", file=sys.stderr)
        return 1

    t3 = tetra3.Tetra3(load_database=None)
    t3.load_database(DB_PATH)
    print(f"DB loaded: {DB_PATH.name}")

    with Image.open(img_path) as img:
        width, height = img.size
        # Extract centroids exactly as solve_from_image does internally
        centroids_yx = tetra3.get_centroids_from_image(img).tolist()
        sol = t3.solve_from_image(img, return_matches=True)

    if sol.get("RA") is None:
        print(f"FAIL: solver returned no solution: {sol}", file=sys.stderr)
        return 1

    fixture = {
        "image_name": IMAGE_NAME,
        "image_size": [height, width],
        "centroids_yx": centroids_yx,
        "ra_deg": float(sol["RA"]),
        "dec_deg": float(sol["Dec"]),
        "roll_deg": float(sol["Roll"]),
        "fov_deg": float(sol["FOV"]),
        "matches": int(sol["Matches"]),
        "matched_cat_ids": sorted([int(x) for x in sol["matched_catID"]]),
    }

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(fixture, indent=2) + "\n")
    print(f"Written: {OUTPUT}")
    print(
        "SOLVE OK: RA=%.6f Dec=%.6f Roll=%.6f FOV=%.4f Matches=%d"
        % (
            fixture["ra_deg"],
            fixture["dec_deg"],
            fixture["roll_deg"],
            fixture["fov_deg"],
            fixture["matches"],
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
