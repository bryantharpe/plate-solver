#!/usr/bin/env bash
# Fetch the upstream reference implementations this project is tested and
# benchmarked against, into ./references/ (gitignored — nothing upstream is
# redistributed by this repository).
#
#   tetra3       — ESA's Python plate solver (Apache-2.0). The algorithm this
#                  project reimplements, and a benchmark subject.
#   cedar-solve  — Steven Rosenthal's tetra3 fork (Apache-2.0). Benchmark subject.
#   cedar-detect — Steven Rosenthal's star detector (Functional Source License
#                  1.1). Used ONLY as a local test oracle: the star-detection
#                  parity test reads its test_data images. Never vendored,
#                  never linked, never redistributed.
#
# Pins: tetra3 is the exact commit the committed parity goldens were generated
# against. The cedar pins are the upstream commits closest to the copies used
# during development; the goldens themselves are committed in-repo
# (crates/star-detection/tests/data/), so upstream drift cannot silently
# change what the parity test asserts.
set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p references

fetch() {
  local name="$1" url="$2" sha="$3"
  if [ -d "references/$name/.git" ]; then
    echo "references/$name already present — leaving as is"
    return
  fi
  echo "Fetching $name @ ${sha:0:12}..."
  git init -q "references/$name"
  git -C "references/$name" remote add origin "$url"
  git -C "references/$name" fetch -q --depth 1 origin "$sha"
  git -C "references/$name" checkout -q FETCH_HEAD
}

fetch tetra3       https://github.com/esa/tetra3.git         f9fa2eb9a32a5efc529e2d86f0b59f35b1e9028d
fetch cedar-solve  https://github.com/smroid/cedar-solve.git 1a8a1d75fbcfc9ea4853af168af176b871954f08
fetch cedar-detect https://github.com/smroid/cedar-detect.git 1fe1bb1c7531f0112ab3fab7895cb61e317b6b71

echo "Done. references/ is gitignored; run 'cargo test' to include the parity tests."
