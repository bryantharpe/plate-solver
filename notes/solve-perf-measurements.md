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
