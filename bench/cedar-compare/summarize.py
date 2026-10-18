"""Summarize one run.sh run (OUT_DIR) into a JSON file, and print the headline numbers.

Reads OUT_DIR/{rust,rust_pgm,cedar}.jsonl and OUT_DIR/meta.json, all written by run.sh.
Run B (rust_pgm: identical PIL pixels for both pipelines) is primary; run A (rust: the Rust
side decodes the JPEG itself, as the product does) supplies decode times and the as-shipped
agreement.
usage: python -I -B summarize.py OUT_DIR DEST_JSON
"""
import json
import math
import statistics as st
import sys
from pathlib import Path

out, dest = Path(sys.argv[1]), Path(sys.argv[2])
load = lambda f: [json.loads(l) for l in (out / f).read_text().splitlines()]
A, B, C = load("rust.jsonl"), load("rust_pgm.jsonl"), load("cedar.jsonl")
img = lambda rows: {r["image"]: r for r in rows if r["kind"] == "image"}
RA, RB, RC = img(A), img(B), img(C)
setup = lambda rows: next(r for r in rows if r["kind"] == "setup")


def sep(a, b):
    r1, d1, r2, d2 = map(math.radians, (a["ra"], a["dec"], b["ra"], b["dec"]))
    h = math.sin((d2 - d1) / 2) ** 2 + math.cos(d1) * math.cos(d2) * math.sin((r2 - r1) / 2) ** 2
    return math.degrees(2 * math.asin(min(1, math.sqrt(h)))) * 3600


def diff(o, c):
    if o["status"] != "MATCH_FOUND" or c["status"] != "MATCH_FOUND":
        return None
    return {"pos": sep(o, c), "roll": abs((o["roll"] - c["roll"] + 180) % 360 - 180) * 3600,
            "fov": abs(o["fov"] - c["fov"]) * 3600, "same_stars": o["hip"] == c["hip"]}


short = lambda n: n.replace("2019-07-29T204726_", "").replace("_Try1", "")
rows = []
for name, c in RC.items():
    b, a = RB[name], RA[name]
    rows.append({
        "image": short(name), "w": b["width"], "h": b["height"], "mp": round(b["width"] * b["height"] / 1e6, 2),
        "stars_rust": b["stars_ours"], "stars_cedar": c["stars"], "top30_same": b["top30_within_0.1px"],
        "top30_n": min(30, c["stars"]), "detect_rust": b["detect_ours_ms"]["median"],
        "detect_cd_inproc": b["detect_cedar_detect_ms"]["median"], "detect_cd_grpc": c["detect_grpc_ms"]["median"],
        "solve_rust": b["solve_ours_ms"]["median"], "solve_cedar": c["solve_ms"]["median"],
        "decode_rust": a["decode_ms"], "decode_pil": c["decode_ms"],
        "status_rust": b["sol_ours"]["status"], "status_cedar": c["sol"]["status"],
        "rust": {k: b["sol_ours"].get(k) for k in ("ra", "dec", "roll", "fov", "matches", "rmse")},
        "cedar": {k: c["sol"].get(k) for k in ("ra", "dec", "roll", "fov", "matches", "rmse")},
        "agree_same_pixels": diff(b["sol_ours"], c["sol"]),
        "agree_as_shipped": diff(a["sol_ours"], c["sol"]),
        "agree_solve_from_image": diff(a["sol_from_image"], c["sol"]),
    })
for r in rows:
    r["e2e_rust"] = r["detect_rust"] + r["solve_rust"]
    r["e2e_cedar"] = r["detect_cd_grpc"] + r["solve_cedar"]

solved = [r for r in rows if r["status_cedar"] == "MATCH_FOUND"]
med = lambda k, s=solved: st.median(r[k] for r in s)
tot = lambda k, s=solved: sum(r[k] for r in s)
ratio = lambda num, den: {"median": st.median(r[num] / r[den] for r in solved), "total": tot(num) / tot(den)}
summary = {
    "solved_rust": sum(r["status_rust"] == "MATCH_FOUND" for r in rows),
    "solved_cedar": len(solved), "n": len(rows),
    "detection_identical": sum(r["top30_same"] == r["top30_n"] and r["stars_rust"] == r["stars_cedar"] for r in rows),
    "stage": {
        "detect": {"rust": med("detect_rust"), "cedar_inproc": med("detect_cd_inproc"), "cedar_grpc": med("detect_cd_grpc"),
                   "ratio_inproc": ratio("detect_cd_inproc", "detect_rust"), "ratio_grpc": ratio("detect_cd_grpc", "detect_rust")},
        "solve": {"rust": med("solve_rust"), "cedar": med("solve_cedar"), "ratio": ratio("solve_cedar", "solve_rust"),
                  "rust_max": max(r["solve_rust"] for r in solved), "cedar_max": max(r["solve_cedar"] for r in solved)},
        "e2e": {"rust": med("e2e_rust"), "cedar": med("e2e_cedar"), "ratio": ratio("e2e_cedar", "e2e_rust"),
                "rust_total": tot("e2e_rust"), "cedar_total": tot("e2e_cedar")},
        "decode": {"rust": med("decode_rust"), "pil": med("decode_pil")},
    },
}
for key in ("agree_same_pixels", "agree_as_shipped", "agree_solve_from_image"):
    v = [r[key] for r in rows if r[key]]
    summary[key] = {"n": len(v), "pos_max": max(d["pos"] for d in v), "pos_median": st.median(d["pos"] for d in v),
                    "roll_max": max(d["roll"] for d in v), "fov_max": max(d["fov"] for d in v),
                    "same_stars": sum(d["same_stars"] for d in v)}
meta = json.loads((out / "meta.json").read_text())
meta["settings"] = ("sigma 8, binning 2, normalize_rows, hot pixels; brightest 30 stars; no FOV hint; "
                    "distortion 0; timeout 5 s; cedar's default database (10–30°)")
data = {
    "meta": meta,
    "setup": {"rust_db_load_ms": [setup(A)["db_load_ms"], setup(B)["db_load_ms"]],
              "cedar_import_ms": setup(C)["import_ms"], "cedar_db_load_ms": setup(C)["db_load_ms"],
              "cedar_first_rpc_ms": next(r for r in C if r["kind"] == "first_rpc")["ms"]},
    "summary": summary, "images": rows,
}
dest.write_text(json.dumps(data, indent=1))
print(json.dumps({"setup": data["setup"], "summary": summary}, indent=1))
print("per image: image | stars r/c | top30 | det r / cd / grpc | solve r / c | status | pos\"")
for r in rows:
    d = r["agree_same_pixels"]
    print(f'{r["image"]:>16} | {r["stars_rust"]}/{r["stars_cedar"]} | {r["top30_same"]}/{r["top30_n"]} | '
          f'{r["detect_rust"]:.2f} / {r["detect_cd_inproc"]:.2f} / {r["detect_cd_grpc"]:.2f} | '
          f'{r["solve_rust"]:.2f} / {r["solve_cedar"]:.2f} | {r["status_rust"][:5]}/{r["status_cedar"][:5]} | '
          f'{(f"{d["pos"]:.4f}" if d else "-")}')
