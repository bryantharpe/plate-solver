#!/usr/bin/env python3
"""Smoke-test the reference parity-capture environment.

Proves that the committed cedar-solve reference can be imported and run inside
``tools/parity/.venv``: it loads the reference ``default_database.npz`` and runs
one tiny lost-in-space solve on a committed example image, then prints the
recovered attitude. Exits 0 on success, non-zero on any failure.

Run:  tools/parity/.venv/bin/python tools/parity/check_env.py
"""

import sys
from pathlib import Path

# Repo root is two levels up from tools/parity/check_env.py
REPO = Path(__file__).resolve().parents[2]
DB_PATH = REPO / "reference-solutions/cedar-solve/tetra3/data/default_database.npz"
IMAGE_DIR = REPO / "reference-solutions/cedar-solve/tests/data/example_images"


def main() -> int:
    # Import here so an import failure is reported clearly rather than at module load.
    import numpy
    import scipy
    from PIL import Image
    import tetra3

    print(f"numpy {numpy.__version__}, scipy {scipy.__version__}")

    if not DB_PATH.is_file():
        print(f"FAIL: reference database not found at {DB_PATH}", file=sys.stderr)
        return 1

    # Load the reference database explicitly from the committed .npz (load_database
    # normalises the suffix, so passing the .npz path uses exactly this file).
    t3 = tetra3.Tetra3(load_database=None)
    t3.load_database(DB_PATH)
    print(f"Loaded reference database: {DB_PATH.name}")

    images = sorted(IMAGE_DIR.glob("*.tiff")) + sorted(IMAGE_DIR.glob("*.tif"))
    if not images:
        print(f"FAIL: no example images found under {IMAGE_DIR}", file=sys.stderr)
        return 1
    image_path = images[0]

    with Image.open(image_path) as img:
        solution = t3.solve_from_image(img)

    ra = solution.get("RA")
    matches = solution.get("Matches")
    if ra is None or not matches:
        print(f"FAIL: solver did not recover an attitude: {solution}", file=sys.stderr)
        return 1

    print(
        "SOLVE OK on %s: RA=%.6f Dec=%.6f Roll=%.6f FOV=%.4f Matches=%d"
        % (
            image_path.name,
            solution["RA"],
            solution["Dec"],
            solution["Roll"],
            solution["FOV"],
            matches,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
