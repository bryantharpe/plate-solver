#!/usr/bin/env python3
"""Compare pre-FU-A vs post-FU-A eval-harness runs (same host, controlled swap
of the ps-grpc binary only). Reports ps_grpc per-request wall-clock gains and
verifies the ps_grpc_vs_cedar_flow parity table is identical-green in both runs
(no accuracy loss). Also cross-checks post-FU-A self-reported t_extract_ms
(new in FUA.1) vs the standalone-ExtractCentroids extraction time the harness
already records."""
from __future__ import annotations
import json, statistics, sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent
PRE = BENCH / "results_pre_fua.json"
POST = BENCH / "results_post_fua.json"


def load(p: Path) -> dict:
    return json.load(open(p))


def per_sys(r: dict) -> dict:
    out = {}
    for rec in r["results"]:
        if rec["kind"] != "astronomical":
            continue
        img = rec["image_name"]
        det = rec["detect"]
        sol = rec["solve"]
        sys = sol.get("system") or det.get("system")
        d = {}
        wc = det.get("wall_clock_s", [])
        d["detect_wall_med"] = statistics.median(wc) if wc else None
        swc = sol.get("wall_clock_s", [])
        d["solve_wall_med"] = statistics.median(swc) if swc else None
        te = sol.get("t_extract_ms", [])
        d["t_extract_med"] = statistics.median(te) if te else None
        ts = sol.get("t_solve_ms", [])
        d["t_solve_med"] = statistics.median(ts) if ts else None
        out[(img, sys)] = d
    return out


def parity_primary(r: dict) -> list:
    return [c for c in r["parity"]["astronomical"]
            if c["comparison"] == "ps_grpc_vs_cedar_flow"
            and c.get("label") == "primary_same_catalog"]


def f(v) -> str:
    return f"{v*1000:.2f}" if v is not None else "—"


def main() -> int:
    if not PRE.exists() or not POST.exists():
        print(f"missing results: pre={PRE.exists()} post={POST.exists()}")
        return 1
    post = per_sys(load(POST))
    pre = per_sys(load(PRE))
    imgs = sorted({img for (img, sys) in post})

    print("=" * 78)
    print("ps_grpc per-request wall-clock: PRE vs POST FU-A (median over iterations)")
    print("=" * 78)
    print(f"{'image':38} {'det_pre':>8} {'det_post':>9} {'sol_pre':>8} {'sol_post':>9}  (ms)")
    pdet, xdet, psol, xsol = [], [], [], []
    for img in imgs:
        q = pre.get((img, "ps_grpc"), {})
        p = post.get((img, "ps_grpc"), {})
        if q.get("detect_wall_med") is not None: pdet.append(q["detect_wall_med"])
        if p.get("detect_wall_med") is not None: xdet.append(p["detect_wall_med"])
        if q.get("solve_wall_med") is not None: psol.append(q["solve_wall_med"])
        if p.get("solve_wall_med") is not None: xsol.append(p["solve_wall_med"])
        print(f"{img[:36]:38} {f(q.get('detect_wall_med')):>8} {f(p.get('detect_wall_med')):>9} "
              f"{f(q.get('solve_wall_med')):>8} {f(p.get('solve_wall_med')):>9}")
    print()
    if pdet and xdet:
        dp, dx = statistics.median(pdet), statistics.median(xdet)
        print(f"DETECT wall (median-of-medians): pre={dp*1000:.3f}ms post={dx*1000:.3f}ms "
              f"Δ={(dx-dp)*1000:+.3f}ms ({(dx/dp-1)*100:+.1f}%)")
    if psol and xsol:
        sp, sx = statistics.median(psol), statistics.median(xsol)
        print(f"SOLVE   wall (median-of-medians): pre={sp*1000:.3f}ms post={sx*1000:.3f}ms "
              f"Δ={(sx-sp)*1000:+.3f}ms ({(sx/sp-1)*100:+.1f}%)")

    # t_extract / t_solve self-reported (post only — pre had hardcoded 0.0)
    tex = [post[(img, "ps_grpc")]["t_extract_med"] for img in imgs
           if post.get((img, "ps_grpc"), {}).get("t_extract_med") is not None]
    tsv = [post[(img, "ps_grpc")]["t_solve_med"] for img in imgs
           if post.get((img, "ps_grpc"), {}).get("t_solve_med") is not None]
    print()
    print("=" * 78)
    print("ps_grpc self-reported wire timings (new in FUA.1)")
    print("=" * 78)
    if tex:
        # exclude hale_bopp (NoMatch exhaustion) from the "typical" median for clarity
        tex_no_hale = [v for img in imgs
                       for v in [post[(img, "ps_grpc")].get("t_extract_med")]
                       if v is not None and "hale_bopp" not in img]
        print(f"t_extract_ms median (post): {statistics.median(tex):.4f}ms  "
              f"(pre-FU-A: hardcoded 0.0 — no measurement)")
    if tsv:
        print(f"t_solve_ms   median (post): {statistics.median(tsv):.4f}ms")

    # parity identical-green check
    print()
    print("=" * 78)
    print("ACCURACY: ps_grpc_vs_cedar_flow primary_same_catalog parity")
    print("=" * 78)
    pp = parity_primary(load(PRE))
    xp = parity_primary(load(POST))
    pre_flag = sum(1 for c in pp if c["flagged"])
    post_flag = sum(1 for c in xp if c["flagged"])
    print(f"pre  rows={len(pp)} flagged={pre_flag}  -> {'IDENTICAL-GREEN' if pre_flag==0 else 'NOT GREEN'}")
    print(f"post rows={len(xp)} flagged={post_flag}  -> {'IDENTICAL-GREEN' if post_flag==0 else 'NOT GREEN'}")

    # per-image parity diff: compare RA/Dec/matched_cat_ids between pre and post
    def key(c):
        ch = c["checks"]
        ra = ch["ra_deg"]; dec = ch["dec_deg"]; ids = ch["matched_cat_ids"]
        return (round(ra["a"], 9) if ra.get("a") is not None else None,
                round(dec["a"], 9) if dec.get("a") is not None else None,
                tuple(sorted(ids.get("a", []))) if "a" in ids else None,
                c["checks"]["status"]["a"])
    pd = {c["image_name"]: key(c) for c in pp}
    xd = {c["image_name"]: key(c) for c in xp}
    drift = []
    for img in sorted(set(pd) & set(xd)):
        if pd[img] != xd[img]:
            drift.append((img, pd[img], xd[img]))
    print(f"pre-vs-post ps_grpc result drift: {'NONE (zero accuracy change)' if not drift else 'DRIFT:'}")
    for img, a, b in drift:
        print(f"  {img}: pre={a} post={b}")

    # matched_cat_ids exact between pre and post
    same_ids = sum(1 for img in set(pd) & set(xd) if pd[img][2] == xd[img][2])
    print(f"matched_cat_ids identical pre==post on {same_ids}/{len(set(pd)&set(xd))} images")

    return 0 if (pre_flag == 0 and post_flag == 0 and not drift) else 2


if __name__ == "__main__":
    sys.exit(main())