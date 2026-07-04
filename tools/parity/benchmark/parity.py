#!/usr/bin/env python3
"""Parity check (feat-09 eval-harness, tasks 3.1-3.5).

Reads a ``results.json`` produced by ``run_benchmark.py`` and writes a
``"parity"`` section into it: for every astronomical image, three pairwise
comparisons (``ps_grpc`` vs ``cedar_flow`` primary/same-catalog, plus
``ps_grpc``/``cedar_flow`` each vs ``tetra3_original`` as cross-catalog sanity
checks); for every stress image, a per-system solve-status check.

Tolerances for RA/Dec, matched catalog IDs, and detection centroids are
reused verbatim from ``openspec/IMPLEMENTATION-STATUS.md`` (ps-solve's sv6
parity tests, ps-detect's detect_parity test). Roll and FOV tolerances are
harness-defined (task 3.2): no prior end-to-end tolerance exists for either
field in this repo, only for pure attitude-math given an already-known
rotation matrix.

A mismatch is recorded as ``"flagged": true`` and never aborts the run -
every comparison for every image is always attempted and always reported.

Run (stdlib only, no venv needed - after run_benchmark.py has produced
results.json):

    python3 tools/parity/benchmark/parity.py [--results tools/parity/benchmark/results.json]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

BENCHMARK_DIR = Path(__file__).resolve().parent

# --- RA/Dec and centroid tolerances, reused verbatim from
# openspec/IMPLEMENTATION-STATUS.md ---
RA_DEC_TOLERANCE_ARCSEC = 10.0
CENTROID_TOLERANCE_PX = 0.1
# IMPLEMENTATION-STATUS.md's own matched-catalog-ID precedent (sv6) is exact
# set equality, not a numeric tolerance - this repo has no established
# *near-exact* matched-ID bound. feat-02's hale_bopp centroid-COUNT tolerance
# (±2) is the closest existing precedent for "near-exact" set agreement, so
# it's adapted here as the symmetric-difference bound, since the primary
# pair's inputs come from two independent detectors (ps-detect, cedar-detect)
# and aren't guaranteed to agree exactly - see feat-05's
# solve_from_image_parity note. This is an adapted bound, not a verbatim one.
MATCHED_ID_SYMMETRIC_DIFF_TOLERANCE = 2

# Harness-defined (task 3.2): no prior end-to-end tolerance exists for these
# two fields in this repo.
ROLL_TOLERANCE_DEG = 0.01
FOV_RELATIVE_TOLERANCE = 0.001  # 0.1%, relative to max(|a|, |b|)

STRESS_EXPECTED_STATUSES = ("NO_MATCH", "TOO_FEW")

# Primary (same-catalog) pair first, then the two cross-catalog sanity checks
# (design.md: tetra3_original's bundled catalog is a different build/schema
# than cedar-solve's, so any comparison involving it is cross-catalog).
PAIRWISE_COMPARISONS = [
    ("ps_grpc", "cedar_flow", False),
    ("ps_grpc", "tetra3_original", True),
    ("cedar_flow", "tetra3_original", True),
]
SYSTEMS = ("ps_grpc", "cedar_flow", "tetra3_original")


def _has_error(stage: Dict[str, Any]) -> bool:
    return "error" in stage


def _check_field_close(a: Optional[float], b: Optional[float], *, abs_tol: Optional[float] = None,
                        rel_tol: Optional[float] = None, scale: float = 1.0) -> Dict[str, Any]:
    """Compares two scalar fields, either by absolute error (optionally scaled,
    e.g. degrees -> arcsec) or by relative error. ``ok`` is ``None`` (not
    ``False``) when a value is missing, since a missing value isn't a
    detected mismatch - it just can't be evaluated."""
    if a is None or b is None:
        return {"a": a, "b": b, "ok": None, "reason": "missing value"}
    diff = abs(a - b)
    if rel_tol is not None:
        denom = max(abs(a), abs(b)) or 1.0
        relative_error = diff / denom
        return {"a": a, "b": b, "relative_error": relative_error, "tolerance": rel_tol,
                 "ok": relative_error <= rel_tol}
    error = diff * scale
    return {"a": a, "b": b, "error": error, "tolerance": abs_tol, "ok": error <= abs_tol}


def _check_matched_cat_ids(a_ids: Optional[List[Any]], b_ids: Optional[List[Any]],
                            cross_catalog: bool) -> Dict[str, Any]:
    if cross_catalog:
        return {"ok": None, "applicable": False,
                 "reason": "cross-catalog: catalog IDs use different catalogs, not directly comparable"}
    if a_ids is None or b_ids is None:
        return {"ok": None, "applicable": True, "reason": "missing matched_cat_ids (no solution or solve error)"}
    try:
        set_a, set_b = set(a_ids), set(b_ids)
    except TypeError:
        # star_catalog_IDs can be 2-D (a per-star tuple of catalog columns)
        # rather than a flat scalar array (see adapters.py's
        # _coerce_matched_cat_ids) - not set-comparable in that shape.
        return {"ok": None, "applicable": True, "reason": "non-scalar catalog IDs, not set-comparable"}
    symmetric_difference = set_a ^ set_b
    return {
        "applicable": True,
        "a_count": len(set_a), "b_count": len(set_b),
        "symmetric_difference_count": len(symmetric_difference),
        "tolerance": MATCHED_ID_SYMMETRIC_DIFF_TOLERANCE,
        "exact": set_a == set_b,
        "ok": len(symmetric_difference) <= MATCHED_ID_SYMMETRIC_DIFF_TOLERANCE,
    }


def _check_centroids(a_centroids: Optional[List[List[float]]],
                      b_centroids: Optional[List[List[float]]]) -> Dict[str, Any]:
    """Index-based comparison (both systems return brightness-descending
    order - see feat-06's "brightest-first centroids match ps-detect" note),
    matching the ±0.1px per-axis tolerance ps-detect's detect_parity test
    already established against cedar-detect."""
    if a_centroids is None or b_centroids is None:
        return {"ok": None, "reason": "missing centroids"}
    n = min(len(a_centroids), len(b_centroids))
    max_error_px = 0.0
    mismatched_count = 0
    for i in range(n):
        ay, ax = a_centroids[i]
        by, bx = b_centroids[i]
        error_px = max(abs(ay - by), abs(ax - bx))
        max_error_px = max(max_error_px, error_px)
        if error_px > CENTROID_TOLERANCE_PX:
            mismatched_count += 1
    return {
        "a_count": len(a_centroids), "b_count": len(b_centroids), "compared": n,
        "count_diff": abs(len(a_centroids) - len(b_centroids)),
        "max_error_px": max_error_px, "mismatched_count": mismatched_count,
        "tolerance_px": CENTROID_TOLERANCE_PX,
        "ok": n > 0 and mismatched_count == 0,
    }


def _index_results(results: List[Dict[str, Any]]) -> Dict[str, Dict[str, Any]]:
    """image_name -> {"kind": ..., "systems": {system_name -> {"detect":, "solve":}}}"""
    out: Dict[str, Dict[str, Any]] = {}
    for entry in results:
        image_name = entry["image_name"]
        system = entry["detect"]["system"]
        bucket = out.setdefault(image_name, {"kind": entry["kind"], "systems": {}})
        bucket["systems"][system] = {"detect": entry["detect"], "solve": entry["solve"]}
    return out


def _pairwise_comparison(image_name: str, sys_a: str, sys_b: str, cross_catalog: bool,
                          image_entry: Dict[str, Any]) -> Dict[str, Any]:
    comparison = f"{sys_a}_vs_{sys_b}"
    label = "cross_catalog_sanity_check" if cross_catalog else "primary_same_catalog"
    a = image_entry["systems"].get(sys_a)
    b = image_entry["systems"].get(sys_b)
    if a is None or b is None:
        missing = sys_a if a is None else sys_b
        return {
            "image_name": image_name, "comparison": comparison, "label": label,
            "cross_catalog": cross_catalog, "flagged": True,
            "reason": f"missing result for {missing}",
        }

    detect_a, detect_b = a["detect"], b["detect"]
    solve_a, solve_b = a["solve"], b["solve"]
    checks: Dict[str, Any] = {}

    if _has_error(detect_a) or _has_error(detect_b):
        errors = [s["error"] for s in (detect_a, detect_b) if _has_error(s)]
        checks["centroids"] = {"ok": False, "reason": f"detect error: {'; '.join(errors)}"}
    else:
        checks["centroids"] = _check_centroids(detect_a.get("centroids_yx"), detect_b.get("centroids_yx"))

    if _has_error(solve_a) or _has_error(solve_b):
        errors = [s["error"] for s in (solve_a, solve_b) if _has_error(s)]
        reason = f"solve error: {'; '.join(errors)}"
        for key in ("ra_deg", "dec_deg", "roll_deg", "fov_deg", "matched_cat_ids", "status"):
            checks[key] = {"ok": False, "reason": reason}
    else:
        checks["ra_deg"] = _check_field_close(solve_a.get("ra_deg"), solve_b.get("ra_deg"),
                                                abs_tol=RA_DEC_TOLERANCE_ARCSEC, scale=3600.0)
        checks["dec_deg"] = _check_field_close(solve_a.get("dec_deg"), solve_b.get("dec_deg"),
                                                 abs_tol=RA_DEC_TOLERANCE_ARCSEC, scale=3600.0)
        checks["roll_deg"] = _check_field_close(solve_a.get("roll_deg"), solve_b.get("roll_deg"),
                                                  abs_tol=ROLL_TOLERANCE_DEG, scale=1.0)
        checks["fov_deg"] = _check_field_close(solve_a.get("fov_deg"), solve_b.get("fov_deg"),
                                                 rel_tol=FOV_RELATIVE_TOLERANCE)
        checks["matched_cat_ids"] = _check_matched_cat_ids(
            solve_a.get("matched_cat_ids"), solve_b.get("matched_cat_ids"), cross_catalog
        )
        checks["status"] = {
            "a": solve_a.get("status"), "b": solve_b.get("status"),
            "ok": solve_a.get("status") == solve_b.get("status"),
        }

    flagged = any(check.get("ok") is False for check in checks.values())
    return {
        "image_name": image_name, "comparison": comparison, "label": label,
        "cross_catalog": cross_catalog, "checks": checks, "flagged": flagged,
    }


def _stress_status_check(image_name: str, system: str, solve_entry: Dict[str, Any]) -> Dict[str, Any]:
    if _has_error(solve_entry):
        return {
            "image_name": image_name, "system": system, "status": None,
            "expected_statuses": list(STRESS_EXPECTED_STATUSES),
            "ok": False, "flagged": True, "reason": f"solve error: {solve_entry['error']}",
        }
    status = solve_entry.get("status")
    ok = status in STRESS_EXPECTED_STATUSES
    return {
        "image_name": image_name, "system": system, "status": status,
        "expected_statuses": list(STRESS_EXPECTED_STATUSES),
        "ok": ok, "flagged": not ok,
    }


def compute_parity(data: Dict[str, Any]) -> Dict[str, Any]:
    indexed = _index_results(data["results"])
    astronomical: List[Dict[str, Any]] = []
    stress: List[Dict[str, Any]] = []

    for image_name, image_entry in indexed.items():
        kind = image_entry["kind"]
        if kind == "astronomical":
            for sys_a, sys_b, cross_catalog in PAIRWISE_COMPARISONS:
                astronomical.append(_pairwise_comparison(image_name, sys_a, sys_b, cross_catalog, image_entry))
        elif kind == "stress":
            for system in SYSTEMS:
                sys_entry = image_entry["systems"].get(system)
                if sys_entry is None:
                    stress.append({
                        "image_name": image_name, "system": system, "status": None,
                        "expected_statuses": list(STRESS_EXPECTED_STATUSES),
                        "ok": False, "flagged": True, "reason": "missing result",
                    })
                    continue
                stress.append(_stress_status_check(image_name, system, sys_entry["solve"]))
        else:
            raise ValueError(f"unknown corpus kind {kind!r} for image {image_name!r}")

    astronomical.sort(key=lambda e: (e["image_name"], e["comparison"]))
    stress.sort(key=lambda e: (e["image_name"], e["system"]))

    return {
        "tolerances": {
            "ra_dec_arcsec": {"value": RA_DEC_TOLERANCE_ARCSEC, "source": "IMPLEMENTATION-STATUS.md (verbatim)"},
            "centroid_px": {"value": CENTROID_TOLERANCE_PX, "source": "IMPLEMENTATION-STATUS.md (verbatim)"},
            "matched_cat_id_symmetric_diff": {
                "value": MATCHED_ID_SYMMETRIC_DIFF_TOLERANCE,
                "source": (
                    "harness near-exact bound, adapted from feat-02's hale_bopp "
                    "centroid-count tolerance - IMPLEMENTATION-STATUS.md's own "
                    "matched-catalog-ID precedent (sv6) is exact set equality, not "
                    "a numeric tolerance"
                ),
            },
            "roll_deg": {"value": ROLL_TOLERANCE_DEG, "source": "harness-defined"},
            "fov_relative": {"value": FOV_RELATIVE_TOLERANCE, "source": "harness-defined"},
            "stress_expected_statuses": list(STRESS_EXPECTED_STATUSES),
        },
        "astronomical": astronomical,
        "stress": stress,
    }


def _parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--results", type=Path, default=BENCHMARK_DIR / "results.json")
    return p.parse_args()


def main() -> int:
    args = _parse_args()
    if not args.results.is_file():
        print(f"FAIL: {args.results} not found - run run_benchmark.py first", file=sys.stderr)
        return 1

    data = json.loads(args.results.read_text())
    if "results" not in data:
        print(f"FAIL: {args.results} has no 'results' key - not a run_benchmark.py output", file=sys.stderr)
        return 1

    data["parity"] = compute_parity(data)
    args.results.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")

    n_astro_flagged = sum(1 for e in data["parity"]["astronomical"] if e["flagged"])
    n_stress_flagged = sum(1 for e in data["parity"]["stress"] if e["flagged"])
    print(
        f"Wrote parity section to {args.results}: "
        f"{len(data['parity']['astronomical'])} pairwise comparisons ({n_astro_flagged} flagged), "
        f"{len(data['parity']['stress'])} stress status checks ({n_stress_flagged} flagged)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
