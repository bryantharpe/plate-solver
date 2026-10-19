#!/usr/bin/env bash
# Compare the Rust pipeline (star-detection + plate-solver) with cedar-detect + cedar-solve
# on the medium_fov images. Builds both, sets up a Python venv, runs both pipelines and
# writes a JSON summary. See README.md for what the numbers mean.
#
# usage: bench/cedar-compare/run.sh [NAME...]   (default: every medium_fov/*.jpg)
#
# Environment (all optional):
#   CARGO_TARGET_DIR  build dir                (default: bench/cedar-compare/target)
#   BENCH_VENV        Python venv              (default: bench/cedar-compare/.venv)
#   BENCH_OUT         raw output + summary     (default: bench/cedar-compare/out)
#   BENCH_REPS        timed reps per image     (default: 7)
#   PYTHON            interpreter for the venv (default: python3.12)
#   BENCH_GRPC_PORT   cedar-detect-server port (default: 50061)
#   BENCH_COMMIT      recorded commit, when git can't tell (default: git rev-parse)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
CEDAR_SOLVE="$REPO/reference-solutions/cedar-solve"
IMAGES="$CEDAR_SOLVE/examples/data/medium_fov"
DB="$CEDAR_SOLVE/tetra3/data/default_database.npz"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HERE/target}"
VENV="${BENCH_VENV:-$HERE/.venv}"
OUT="${BENCH_OUT:-$HERE/out}"
REPS="${BENCH_REPS:-7}"
PYTHON="${PYTHON:-python3.12}"

die() { echo "run.sh: $*" >&2; exit 1; }
[ "$(uname -s)" = Linux ] || die "Linux only (cedar-detect-server shares frames through /dev/shm)"
command -v cargo >/dev/null || die "cargo not found"
command -v "${PROTOC:-protoc}" >/dev/null || die "protoc not found (cedar-detect's build needs it); see README.md"
[ -f "$DB" ] || die "missing $DB"

if [ "$#" -gt 0 ]; then
  NAMES=("$@")
else
  NAMES=()
  while IFS= read -r f; do NAMES+=("$(basename "$f" .jpg)"); done \
    < <(LC_ALL=C find "$IMAGES" -maxdepth 1 -name '*.jpg' | LC_ALL=C sort)
fi
[ "${#NAMES[@]}" -gt 0 ] || die "no images in $IMAGES"

echo "== build (CARGO_TARGET_DIR=$CARGO_TARGET_DIR)"
# cedar-detect-server is built as a dependency of this package, so nothing is written
# into reference-solutions/ (no Cargo.lock, no target/).
cargo build --release --locked --manifest-path "$HERE/Cargo.toml"
cargo build --release --locked --manifest-path "$HERE/Cargo.toml" -p cedar_detect --bin cedar-detect-server
BIN="$CARGO_TARGET_DIR/release/cedar-compare"
SERVER="$CARGO_TARGET_DIR/release/cedar-detect-server"

echo "== venv ($VENV)"
[ -x "$VENV/bin/python" ] || "$PYTHON" -m venv "$VENV"
PY="$VENV/bin/python"
"$PY" -m pip install -q --disable-pip-version-check -r "$HERE/requirements.txt"

mkdir -p "$OUT"
GRAY="$OUT/gray"

# What the summary records about this run. -I keeps cedar-solve's tree out of sys.path,
# -B keeps it free of __pycache__.
commit="${BENCH_COMMIT:-$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo unknown)}"
if st="$(git -C "$REPO" status --porcelain 2>/dev/null)"; then
  dirty="$(printf '%s' "$st" | grep -c . || true)"
else
  dirty=unknown
fi
machine="$(lscpu 2>/dev/null | sed -n 's/^Model name: *//p' | head -n1), $(nproc) threads, $(uname -m), $(uname -r)"
"$PY" -I -B - "$OUT/meta.json" "$commit" "$dirty" "$REPS" "$machine" "$(rustc --version)" <<'EOF'
import json, platform, sys
from datetime import date
from importlib.metadata import version
dest, commit, dirty, reps, machine, rustc = sys.argv[1:]
meta = {"commit": commit, "dirty_files": dirty, "date": date.today().isoformat(),
        "reps": int(reps), "machine": machine, "rust": rustc.split()[1],
        "python": platform.python_version()}
meta.update({p: version(p) for p in ("numpy", "scipy", "pillow", "grpcio", "protobuf")})
open(dest, "w").write(json.dumps(meta, indent=1) + "\n")
print(json.dumps(meta))
EOF

run() {  # label, stdout file, command...
  local label="$1" dest="$2"; shift 2
  if ! "$@" > "$OUT/$dest.jsonl" 2> "$OUT/$dest.err"; then
    tail -n 20 "$OUT/$dest.err" >&2
    die "$label failed (log: $OUT/$dest.err)"
  fi
  echo "$label: $(grep -c '"kind": *"image"' "$OUT/$dest.jsonl") images"
}

echo "== run (${#NAMES[@]} images, $REPS reps each)"
# cedar first: it also saves PIL's gray frames, which run B reads.
run "cedar (PIL + gRPC cedar-detect + cedar-solve)" cedar \
  "$PY" -I -B "$HERE/cedar_bench.py" "$CEDAR_SOLVE" "$IMAGES" "$GRAY" "$REPS" "$SERVER" "${NAMES[@]}"
run "rust A (decodes the JPEGs itself)" rust \
  "$BIN" "$DB" "$IMAGES" "$REPS" "${NAMES[@]}"
run "rust B (PIL's gray frames)" rust_pgm \
  env BENCH_EXT=pgm "$BIN" "$DB" "$GRAY" "$REPS" "${NAMES[@]}"

echo "== summary"
"$PY" -I -B "$HERE/summarize.py" "$OUT" "$OUT/bench-data.json"
echo "wrote $OUT/bench-data.json"
