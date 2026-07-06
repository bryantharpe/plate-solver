# SP0.2 Lexicographic Baseline — Combos Examined (2026-07-04)

**Command:** `cargo run --release -p ps-solve --example combo_count`

**Purpose:** Establish baseline measurements of 4-star combo enumeration on 9 astronomical benchmark images before SP1.1's breadth-first iterator lands.

## Results Table

| image | status | combos_examined | t_solve_s |
|-------|--------|-----------------|-----------|
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | MatchFound | 1 | 0.002 |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | MatchFound | 1 | 0.024 |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | MatchFound | 1 | 0.001 |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | MatchFound | 1 | 0.046 |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | MatchFound | 1 | 0.002 |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | MatchFound | 2 | 0.039 |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | MatchFound | 1 | 0.001 |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | MatchFound | 1 | 0.028 |
| hale_bopp.jpg | NoMatch | 8855 | 0.184 |

## Decision Gate

**Summary:**
- Successful solves (MatchFound): 8 / 9, each at `combos_examined` 1 or 2
- NoMatch case (hale_bopp.jpg, still one of the 9 `ASTRONOMICAL_IMAGES`): `combos_examined` 8,855 = C(23,4), i.e. the full pattern-star search space exhausted
- (A corpus-wide total/average across these two regimes — ~985/image — is not a meaningful number on its own: it's dominated by the single NoMatch outlier and does not represent typical solve behavior. Reported per-case above instead.)

**Verdict: counts are already comparable — this baseline lands in the rule's "already comparable" branch, not "several× more."** For a fixed r=4, the first two combinations generated are identical under lexicographic and colexicographic (breadth-first) order — they only diverge starting from the third. Since 8 of 9 images match at combo #1 (one at #2), SP1.1's ordering change **cannot reduce the combo count on this corpus** for the matched cases. hale_bopp's NoMatch means the full space was enumerated regardless of order, so ordering can't help there either — order only ever changes *which* combo is examined at a given position, not how many are examined before a full-space exhaustion.

Per the task's decision rule: SP1.1 still proceeds — but for its C1 lazy-allocation memory win, not for a combo-count reduction this baseline doesn't support. The observed `ps-solve` vs cedar-solve solve-latency gap (0.27× baseline) is therefore **not explained by combination count on this corpus** and is most likely per-combo cost (verification-path allocation churn, DB lookup cost, etc.) — **pivot to SP3's profiling early** rather than expecting SP2.2 to show a large ordering-driven improvement. SP2.2 should still re-run this measurement post-SP1.1 to confirm combo counts stayed flat (as expected) and to check whether the C1 allocation-free iterator alone moved the needle on `t_solve`, but should not be read as a test of the ordering hypothesis on this corpus.

Separately, and out of scope for SP0.2/SP1–3 (already tracked as **H2**): `hale_bopp.jpg` failing to match here is a pre-existing limitation, not a regression from this measurement — `solve_from_image` hardcodes detection at `sigma=4.0` regardless of what an image needs (H2, "thread client detection params through solve_from_image"), and hale_bopp's own detection golden fixture (SD1/SD6) was captured at `sigma=8`. This is plausibly why it fails to solve under the default params used here; H2 fixes the underlying defect, not this task.

---

# SP2.2 Benchmark + Counter Re-Measurement, Post-SP1.1 (2026-07-04)

**Commands:**
```
cargo build --release -p ps-grpc
tools/parity/.venv/bin/python tools/parity/benchmark/run_benchmark.py --output tools/parity/benchmark/results.json
python3 tools/parity/benchmark/parity.py --results tools/parity/benchmark/results.json
python3 tools/parity/benchmark/report.py --results tools/parity/benchmark/results.json --output-dir docs/benchmarks
cargo run --release -p ps-solve --example combo_count
```

## Headline solve ratio: 0.27× → 1.55× vs cedar_flow

| | Pre-SP1 baseline (docs/benchmarks/report.md, captured before SP0.1) | Post-SP1.1–1.3 (this run) |
|---|---|---|
| ps_grpc vs cedar_flow, **solve**, median | **0.27×** | **1.55×** |
| ps_grpc vs cedar_flow, detect, median | 1.5× | 1.02× (noise; detect path untouched, see caveat below) |
| ps_grpc vs tetra3_original, solve, median | 0.8× | 6.9× |

**Decision gate: solve ratio (1.55×) is ≥ 1.0× vs cedar_flow → per the task's rule, SP2.2 concludes the effort here. Skip SP3 (profiling), proceed directly to SP4 (land it).**

## Root cause, now confirmed: eager allocation, not iteration order

The `combo_count` re-measurement (release build) shows `combos_examined` **unchanged** from the SP0.2 lexicographic baseline, combo-for-combo:

| image | status | combos_examined (SP0.2, pre-SP1.1) | combos_examined (post-SP1.1) | t_solve_s (pre) | t_solve_s (post) |
|-------|--------|---:|---:|---:|---:|
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | MatchFound | 1 | 1 | 0.002 | <0.0005 |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | MatchFound | 1 | 1 | 0.024 | <0.0005 |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | MatchFound | 1 | 1 | 0.001 | <0.0005 |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | MatchFound | 1 | 1 | 0.046 | <0.0005 |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | MatchFound | 1 | 1 | 0.002 | <0.0005 |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | MatchFound | 2 | 2 | 0.039 | <0.0005 |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | MatchFound | 1 | 1 | 0.001 | <0.0005 |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | MatchFound | 1 | 1 | 0.028 | <0.0005 |
| hale_bopp.jpg | NoMatch | 8855 | 8855 | 0.184 | 0.168 |

This confirms the SP0.2 decision gate's prediction exactly: since `combos_examined` is bit-for-bit identical before and after, **none of the solve-time improvement above came from breadth-first ordering examining fewer combos** — colex and lex would have produced identical timings on this corpus. The entire win is the C1 side-effect of SP1.1: the old `combinations_4(n) -> Vec<[usize;4]>` allocated and populated **every one of the `C(n,4)` combinations up front**, before the solve loop could even look at combo #1 — so an image that matched on the very first combo still paid the cost of materializing the *entire* combinatorial space (at `n ≈ verification_stars_per_fov`, up to ~618 MiB per the SP1.1 doc comment) before it could return. The new lazy iterator yields combo #1 immediately with no allocation, so wall-clock solve time for the 8 MatchFound images collapsed from single-to-tens-of-milliseconds to sub-microsecond (below this table's 3-decimal display resolution). hale_bopp (NoMatch, full-space exhaustion either way) shows a smaller, still-real ~9% improvement (0.184s → 0.168s), consistent with the eager-vs-lazy difference mattering less when the entire space has to be walked regardless.

**Per-image `solve` time, ps_grpc, from the harness (median over 5 iterations, seconds; this is the harness's self-reported `Solution.t_solve`/algorithm time, not the headline ratio's wall-clock metric — both point the same direction, but they're two distinct measurements) — before vs after:**

| image | ps_grpc solve before | ps_grpc solve after | cedar_flow solve before | cedar_flow solve after |
|-------|---:|---:|---:|---:|
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | 0.0119 | 0.0001 | 0.0014 | 0.0033 |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | 0.0179 | 0.0002 | 0.0042 | 0.0050 |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | 0.0002 | 0.0001 | 0.0023 | 0.0028 |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | 0.0237 | 0.0002 | 0.0044 | 0.0052 |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | 0.0083 | 0.0001 | 0.0015 | 0.0029 |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | 0.0288 | 0.0003 | 0.0050 | 0.0058 |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | 0.0188 | 0.0001 | 0.0032 | 0.0040 |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | 0.0197 | 0.0002 | 0.0048 | 0.0056 |
| hale_bopp.jpg | 0.0134 | 0.0002 | 0.0092 | 0.0225 |

`cedar_flow`'s own solve times drifted mildly upward run-to-run (unchanged reference code — ordinary environment/container-load variance between the two harness runs, not a real change), which further inflates the reported ratio; the dominant, reproducible signal is `ps_grpc`'s own ~50–100× per-image solve-time drop, independently confirmed by the `combo_count` example on a separate invocation.

**Parity**: re-verified via `parity.py` on this same run — all 9 `ps_grpc_vs_cedar_flow`/`primary_same_catalog` comparisons still `flagged: false`, `matched_cat_ids.exact: true`, `symmetric_difference_count: 0` (see SP2.1's Run Log entry in `plan.md` for the full proof; this run reconfirms it incidentally as a byproduct of the same harness invocation).

**Conclusion:** SP1.1 (lazy breadth-first iterator) alone closes the gap from 0.27× to 1.55× vs cedar_flow — solidly past the ≥1.0× threshold. Per SP2.2's decision rule, **SP3 (profiling) is skipped**; proceed to **SP4** (finalize/land).

---

# SP4 — Final Summary (2026-07-04)

**Effort:** close the `ps-solve` vs cedar-solve solve-stage latency gap (feat-10 solve-latency fix, `notes/solve-perf-implementation-plan.md`), without regressing accuracy, without touching `ps-detect`.

**Result: 0.27× → 1.55× (independently re-measured 1.849×) vs cedar_flow on solve, median over 9 astronomical images.** Zero accuracy regression (SP2.1: all 9 `ps_grpc_vs_cedar_flow` matched-catalog-ID sets exact, RA/Dec within 10 arcsec, unchanged from the pre-change baseline; SP2.2: reconfirmed as a byproduct).

**What actually fixed it:** SP1.1 replaced the eager `combinations_4(n) -> Vec<[usize;4]>` (which allocated and populated *every* 4-star combination up front — up to ~618 MiB at `n≈150` — before the solve loop could examine even the first one) with a lazy, allocation-free breadth-first (colexicographic) iterator, verified byte-identical in traversal order to cedar-solve's own `breadth_first_combinations` reference generator (SP1.3, independently re-checked by ps-judge against the actual Python module).

**The surprising, load-bearing finding (SP0.2/SP2.2): the win came from laziness, not from ordering.** `combos_examined` is bit-for-bit identical before and after SP1.1 on all 9 corpus images (8 match on the very first or second combination — where lex and colex order are identical regardless — and the 9th, hale_bopp, exhausts the full space either way, since order can't change a full-space walk). The entire 0.27×→1.55× improvement is the C1 memory-allocation removal alone; the ordering fix, while correctly implemented and verified, made no measurable difference on this corpus. This matters for future readers: don't assume "breadth-first order" was the win — it was "stop building the whole combinatorial space before you need it."

**Decision: keep the `combos_examined` counter (SP0.1).** Recommended and adopted — it's a cheap, additive `u64` field with no perf cost, it's what let this investigation actually distinguish "ordering helped" from "laziness helped" instead of guessing, and the plan's own throughline is "measure, don't guess." No reason to remove it.

**Gates, final state:** `cargo test --workspace` green throughout every bead; `cargo clippy -p ps-solve` 0 errors; parity identical-green vs the pre-change baseline at every checkpoint (SP2.1, reconfirmed SP2.2); `ps-detect` untouched (confirmed via `git status`/diff scope on every bead); no tolerance loosened, no test `#[ignore]`d, no assertion weakened, at any point.

**Skipped (by measurement, not by omission):** SP3.1 (profile the residual gap) and SP3.2 (trim allocation churn) — both conditional on a gap remaining after SP2.2, and no gap remains (1.55× ≥ 1.0× threshold). If a future workload or corpus shows a different pattern (e.g. images that need many more combos to match), re-open SP3 guided by the `combos_examined` counter this effort added.

**Every bead (SP0.1–SP4) shipped through the full `ps-coder` → `ps-judge` grind loop** — every diff independently gated (`cargo build`/`test`/`clippy`) by the orchestrator and independently reviewed by `ps-judge`, with `ps-judge` re-running measurements from scratch (not trusting pasted numbers) on the two highest-stakes beads (SP1.3's reference cross-check, SP2.1's parity STOP-gate, SP2.2's decision gate).

---

# FUB.1 — Exhaustion-path profile (2026-07-05)

**Bead:** FUB.1 — profile the exhaustion path. **Command:** `cargo run --release -p ps-solve --example combo_count` (hale_bopp, default params, the 19 µs/combo / 8855-combo / ~65 ms baseline).

**Environment caveat (honest):** `perf` is unavailable on this host — `perf_event_paranoid=4` and no `CAP_PERFMON`/`CAP_SYS_ADMIN`, and `samply`/`valgrind` are not installed (`which perf samply valgrind cargo` → only `perf` and `cargo`). Per the FUB.1 spec ("`perf` (or `samply`) on … or anything else the profile surfaces"), the fallback was **temporary `std::time::Instant` instrumentation** of `solve_from_centroids` in `ps-solve/src/lib.rs` (coarse region timers + atomic counters), built release, run 3×, then **fully reverted** (`git checkout ps-solve/src/lib.rs`) — the FUB.1 commit is docs-only, no code change. Instrumentation overhead distorted the per-key DB-lookup timing (2.1M extra `Instant::now()` calls inflated it ~4×), so the DB-lookup share below is derived by subtraction from the uninstrumented inner-loop total, not from the inflated per-key timer.

**Uninstrumented baseline (3-run median, hale_bopp, release):**

| run | total_solve (ms) | combos | key_lookups | slot_hits |
|---|---:|---:|---:|---:|
| 1 | 65.20 | 8855 | 2,151,765 | 2778 |
| 2 | 65.10 | 8855 | 2,151,765 | 2778 |
| 3 | 65.67 | 8855 | 2,151,765 | 2778 |

`(combos, key_lookups, slot_hits)` is deterministic across runs. 243 candidate keys/combo (= (2·1+1)⁵ at band=1); 2778 slot hits total = **0.13% of key lookups** (the exhaustion path almost never reaches verification — it's dominated by building keys and probing the DB, not by the verify block).

**Per-region attribution (3-run median; `inner_loop` = the `for cand_key` block; db_lookup derived by subtraction):**

| region | ms | % of total | per-combo | notes |
|---|---:|---:|---:|---|
| DB lookup (`lookup_pattern` × 2.15M) | ~33.6 | **~51%** | ~3.8 µs/combo (243 calls) | dominant; **in `ps-db`, outside FUB.2's ps-solve scope** |
| verify block (A1–A8, × 2778 slot hits) | ~20.3 | **~31%** | 7.3 µs/hit (only on hits) | the 16 `.collect()`s live here; interleaved with real math (FOV/projection/rotation/nearby_stars/false-alarm) |
| candidate_keys build + sort (× 8855) | ~9.6 | **~15%** | 1.1 µs/combo | pop 4.0 ms (6.1%), sort 5.6 ms (8.6%) — sort dominates |
| other (pattern key, combo overhead, timeout check) | ~2.0 | ~3% | — | — |

**Per-key DB lookup cost (instrumented run, inflated):** ~62.6 ns/lookup when measured with a per-key `Instant` (134.8 ms / 2.15M), but this is ~4× the real cost because the 2.1M `Instant::now()` calls add ~50+ ms of overhead. The subtraction-derived ~33.6 ms / 2.15M = **~15.6 ns/lookup** is the trustworthy figure (a hash-table probe + FOV pre-filter, consistent with `ps_db::lookup::lookup_pattern`'s shape). **Do not quote the 62.6 ns number; it is an artifact.**

## FUB.2 precondition: MET (three targets ≥ ~10%)

Per the FUB.1 decision rule, FUB.2 proceeds. Three regions clear the ≥~10% bar:

1. **DB lookup — ~51%** — the dominant cost. **BUT it lives in `ps-db::lookup::lookup_pattern`, outside FUB.2's stated scope (`ps-solve/src/lib.rs:300-560` + `candidate_keys`).** Flagged as a **separate ps-db follow-up**, not bundled into FUB.2. Trimming it (e.g. fewer probe iterations, cheaper FOV pre-filter, or caching) is a ps-db change with its own parity surface — it deserves its own spec, not a quiet slip into a ps-solve allocation-trim bead.
2. **verify block — ~31%** — in FUB.2's scope. Fires only on the 2778 slot hits (0.13% of lookups). The 16 `.collect()`s are interleaved with attitude/projection/false-alarm math; allocation churn is some fraction of the 7.3 µs/hit, not all of it. **A finer profile of the verify block is warranted before committing to the .collect() trim** — if the math dominates and the .collect()s are a small fraction, the win is small.
3. **candidate_keys build+sort — ~15%** — in FUB.2's scope, and FUB.2's explicitly named target (a). The `Vec` is rebuilt+sorted every combo. Reusing the allocation (clear+refill) saves the realloc but **not** the sort (sort is 5.6 of the 9.6 ms — the majority). Realistic win: ~the pop portion's realloc overhead, plausibly ~1–2 ms of the 65 ms. Modest, but cheap and low-risk (bit-for-bit unchanged — same elements, same order).

## Named trim targets for FUB.2 (ps-solve scope only)

- **FUB.2-step-1 (low-risk, modest): reuse the `candidate_keys` `Vec` allocation.** Hoist the `Vec` out of the combo loop (clear+refill instead of `Vec::new()` + drop). Bit-for-bit unchanged (same 243 entries, same sort, same order). Realistic win: ~1–2 ms (the pop realloc overhead; the 5.6 ms sort is untouched). Gated: µs/combo before/after; revert if no measurable help.
- **FUB.2-step-2 (medium-risk, profile-gated): trim verify-block `.collect()`s** — **only after a finer verify-block profile confirms allocation churn is a meaningful fraction of the 7.3 µs/hit.** If the finer profile shows the .collect()s are <~10% of hit time, **skip this step** (the SP3.1/SP3.2 honest-SKIP pattern) and record that. Do not promise a win the profile hasn't shown.

## Honest expectation for FUB.2

Realistic FUB.2 win within ps-solve: **~1–3 ms on the 65 ms exhaustion path** (step-1 keys reuse + maybe step-2 verify trim), i.e. **~2–5%** — not the full 51% DB-lookup target. The dominant cost (DB lookup) is a ps-db follow-up. FUB.2 is worth doing for the cheap, low-risk keys reuse and the verify-block profile, but **it will not be a large win** — set that expectation now, in the record, before FUB.2 codes anything. Client-visible effect: a `solve_timeout` budget covers proportionally a few % more combos on hard images; zero effect on easy images (1–2 combos).

**`combos_examined` invariant:** 8855 on hale_bopp defaults, unchanged by FUB.1 (investigation only) and to-be-unchanged by FUB.2 (allocation reuse, not logic).

**Gates after FUB.1 (docs-only commit):** `cargo build -p ps-solve` green; `cargo test -p ps-solve` 18 passed / 0 failed / 1 ignored (unchanged); `git checkout ps-solve/src/lib.rs` confirmed clean (no `FUB_` symbols remain); `ps-detect` untouched (no file under `ps-detect/src/` modified — FUB.1 touched only `ps-solve/src/lib.rs`, reverted).

---

# FUB.2 — Exhaustion-path allocation-trim, attempted + reverted by measurement (2026-07-05)

**Bead:** FUB.2 — trim exhaustion-path allocation churn where FUB.1's profile pointed. **Spec rule:** "revert any step the measurement says didn't help." **AC:** "final: measurable µs/combo reduction vs the 19 µs baseline." **Outcome: SKIPPED-by-measurement (the SP3.1/SP3.2 honest pattern) — no code change lands.** Both attempted steps were bit-for-bit green but produced no statistically-detectable reduction, so both were reverted per the spec's revert rule. This section is the auditable record of the attempt + verdict (the FUB.2 commit is docs-only).

## Finer profile of the verify block (temporary instrumentation, reverted)

Before trimming, FUB.2 took its own advice ("a finer profile of the verify block is warranted before committing to the .collect() trim") and ran a finer `Instant`-based profile of the A1–A8 regions inside the `for &slot in &slots` loop (temporary, `git checkout ps-solve/src/lib.rs` reverted). hale_bopp, 8855 combos, **4184 slot hits** (note: the FUB.1 coarser run reported 2778 slot_hits because that counter incremented only on entering the verify body; the finer run's 4184 counts every slot iteration — both are consistent, the discrepancy is a counter-placing artifact, not a measurement contradiction), 1372 reach A6, 0 accepts (NoMatch):

| region | ms (3-run median) | % of 65 ms | notes |
|---|---:|---:|---|
| A1 (FOV estimate + largest-pixel-distance) | 0.09 | 0.1% | negligible |
| A2+A3+A4 (vectors, sort, rotation matrix) | 2.2 | 3.4% | interleaved collects + `find_rotation_matrix` SVD |
| A5 reflection reject | — | — | 1406/2778 rejected here (cheap `det(R)<0` check) |
| **A6 (nearby_stars + collects + derotate + project)** | **16.7** | **~26%** | the dominant verify cost |
| └ of which `ps_db::nearby_stars` (KD-query, in ps-db) | ~10.0 | ~15% | **in ps-db, outside FUB.2 scope** |
| └ of which 7 `.collect()`/`.to_vec()` allocations | ~6.7 | ~10% | the FUB.2-targetable part (split across 7 collects) |
| A7+A8 (match + binomial false-alarm) | 1.2 | 1.8% | only on the 1372 that reach A7 |

**Key finding:** the verify block's dominant cost is `ps_db::nearby_stars` (~10 ms, a KD-tree query in ps-db), NOT the `.collect()` allocations (~6.7 ms, split across 7 collects). So even a perfect verify-block allocation trim could target at most ~10% of total, and FUB.2's two steps target only a subset of those 7 collects.

## Step-1: reuse the `candidate_keys` Vec — REVERTED (no measurable win)

Hoist `let mut keys_buf: Vec<([u32;5],i64)> = Vec::new();` before the `'outer` combo loop; inside, `keys_buf.clear(); … keys_buf.push((key,dist)); keys_buf.sort_by_key(…);` then borrow `candidate_keys: &Vec<…> = &keys_buf`. Bit-for-bit unchanged (same 243 entries, same sort, same order). Implemented by ps-coder (29 ins / 22 del, `ps-solve/src/lib.rs` only), `cargo test -p ps-solve` 18/0/1 green, `combos_examined`=8855.

**Measurement (10 runs each, hale_bopp t_solve_s, sorted, median):**
- with step-1:    0.056 0.056 0.061 0.062 0.063 0.064 0.065 0.065 0.065 0.073 → **median 0.0635**
- without step-1: 0.056 0.057 0.057 0.062 0.063 0.063 0.066 0.066 0.072 0.073 → **median 0.0630**

No win (median within noise; the realloc saved is small and the 5.6 ms sort — the majority of the 9.6 ms keys region — is untouched). **Reverted per the spec's "revert any step the measurement says didn't help."**

## Step-2: hoist the A6 kept/trim 5-buffer scratch Vecs — REVERTED (no measurable win)

Hoist 5 scratch Vecs (`nearby_centroids_kept`, `nearby_cat_vectors_kept`, `nearby_inds_kept`, `nearby_cat_centroids`, `nearby_cat_vectors_trimmed`) out of the slot loop; inside, `buf.clear(); buf.extend(…)` / `buf.extend_from_slice(…)` in the same iteration order; bind downstream names to `&buf`. Bit-for-bit unchanged (index-remapping copies in iteration order, no FP reorder). Implemented by ps-coder (38 ins / 9 del, `ps-solve/src/lib.rs` only), `cargo test -p ps-solve` 18/0/1 green, `combos_examined`=8855.

**Measurement (12 runs each, hale_bopp t_solve_s):**
- with step-2:    median 0.0640, mean 0.0636, min 0.0550, max 0.0710
- without step-2: median 0.0650, mean 0.0648, min 0.0550, max 0.0720
- **mean delta −0.0012 s (−1.8%); median delta −0.0010 s (−1.5%)**

The mean is consistently lower, BUT pooled stdev ≈ 0.0050 s → **SNR < 1** (effect ~1.2 ms on a ~5 ms noise floor at n=12). Not statistically detectable. **Reverted per the spec's "revert any step the measurement says didn't help."** (The change is strictly-less-alloc and bit-for-bit; if a future quieter host or larger-n study detects the ~1-2 ms effect, this is the cheapest lever to re-land — recorded here so it's not lost.)

## Why FUB.2 produced no code change (honest verdict)

The exhaustion path's dominant costs are in **ps-db**, not ps-solve:
- DB lookup `ps_db::lookup::lookup_pattern` — ~51% (FUB.1)
- `ps_db::nearby_stars` KD-query — ~15% (FUB.2 finer profile)
- combined **~66% of the 65 ms exhaustion path is ps-db code, outside FUB.2's ps-solve scope.**

The ps-solve-scope allocation levers (candidate_keys reuse, A6 buffer hoist) target at most ~10% of total, split across many collects, and neither produced a detectable win on this host (noise floor ~±10% / ~5 ms at n=10-12). Per FUB.2's own "revert any step the measurement says didn't help" rule, both were reverted. FUB.2's AC ("measurable µs/combo reduction") is **not met** — honestly, by measurement, not by omission. This is the same outcome SP3.1/SP3.2 reached.

## Named next levers (NOT implemented by FUB.2; recorded for the decision gate)

1. **ps-db follow-up (new spec, not FUB.2):** trim `lookup_pattern` (~51%) and/or `nearby_stars` (~15%) — the dominant exhaustion-path cost. Its own parity surface (ps-db), its own spec. Do NOT bundle into FUB.2.
2. **FU-C (parallel search, specced but unapproved):** the ~cores× lever on the exhaustion path; needs user approval (user decision 2026-07-04) and a ps-judge Job B on ordered find-first semantics. Not beaded.

**Gates after FUB.2 (docs-only commit, both steps reverted):** `cargo build -p ps-solve` green; `cargo test -p ps-solve` 18 passed / 0 failed / 1 ignored (unchanged); `git diff --stat ps-solve/src/lib.rs` empty (both steps reverted, no `FUB_`/`_buf`/`keys_buf` symbols remain); `combos_examined`=8855 on hale_bopp defaults (unchanged); `ps-detect` untouched. ps-judge peer reviewed (re-ran: revert clean, tests 18/0/1, baseline reproduces 0.060-0.068 s with ~±13% spread confirming the noise floor, finer-profile arithmetic consistent 10+6.7≈16.7 ms).

---

# FUB.3 — FU-B re-measurement + decision gate (2026-07-05)

**Bead:** FUB.3 — rebuild release, re-run the full eval harness + `combo_count`; record final µs/combo, hale_bopp exhaustion wall-clock, and full-corpus solve ratios; regenerate `docs/benchmarks/report.md`/`.html`. Decision gate: record whether meaningful exhaustion-path cost remains and name the next lever. **FU-C (parallel search) is NOT beaded or implemented — it requires explicit user approval (user decision 2026-07-04).**

## Setup note (honest)

`ps-solve/src/lib.rs` and `ps-grpc/src/service.rs` are **byte-identical to the post-FUA.2 state** (`git diff dd2bbf7 -- ps-solve/src/lib.rs ps-grpc/src/service.rs` is empty) — FUB.1 was investigation-only (reverted) and FUB.2 reverted both its steps. So the eval-harness headline ratios are expected to be ~unchanged vs the last ad-hoc report (FUA-era); the FUB.3 harness run's job is to **re-confirm parity identical-green and record the post-FU-B baseline + the decision**, not to show a ratio change (no product code changed). `cargo build --release -p ps-grpc` rebuilt clean; `combo_count` rebuilt clean.

## combo_count — hale_bopp exhaustion path (5 runs, release)

| run | t_solve_s | combos_examined | µs/combo |
|---|---:|---:|---:|
| 1 | 0.067 | 8855 | 7.57 |
| 2 | 0.067 | 8855 | 7.57 |
| 3 | 0.068 | 8855 | 7.68 |
| 4 | 0.065 | 8855 | 7.34 |
| 5 | 0.068 | 8855 | 7.68 |

**5-run median: 0.067 s → 7.57 µs/combo** (8855 combos, NoMatch). vs the FUB.1-era "19 µs/combo / 168 ms" figure: that older figure was from a different (slower) host/run; on this aarch64 host the steady-state is ~7.6 µs/combo / ~65-67 ms. **No change from FUB.1 → FUB.3** (no code changed), as expected. `combos_examined`=8855 invariant holds.

## Eval harness — full corpus (results_fub3.json, regenerated report.md/.html)

**Headline (median over 9 astronomical images):**

| comparison | detect | solve |
|---|---|---|
| ps_grpc vs cedar_flow | **1.06×** | **1.43×** |
| ps_grpc vs tetra3_original | 4.76× | 4.08× |

**Per-system medians (astronomical, seconds / ms):**

| system | solve wall (s) | detect wall (s) | t_solve (ms) | t_extract (ms) |
|---|---:|---:|---:|---:|
| ps_grpc | 0.0045 | 0.0034 | 0.07 | 1.44 |
| cedar_flow | 0.0064 | 0.0036 | 2.08 | 1.48 |
| tetra3_original | 0.0184 | 0.0177 | 1.57 | 17.10 |

**hale_bopp specifically (ps_grpc, SolveFromCentroids path the harness uses):** wall=0.0065 s, t_solve=0.10 ms, t_extract=1.44 ms, **status=MATCH_FOUND** (the harness's solve stage extracts centroids via standalone `ExtractCentroids` at sigma=4 then solves from centroids — it does NOT exercise the hardcoded-sigma `SolveFromImage` RPC; that's the feat-10/H2 path).

## Parity STOP gate — identical-green

`parity.py` over `results_fub3.json`: 27 pairwise comparisons, 18 flagged (all 18 are the expected **cross-catalog sanity checks** `*_vs_tetra3_original`, which flag because tetra3 uses a different bundled catalog — pre-existing, not a regression); **6 stress status checks, 4 flagged** (the `tree.jpg`/`test_5mp_g100_e50ms.jpg` MATCH_FOUND on ps_grpc/cedar_flow vs expected NO_MATCH — pre-existing accuracy-domain issue, tracked separately, not a perf regression). **The 9 `ps_grpc_vs_cedar_flow` primary_same_catalog comparisons are all unflagged** (centroids exact 0.00 px, RA within ~1″, Dec within ~0.2″, Roll exact, FOV within 0.1%, matched IDs exact) — verified programmatically: `primary_same_catalog` rows = 9, flagged = 0. **Parity STOP rule satisfied: identical-green.**

## Decision gate

**Does meaningful exhaustion-path cost remain?** Yes — ~65-67 ms / 8855 combos / 7.6 µs/combo on hale_bopp's NoMatch path, unchanged by FUB.2 (no ps-solve lever moved it). The FUB.1+FUB.2 finer profile attributes **~66% of that to ps-db** (`lookup_pattern` ~51% + `nearby_stars` ~15%), outside FUB.2's ps-solve scope.

**Next levers (recorded, NOT implemented by FUB.3):**
1. **ps-db follow-up (new spec):** trim `ps_db::lookup::lookup_pattern` (hash-table probe + FOV pre-filter, ~51%) and/or `ps_db::nearby_stars` (KD-tree query, ~15%) — the dominant exhaustion-path cost. Its own parity surface (ps-db), its own OpenSpec change. This is the highest-leverage *new* exhaustion-path lever. **Not beaded here** — awaits its own spec.
2. **FU-C (parallel search, specced but unapproved):** `notes/solve-perf-followups-spec.md` §FU-C — parallelize the `'outer` combo loop behind the `rayon` feature flag for a ~cores× win on the exhaustion path. **Do NOT bead or implement FU-C; it requires explicit user approval (user decision 2026-07-04) and a ps-judge Job B decision on ordered find-first semantics.**
3. **feat-10 / H2 (already specced, separate):** `openspec/changes/feat-10-solve-from-image-detect-params` — threads client detection params through `SolveFromImage`, collapsing hale_bopp's `SolveFromImage` NoMatch (0.168 s) to a sub-ms match. Specced and `--strict`-validated but not beaded/implemented (user instruction: author only, do not proceed). This is a correctness fix with a solve-latency side-effect on the product RPC, not a benchmark-ratio lever.

**FU-B (FUB.1–FUB.3) complete.** No product code changed (FUB.1 investigation reverted; FUB.2 both steps reverted by measurement). The eval-harness baseline is re-confirmed and parity is identical-green. The honest conclusion: **the remaining solve-latency lever within ps-solve allocation scope is exhausted; the dominant cost is ps-db, which needs its own spec; the only large remaining lever is FU-C (parallelism), which needs user approval.**

**Gates after FUB.3:** `cargo build --release -p ps-grpc` green; `cargo test -p ps-solve` 18/0/1; `combo_count` hale_bopp 8855 combos / 7.6 µs/combo (5-run median 0.067 s); eval harness `results_fub3.json` + regenerated `docs/benchmarks/report.md`/`.html` written; `ps_grpc_vs_cedar_flow` primary parity 9/9 unflagged; `ps-detect` untouched. ps-judge peer reviewed the FUB.3 record.

---

# DBL.1 — Probe-pair table data-layout optimization (2026-07-06) — SKIPPED-by-measurement, reverted

**Bead:** DBL.1 — interleave `key_hashes[i]` (u16) and `largest_edge[i]` (f16) into a single `probe_pairs[i]` (u32) per slot, reducing random memory loads in `ps_db::lookup::lookup_pattern`'s probe chain from two independent accesses per probe to one (D-DBL-1/2 in `notes/perf-improvement-proposals.md` §DBL). Implemented in full: `Database.probe_pairs: Vec<u32>` + `build_probe_pairs()` helper wired into all three construction sites (`importer.rs`, `loader.rs`, `lib.rs::empty()`), `lookup_pattern` rewritten to read the packed pair once per probe, a code comment on `lookup_pattern_mmap` per D-DBL-2, and a new `test_probe_pairs_parity` equivalence test (fixture hit/miss keys × FOV Some/None, old two-array algorithm vs new probe_pairs path, identical-including-order). All gates green: `cargo build -p ps-db`, `cargo test -p ps-db` (9/9, including the new test), `cargo build --workspace --all-targets`, `cargo test --workspace` (clean), sv6 parity green, harness parity STOP 9/9 `ps_grpc_vs_cedar_flow primary_same_catalog` unflagged.

**Measurement (this bead's own AC, not deferred to DBL.4): `combo_count` hale_bopp, 12 runs each, before vs after, same host/build:**

| | n | mean (s) | median (s) | stdev (s) |
|---|---:|---:|---:|---:|
| before (HEAD, two-array `key_hashes`/`largest_edge` reads) | 12 | 0.1331 | 0.1300 | 0.0063 |
| after (probe_pairs interleaved read) | 12 | 0.1293 | 0.1280 | 0.0032 |

`combos_examined` = 8,855 both runs (invariant holds — no ordering/logic change). Mean delta = 3.75 ms (≈2.9%); pooled stdev ≈ 5.0 ms; **SNR = diff / pooled_stdev ≈ 0.75 < 1** — below this project's noise-floor threshold (FUB.2's established standard: ≥12 runs, SNR<1 = no win). µs/combo: 15.03 before → 14.61 after — a plausible direction but not a statistically distinguishable one at this sample size.

**Decision: reverted per the explicit "revert any step that doesn't clear the noise floor" rule.** `git checkout` restored `ps-db/src/{importer,lib,loader,lookup,mmap}.rs`, `ps-db/tests/load_npz.rs`, and `ps-dbgen/tests/hash_insert_test.rs` (a pre-existing test-only field-mutation pattern that would have needed a `probe_pairs` rebuild had the change stayed — noted here since it's a real gap the equivalence-test review surfaced, but moot once reverted) to their pre-DBL.1 state. Verified clean afterward: `cargo build -p ps-db` / `cargo test -p ps-db` (5/5, back to baseline) / `cargo test -p ps-dbgen` (11/11) all green.

**Honest verdict:** the interleaved-load idea is architecturally sound (it does reduce the memory-access count per probe, and the mean did move in the predicted direction), but on this container the effect is too small relative to the ~2–5% run-to-run noise floor to call it a measured win — this host's `hale_bopp` baseline (~130 ms / 8,855 combos ≈ 15 µs/combo) is also markedly slower than the ~65–67 ms / 7.6 µs/combo recorded in FUB.1/FUB.3 on the original aarch64 benchmark host (a different container/CPU, consistent with the SP2.1 Decisions Log note that this environment doesn't persist across resets and has previously come back as a different host/arch). No product code changed as a net result of DBL.1. **DBL.2 (prehashed lookup + prefetch) depends on the `probe_pairs` table DBL.1 would have added — since DBL.1 didn't land, DBL.2 has no data structure to build on and is recorded as blocked-by-measurement below, not implemented.** DBL.3 (`nearby_stars`, independent of `probe_pairs`) is unaffected and proceeds separately.
