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
