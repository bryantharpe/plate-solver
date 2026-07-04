# Parity-capture environment (`tools/parity`)

This directory holds the **offline reference environment** used to capture golden
parity fixtures for the Rust crates. The Rust implementation must match the
Python/Rust reference (tetra3 / cedar-solve / cedar-detect) within each OpenSpec
feature's stated tolerance, and the golden values come from *running the
reference here* — never hand-fabricated.

`reference-solutions/` is **read-only**; we install it (editable) but never
modify it.

## What's here

| Path | Purpose |
| --- | --- |
| `.venv/` | Python virtualenv with the cedar-solve/cedar-detect reference deps (git-ignored). |
| `requirements.txt` | Pinned, `pip check`-clean dependency lock for `.venv`. |
| `.venv-tetra3-orig/` | Separate virtualenv for the **original, upstream** tetra3 (git-ignored). |
| `requirements-tetra3-orig.txt` | Pinned dependency lock for `.venv-tetra3-orig`. |
| `check_env.py` | Smoke test: imports `tetra3` and runs one tiny solve against the reference `default_database.npz`. |
| `capture_*.py` | Per-feature fixture capture scripts (added by later tasks, e.g. `capture_core.py`). |
| `benchmark/` | `feat-09-eval-harness` benchmark/parity harness (see `openspec/changes/feat-09-eval-harness/`). |

## Two venvs, and why they're separate

`reference-solutions/tetra3` (original upstream, `setup.py`/`find_packages()`) and
`reference-solutions/cedar-solve` (`pyproject.toml`, `include = ["tetra3*"]`) both
install a top-level Python package named `tetra3` — one process can only ever
`import tetra3` from whichever was installed last, silently shadowing the other.
They also ship **incompatible on-disk catalog formats**: original tetra3 hard-codes
quadratic probing with no `hash_table_type` field, so it cannot read cedar-solve's
`linear_probe` catalog (see `openspec/changes/feat-09-eval-harness/design.md`).
So there are two isolated venvs:

- **`.venv`** — cedar-solve (its bundled `tetra3` fork) + the `cedar-detect` extra
  (`grpcio`, `protobuf`) + `grpcio-tools`. Used for the cedar-flow and ps-grpc
  comparisons, and for capturing parity fixtures against cedar-solve's reference.
- **`.venv-tetra3-orig`** — the original upstream `tetra3` package, installed
  editable, nothing else. Used only to run tetra3-original as an isolated
  subprocess (its own bundled `default_database.npz`, its own catalog format) for
  the eval-harness's cross-catalog comparison.

## Toolchain

- Python **3.12** (system `python3` on this Linux/aarch64 host, re-verified
  2026-07-04). The dep pins below reflect that host; a different Python/arch
  combination may need different wheel/source-build resolutions — re-freeze
  rather than assuming these pins are portable.
- `libjpeg-dev`/`zlib1g-dev` (OS packages) must be installed before `Pillow<9`
  will build here: no linux/aarch64 wheel exists below Pillow 9.2.0, so
  cedar-solve's declared `Pillow<9` upper bound must compile from source.

The `.venv` pins honor cedar-solve's declared constraints (`numpy<2,>=1.21.1`,
`Pillow<9,>=8.3.1`, `scipy<2,>=1.7.1`), so the reference runs under exactly the
versions it was written for. `scikit-image` and `astropy` were previously listed
here as speculative extras for future coordinate/image parity captures, but
neither is used by any capture script today, so both are dropped from `.venv`
until something in this repo actually needs them. `scikit-image==0.19.3` (the
last version accepting `Pillow<9`) additionally fails to build on this host's
Python 3.12 (its old build backend can't be installed under 3.12); `astropy`
has no such issue — a `numpy<2`-compatible release (e.g. 7.2.1) installs fine,
it's simply unused. Add either back only once something actually needs it.

## (Re)create the environment

From the repo root:

```bash
python3 -m venv tools/parity/.venv
tools/parity/.venv/bin/python -m pip install --upgrade pip setuptools wheel
# cedar-solve brings numpy<2, Pillow<9, scipy<2; [cedar-detect] adds grpcio/protobuf:
tools/parity/.venv/bin/python -m pip install -e "reference-solutions/cedar-solve[cedar-detect]"
tools/parity/.venv/bin/python -m pip install grpcio-tools
```

Or restore the exact lock plus the editable reference:

```bash
tools/parity/.venv/bin/python -m pip install -r tools/parity/requirements.txt
tools/parity/.venv/bin/python -m pip install -e "reference-solutions/cedar-solve[cedar-detect]"
```

The second venv, for tetra3-original:

```bash
python3 -m venv tools/parity/.venv-tetra3-orig
tools/parity/.venv-tetra3-orig/bin/python -m pip install --upgrade pip setuptools wheel
tools/parity/.venv-tetra3-orig/bin/python -m pip install -e reference-solutions/tetra3
```

Or restore its lock plus the editable reference:

```bash
tools/parity/.venv-tetra3-orig/bin/python -m pip install -r tools/parity/requirements-tetra3-orig.txt
tools/parity/.venv-tetra3-orig/bin/python -m pip install -e reference-solutions/tetra3
```

## Verify the environment

```bash
tools/parity/.venv/bin/python tools/parity/check_env.py
```

Expected: exit 0, after loading `default_database.npz`, a line like

```
SOLVE OK on <image>.tiff: RA=230.668489 Dec=11.035496 Roll=332.283248 FOV=11.4221 Matches=22
```

`tools/parity/.venv/bin/python -m pip check` should also report
`No broken requirements found`.

Smoke-check `.venv-tetra3-orig` the same way — confirm it imports the *original*
package (not cedar-solve's fork) and reports no broken requirements:

```bash
tools/parity/.venv-tetra3-orig/bin/python -c "import tetra3; print(tetra3.__file__)"
# expect a path under reference-solutions/tetra3/, not reference-solutions/cedar-solve/
tools/parity/.venv-tetra3-orig/bin/python -m pip check
```

## How to (re)capture fixtures

Each Rust feature that asserts numerical parity has a capture script here that
emits golden JSON (or binary) into the consuming crate's `tests/fixtures/`
directory. The pattern for every capture script:

1. Activate the reference: import from `tetra3` / cedar-solve (and, for detection,
   the reference cedar-detect). The repo root is two levels up from this dir
   (`Path(__file__).resolve().parents[2]`).
2. Drive the reference with a **fixed, documented input** (e.g. a known set of
   RA/Dec pairs, a committed example image, or the reference
   `default_database.npz`) so the output is deterministic.
3. Write the reference output as a fixture under the target crate's `tests/`
   (e.g. `ps-core/tests/fixtures/*.json`). Commit the fixture; the Rust test
   reads it and asserts parity within the spec tolerance.
4. Re-running the capture script must reproduce byte-identical fixtures.

Run a capture script with the venv interpreter, from the repo root, e.g.:

```bash
tools/parity/.venv/bin/python tools/parity/capture_core.py
```

**Never edit a committed golden fixture by hand to make a test pass** — re-capture
it from the reference instead.
