#!/usr/bin/env python3
"""cedar-goldens.py — re-derive every cedar answer in the e2e fixture from the vendored oracle.

The e2e gate (scripts/ci/e2e-eval.sh) compares the product against goldens in
crates/plate-solver/tests/data/cedar_parity.json. A golden is only as good as its
provenance, so this script recomputes each one with the cedar-solve copy vendored in
reference-solutions/, from the fixture's own centroid lists:

  cedar         solve_from_centroids, no FOV estimate (what E1/E2/E3 compare against)
  cedar_hinted  the same with the case's fov_hint (E1's FOV-estimate pass)

  --check  recompute and compare with the committed fixture; exit 1 on any difference.
           A committed golden that the oracle no longer reproduces is either a tampered
           fixture or a changed oracle, and both need a human.
  --write  recompute and rewrite the fixture (review the diff; .github/CODEOWNERS).

The INPUTS are pinned, not re-derived: re-deriving outputs from edited inputs would
reproduce cleanly. --check fails unless the fixture holds exactly EXPECTED_CASES cases
whose inputs (image, size, centroid list, FOV hint) hash to INPUTS_SHA256. Changing
them is a human decision: update both constants in this owned file, in the same PR.

The centroid lists are cedar-detect's, the copy vendored in reference-solutions/
(estimate_noise_from_image, then get_stars_from_image: sigma 8, binning 2,
normalize_rows, hot pixels), on PIL-decoded gray (convert('L')), brightest 30, as (y, x).
The goldens are cedar-solve's answers to those centroids, so E2 and E3 hold the
product's image path to cedar's own detect-then-solve pipeline. star-detection's
detect_stars returns the same lists bit for bit at these settings. Each fov_hint is
cedar's FOV to the nearest degree, +/-1 deg (roof, which cedar cannot solve, keeps 12).

Needs numpy, scipy and Pillow (cedar-solve's own dependencies; scripts/ci/cedar-requirements.txt).
Exit: 0 ok · 1 differs · 2 cannot run.
"""

import hashlib
import json
import math
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "crates/plate-solver/tests/data/cedar_parity.json"
CEDAR = ROOT / "reference-solutions/cedar-solve"
# Matches the fixture's documented solve settings.
SOLVE = dict(solve_timeout=5000, distortion=0, return_matches=True)
EXPECTED_CASES = 21
INPUTS_SHA256 = "90f3d167eaefe1213f6aef6e58305825905c851711d721ae27ea7a94781e5d5f"


def inputs_sha256(cases):
    keys = ("image", "width", "height", "centroids_yx", "fov_hint")
    canon = json.dumps([{k: c.get(k) for k in keys} for c in cases], sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canon.encode()).hexdigest()


def solve(t3, case, hint):
    import numpy as np

    cents = np.array(case["centroids_yx"], dtype=float).reshape(-1, 2)
    kwargs = dict(SOLVE)
    if hint is not None:
        kwargs.update(fov_estimate=hint["fov_estimate"], fov_max_error=hint["fov_max_error"])
    sol = t3.solve_from_centroids(cents, (case["height"], case["width"]), **kwargs)
    if sol.get("RA") is None:
        # cedar reports why it stopped via `status`; the fixture keeps cedar's names.
        names = {1: "MATCH_FOUND", 2: "NO_MATCH", 3: "TIMEOUT", 4: "CANCELLED", 5: "TOO_FEW"}
        return {"status": names.get(sol.get("status"), str(sol.get("status")))}
    return {
        "status": "MATCH_FOUND",
        "ra": float(sol["RA"]),
        "dec": float(sol["Dec"]),
        "roll": float(sol["Roll"]),
        "fov": float(sol["FOV"]),
        "matches": int(sol["Matches"]),
        "hip": sorted(int(x) for x in sol["matched_catID"]),
    }


def same(a, b):
    if a.get("status") != b.get("status"):
        return False
    if a["status"] != "MATCH_FOUND":
        return True
    # 1e-6 deg (3.6 mas): numpy/scipy round differently across CI runner hardware (up to
    # 0.65 mas seen), and a tampered golden is off by arcseconds, not milliarcseconds.
    close = all(math.isclose(a[k], b[k], rel_tol=0, abs_tol=1e-6) for k in ("ra", "dec", "roll", "fov"))
    return close and a["matches"] == b["matches"] and sorted(a["hip"]) == sorted(b["hip"])


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "--check"
    if mode not in ("--check", "--write"):
        sys.exit(__doc__)
    try:
        sys.path.insert(0, str(CEDAR))
        import tetra3  # noqa: E402  (the vendored oracle)
    except Exception as e:  # pragma: no cover
        print(f"cannot import the vendored cedar-solve: {e}")
        return 2
    fixture = json.loads(FIXTURE.read_text())
    cases = fixture["cases"]
    digest = inputs_sha256(cases)
    if mode == "--check" and (len(cases) != EXPECTED_CASES or digest != INPUTS_SHA256):
        print(f"FIXTURE INPUTS CHANGED: {len(cases)} cases (approved {EXPECTED_CASES}), "
              f"inputs sha256 {digest} (approved {INPUTS_SHA256}).")
        print("Outputs re-derived from edited inputs would reproduce cleanly, so inputs are pinned.")
        print("If the change is intended, update EXPECTED_CASES and INPUTS_SHA256 in this file.")
        return 1
    t3 = tetra3.Tetra3(load_database="default_database")
    diffs = 0
    for case in cases:
        fresh = {"cedar": solve(t3, case, None)}
        if "fov_hint" in case:
            fresh["cedar_hinted"] = solve(t3, case, case["fov_hint"])
        for key, value in fresh.items():
            if mode == "--write":
                case[key] = value
            elif key not in case or not same(case[key], value):
                diffs += 1
                print(f"DIFFERS: {case['image']} {key}: committed {case.get(key)} vs cedar {value}")
    if mode == "--write":
        FIXTURE.write_text(json.dumps(fixture, indent=1) + "\n")
        print(f"rewrote {FIXTURE.relative_to(ROOT)}; {len(cases)} cases, inputs sha256 {digest}")
        return 0
    n = sum(1 + ("fov_hint" in c) for c in cases)
    line = f"{n - diffs}/{n} committed cedar answers reproduced by the vendored oracle"
    print(line)
    # In CI: evidence for the job summary and the attestation.
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as out:
            out.write(f"reproduced={n - diffs}/{n} goldens reproduced by cedar\n")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as out:
            out.write(f"**Oracle integrity:** {line}.\n\n")
    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main())
