"""Time cedar's own combination the way its example (examples/test_tetra3.py) runs it:
PIL gray -> cedar-detect-server over gRPC (shared memory), sigma 8, binned, hot pixels,
normalize_rows -> brightest 30 -> cedar-solve solve_from_centroids(timeout 5000, distortion 0).

Also saves PIL's gray frames to GRAY_DIR as binary PGM, so the Rust harness can run on
exactly the same pixels (BENCH_EXT=pgm).

usage: python -I -B cedar_bench.py CEDAR_SOLVE_DIR IMAGE_DIR GRAY_DIR REPS SERVER_BIN NAME...
       (JSON lines on stdout; the gRPC port is $BENCH_GRPC_PORT, default 50061)
"""
import json
import os
import sys
import time
from pathlib import Path

cedar_solve, image_dir, gray_dir = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
reps, server, names = int(sys.argv[4]), sys.argv[5], sys.argv[6:]
port = int(os.environ.get("BENCH_GRPC_PORT", "50061"))
sys.path.insert(0, str(cedar_solve))
gray_dir.mkdir(parents=True, exist_ok=True)

t0 = time.perf_counter()
import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402
import tetra3  # noqa: E402
from tetra3 import cedar_detect_client  # noqa: E402
import_ms = (time.perf_counter() - t0) * 1e3

t0 = time.perf_counter()
t3 = tetra3.Tetra3(load_database="default_database")
load_ms = (time.perf_counter() - t0) * 1e3

cd = cedar_detect_client.CedarDetectClient(binary_path=server, port=port)


def stats(v):
    v = sorted(v)
    return {"median": v[len(v) // 2], "min": v[0], "max": v[-1]}


def ms(t):
    return (time.perf_counter() - t) * 1e3


def detect(arr):
    return cd.extract_centroids(arr, sigma=8, use_binned=True, detect_hot_pixels=True, normalize_rows=True)


# First RPC waits for the server process to come up; time it on its own.
first = None
print(json.dumps({"kind": "setup", "import_ms": import_ms, "db_load_ms": load_ms}), flush=True)
names_status = {1: "MATCH_FOUND", 2: "NO_MATCH", 3: "TIMEOUT", 4: "CANCELLED", 5: "TOO_FEW"}
try:
    for name in names:
        t = time.perf_counter()
        with Image.open(image_dir / f"{name}.jpg") as img:
            gray = img.convert(mode="L")
            arr = np.asarray(gray, dtype=np.uint8)
        decode_ms = ms(t)
        # Save exactly these pixels (binary PGM) so the Rust harness can run on them too.
        gray.save(gray_dir / f"{name}.pgm")
        h, w = arr.shape
        if first is None:
            t = time.perf_counter()
            detect(arr)
            first = ms(t)
            print(json.dumps({"kind": "first_rpc", "ms": first}), flush=True)
        d, s = [], []
        for _ in range(reps):
            t = time.perf_counter()
            cents = detect(arr)
            d.append(ms(t))
            t = time.perf_counter()
            sol = t3.solve_from_centroids(cents[:30], (h, w), return_matches=True, return_catalog=False,
                                          solve_timeout=5000, distortion=0)
            s.append(ms(t))
        if sol.get("RA") is None:
            out = {"status": names_status.get(sol.get("status"), str(sol.get("status")))}
        else:
            out = {"status": "MATCH_FOUND", "ra": float(sol["RA"]), "dec": float(sol["Dec"]),
                   "roll": float(sol["Roll"]), "fov": float(sol["FOV"]), "matches": int(sol["Matches"]),
                   "rmse": float(sol["RMSE"]), "hip": sorted(int(x) for x in sol["matched_catID"])}
        print(json.dumps({"kind": "image", "image": name, "width": w, "height": h, "decode_ms": decode_ms,
                          "stars": len(cents), "top30": [[float(y), float(x)] for y, x in cents[:30]],
                          "detect_grpc_ms": stats(d), "solve_ms": stats(s), "sol": out}), flush=True)
finally:
    del cd
