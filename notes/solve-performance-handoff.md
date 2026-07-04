# Handoff — improve `ps-solve` solve latency without reducing accuracy

> Written 2026-07-04 after the `feat-09-eval-harness` benchmark landed. Self-contained:
> a fresh session needs no prior conversation, only this file + the repo.
> Line numbers are given but **drift** — always `grep`/re-read before editing.

## Mandate (read this first)

**Goal.** Close the *solve*-stage latency gap between the Rust port (`ps-grpc`/`ps-solve`)
and the reference implementations it replaces (cedar-solve, original tetra3), measured by
the new benchmark harness.

**Hard constraint — do NOT reduce accuracy.** Every change must keep the numerical-parity
tests green and must never loosen a tolerance, `#[ignore]` a test, or stub a check to make
a gate pass. Parity is the contract (see *Accuracy guardrail* below). A faster solver that
drifts even slightly from the reference attitude is a regression, not a win.

**Non-goals.**
- **Do not touch the detect stage.** It already *beats* the references (1.5× faster than
  cedar-detect, 6.7× faster than original tetra3). Leave it alone.
- Not a memory audit, not a mobile port, not an accuracy improvement. Latency only.

## TL;DR

The solve loop tries 4-star combinations until one matches the pattern DB. The reference
(cedar-solve) iterates those combinations in a **breadth-first ("brightest first") order**
deliberately tuned to hit a match in few iterations. The Rust port iterates them in plain
**lexicographic order**, so it almost certainly examines many more combinations — each
paying a DB lookup + verification — before finding the *same* match. **Fixing the iteration
order is the #1 lever, and it is currently unaddressed** (the existing C1 fix note preserves
the lexicographic order). Confirm by counting combinations examined, then port the
breadth-first order, then re-run parity + the benchmark.

## Where things stand (measured, not guessed)

The benchmark report is committed on `main` at:
- `docs/benchmarks/report.md` (human-readable) and `docs/benchmarks/report.html`

Solve-stage results (median over the 9 astronomical images; ratio >1 = Rust faster):

| Comparison | Detect | Solve |
|---|---|---|
| ps_grpc vs cedar_flow | **1.5×** ✅ | **0.27×** ⚠️ (~3.7× slower) |
| ps_grpc vs tetra3_original | **6.69×** ✅ | **0.8×** ⚠️ (~1.25× slower) |

Concrete per-image solve time (image `2019-07-29T204726_Alt40_Azi-135_Try1.jpg`):
`ps_grpc 11.9 ms` vs `cedar_flow 1.4 ms` vs `tetra3_original 1.2 ms`. Extraction is on par
(~1.35 ms both) — **the entire gap is in the solve search loop**, not extraction.

Note on scale: absolute solve is ~12 ms, still roughly within the PRD's *~10 ms/solve*
budget. This is "catch the reference / earn mobile headroom," not "it's broken."

## Accuracy guardrail (never violate)

These must stay green after every change. Re-run them before declaring any win.

- `ps-solve/src/lib.rs::sv6_solve_from_centroids_parity` (~line 1554) — **19/19 matched
  catalog IDs exact**, RA/Dec within **10 arcsec** vs the tetra3 Python reference.
- `ps-solve/src/lib.rs::sv6_solve_from_image_parity` (~line 1399) — `MATCH_FOUND`, RA/Dec
  within 10 arcsec on the real JPEG.
- The harness's own **`ps_grpc_vs_cedar_flow` primary (same-catalog) parity table** in the
  report — currently all `✓` (centroids exact, RA within ~1″, Dec within ~0.2″, Roll exact,
  FOV within 0.05%, matched IDs exact). Must remain all `✓`.

Run: `cargo test -p ps-solve` (and `cargo test --workspace` for the full gate).

Tolerances live in `openspec/IMPLEMENTATION-STATUS.md` (RA/Dec 10 arcsec, matched IDs exact,
centroids ±0.1 px). **Do not invent looser ones.**

## Root cause, prioritized

### 1. Combination ordering — lexicographic → breadth-first  (biggest expected win, unaddressed)

Solve time ≈ **(combos examined before a match) × (cost per combo)**. The port loses on the
first factor.

- **Reference (cedar-solve):** `reference-solutions/cedar-solve/tetra3/tetra3.py:1819` —
  `for image_pattern_indices in breadth_first_combinations(pattern_centroids_inds, p_size)`;
  the comment at `:1815` reads *"Try all p_size star combinations chosen from the image
  centroids, brightest first."* Implementation:
  `reference-solutions/cedar-solve/tetra3/breadth_first_combinations.py` — *"breadth-first
  rather than depth-first."* Roughly: it brings in each new (dimmer) star only after
  exhausting combinations among the brighter ones, so a valid match among the bright stars
  surfaces early.
- **Rust port:** `ps-solve/src/lib.rs:595` `fn combinations_4(n) -> Vec<[usize;4]>` — plain
  lexicographic nested `a<b<c<d`. Consumed at `ps-solve/src/lib.rs:190`
  (`'outer: for combo in combinations_4(num_pattern_centroids)`). Lexicographic exhausts
  every combo with prefix `[0,1,2]` before it ever tries star 3+ — on a real image (false
  detections, missing/saturated stars) the true match usually isn't there, so it grinds
  through far more combos first.

**Action:** replace the lexicographic order with cedar's breadth-first order. Port
`breadth_first_combinations` faithfully (fixed `r = PATTERN_SIZE = 4`).

**Parity risk to verify, not assume:** changing the order changes *which combo is tried
when*, not the set of valid matches — for a correctly-solvable image both orders should
converge to the same attitude and matched IDs. **But you must prove it:** re-run the two
`sv6_*` parity tests and the harness `ps_grpc_vs_cedar_flow` table; all must stay green. If
a match changes, stop and investigate before proceeding.

### 2. C1 eager allocation — make it lazy, and do it *with* #1

`combinations_4` materializes the **entire** `Vec` of all C(n,4) combos before the loop
runs. Worst case (bundled DB, `verification_stars_per_fov = 150`): C(150,4) ≈ 20.3M combos
× 32 B ≈ **618 MiB** held for the whole solve. Details + a lazy-iterator scaffold already
exist in `notes/C1-lazy-combinations-fix.md`.

**Important framing:** C1 is primarily a **memory / mobile-OOM** fix, *not* the main
desktop-latency fix — at these test images' small star counts, eager generation is cheap in
time. So **don't do C1 alone expecting the benchmark to move much.** Instead, implement #1's
breadth-first order **as a lazy iterator**, killing both birds: correct order + no
allocation blowup + no front-loaded generation. ⚠️ The existing C1 note preserves the
lexicographic order — **change it to breadth-first**, don't copy it verbatim.

### 3. Per-combo allocation churn — secondary cleanup

The verification path (roughly `ps-solve/src/lib.rs:300–540`, on a candidate-key hit) does
~20 `.collect()` calls per candidate (nearby-star filtering, catalog-vector mapping,
matched-pair extraction) — each a heap allocation. Reuse scratch buffers / avoid
intermediate `Vec`s. **Do this last:** its impact scales with how many combos reach
verification, which #1 already reduces. Only pursue if the benchmark still shows a gap after
#1 (+#2). Note `candidate_keys` per combo is small (243 entries) and **not** worth touching.

## Confirm before you optimize (do not optimize blind)

Before changing any ordering, **instrument the combo counter**:

1. Add a counter in the `'outer` loop (`ps-solve/src/lib.rs:190`) that records combos
   examined per solve; log it (or expose it) for each benchmark image.
2. Run the harness, capture combos-per-image for the current lexicographic order.
3. (Optional but ideal) get the equivalent count from a cedar-solve trace for the same
   images.

Expected result: the Rust port examines several× more combos than cedar. If it does, #1 is
confirmed as the dominant cost and worth the work. If it does *not* (combos are already
comparable), the gap is per-combo cost instead — pivot to #3 and profile the DB
lookup / `nearby_stars` path. **Let the measurement pick the lever.**

## How to measure every change (the feedback loop)

The whole point of `feat-09-eval-harness` is that you can now measure each change in
isolation. After every change:

```
# (one-time) env is built by the harness's setup; venvs live at
#   tools/parity/.venv  and  tools/parity/.venv-tetra3-orig
cargo build --release -p ps-grpc          # rebuild the changed solver
tools/parity/.venv/bin/python tools/parity/benchmark/run_benchmark.py \
    --output tools/parity/benchmark/results.json
tools/parity/.venv/bin/python tools/parity/benchmark/report.py \
    --input tools/parity/benchmark/results.json \
    --out-md docs/benchmarks/report.md --out-html docs/benchmarks/report.html
```

Compare the new `ps_grpc vs cedar_flow` solve ratio against the 0.27× baseline above, and
confirm the `ps_grpc_vs_cedar_flow` parity table is still all `✓`. See
`tools/parity/benchmark/` and `openspec/changes/archive/.../feat-09-eval-harness/` (or
`openspec/specs/eval-harness/` once archived) for the harness contract.

## Suggested execution path

This is real solver work with a parity contract, so it fits the repo's spec-driven,
two-loop pipeline — the same one that built the harness:

- **Option A (governed):** open an OpenSpec change (e.g. `feat-10-solve-perf`) →
  proposal/design/tasks/spec → bead the tasks → `gt sling` them so each goes through
  `ps-coder` (implement) → `ps-judge` (review vs parity contract) → refinery merge. The
  `ps-judge` agent already knows to reject any loosened parity check.
- **Option B (direct):** implement #1 behind the combo-counter confirmation, run the
  benchmark + parity, iterate. Faster, less ceremony, but you own the review.

Either way: **confirm (counter) → change ordering (lazy breadth-first) → prove parity →
measure → (only if needed) trim allocations.**

## Exact code references (verify — line numbers drift)

| What | Path | ~Line |
|---|---|---|
| Solve entry point | `ps-solve/src/lib.rs` `solve_from_centroids` | 122 |
| The combo search loop (add counter here) | `ps-solve/src/lib.rs` `'outer: for combo …` | 190 |
| Lexicographic generator (REPLACE order) | `ps-solve/src/lib.rs` `fn combinations_4` | 595 |
| Verification path (~20 collects) | `ps-solve/src/lib.rs` | 300–540 |
| Parity test — from centroids | `ps-solve/src/lib.rs` `sv6_solve_from_centroids_parity` | 1554 |
| Parity test — from image | `ps-solve/src/lib.rs` `sv6_solve_from_image_parity` | 1399 |
| Reference breadth-first order | `reference-solutions/cedar-solve/tetra3/breadth_first_combinations.py` | — |
| Reference call site + "brightest first" | `reference-solutions/cedar-solve/tetra3/tetra3.py` | 1815, 1819 |
| Existing C1 lazy scaffold (make breadth-first) | `notes/C1-lazy-combinations-fix.md` | — |
| Established tolerances | `openspec/IMPLEMENTATION-STATUS.md` | — |
| Benchmark harness | `tools/parity/benchmark/` | — |
| Benchmark result (baseline) | `docs/benchmarks/report.md` | — |

> The C1 note cites `combinations_4` at `lib.rs:582-594`; it is now at 595 — line numbers
> have already drifted once. Always `grep` for the symbol, don't trust the number.

## Open questions / risks

1. **Does breadth-first change the recovered match?** Expected: no (same valid-match set,
   same verification). Must be proven by the `sv6_*` tests + harness parity table before the
   change lands. If it changes, the ordering interacts with the false-alarm/verification
   logic — investigate, don't paper over.
2. **Is `num_pattern_centroids` (n) large enough per image that C1's eager gen is a real
   latency cost here, or only a mobile-memory cost?** The combo counter answers this. If n
   is small on these images, don't expect C1-alone to move the desktop benchmark.
3. **After #1, is the residual gap per-combo cost (DB lookup / `nearby_stars`)?** If so, #3
   + profiling that path is the next lever. Measure before assuming.
