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
| `.venv/` | Python virtualenv with the reference deps (git-ignored). |
| `requirements.txt` | Pinned, `pip check`-clean dependency lock for `.venv`. |
| `check_env.py` | Smoke test: imports `tetra3` and runs one tiny solve against the reference `default_database.npz`. |
| `capture_*.py` | Per-feature fixture capture scripts (added by later tasks, e.g. `capture_core.py`). |

## Toolchain

- Python **3.9** (system `python3` on this machine).
- Apple Silicon (arm64); all pinned deps install from wheels (no source builds).

The pins honor cedar-solve's declared constraints (`numpy<2`, `Pillow<9`,
`scipy<2`), so the reference runs under exactly the versions it was written for.
`scikit-image` is pinned to `0.19.3` (the last line that accepts `Pillow<9`); a
newer scikit-image would force `Pillow>=9.1` and break that contract. `astropy`
and `scikit-image` are present for future coordinate/image parity captures even
though the core tetra3 solver only needs numpy/scipy/Pillow.

## (Re)create the environment

From the repo root:

```bash
python3 -m venv tools/parity/.venv
tools/parity/.venv/bin/python -m pip install --upgrade pip setuptools wheel
# cedar-solve brings numpy<2, Pillow<9, scipy<2:
tools/parity/.venv/bin/python -m pip install -e reference-solutions/cedar-solve
# extra reference deps, pinned to keep Pillow<9:
tools/parity/.venv/bin/python -m pip install "Pillow<9" "scikit-image<0.20" astropy
```

Or restore the exact lock plus the editable reference:

```bash
tools/parity/.venv/bin/python -m pip install -r tools/parity/requirements.txt
tools/parity/.venv/bin/python -m pip install -e reference-solutions/cedar-solve
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
