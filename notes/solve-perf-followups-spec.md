# Solve-perf follow-ups — implementation specs (FU-A / FU-B / FU-C)

_Authored 2026-07-04, immediately after the SP0.1–SP4 solve-perf effort landed (see
`notes/solve-perf-measurements.md` for its final summary). **Status: specs only —
no `plan.md` beads exist for these yet.** When approved, convert each spec into
beads the same way `notes/solve-perf-implementation-plan.md` became SP0.1–SP4,
run through the same `ps-coder` → `ps-judge` grind loop._

## Where the time goes now (measured 2026-07-04, post-SP4)

The SP effort made the solve stage itself fast (0.27× → 1.55–1.85× vs cedar_flow).
A fresh per-request breakdown from the harness's `results.json` (medians, ms;
`overhead` = wall_clock − t_solve − t_extract, i.e. transport + proto + copies +
decode + python-client time):

| image (MatchFound, ps_grpc) | wall | t_solve | t_extract | overhead |
|---|---:|---:|---:|---:|
| Alt40_Azi-135 | 7.04 | 0.086 | 3.12 | 3.84 |
| Alt40_Azi135 | 7.58 | 0.167 | 3.25 | 4.16 |
| Alt40_Azi-45 | 6.16 | 0.081 | 2.41 | 3.67 |
| Alt40_Azi45 | 7.51 | 0.190 | 3.32 | 4.00 |
| Alt60_Azi-135 | 6.87 | 0.078 | 2.91 | 3.88 |
| Alt60_Azi135 | 6.72 | 0.240 | 2.37 | 4.11 |
| Alt60_Azi-45 | 7.50 | 0.097 | 3.19 | 4.22 |
| Alt60_Azi45 | 8.70 | 0.160 | 4.43 | 4.12 |
| hale_bopp | 10.44 | 0.263 | 3.25 | 6.93 |

**The solver is now <4% of client-perceived latency.** The remaining levers are
(a) per-request overhead/copies, (b) the worst-case *exhaustion* path (NoMatch /
weak-signal images: `combo_count` with default params shows hale_bopp walking
8,855 combos in ~168 ms ≈ **19 µs/combo** — that per-combo cost is what a
5-second `solve_timeout` budget buys), and (c) parallelism.

**Constraints carried forward from the SP effort, unchanged:** no accuracy
regression (tolerances fixed in `openspec/IMPLEMENTATION-STATUS.md`; never
loosen/`#[ignore]`/stub); `ps-detect` internals untouched; measure before/after
every phase with the eval harness; a parity STOP rule identical to SP2.1's.

## Ordering (ease of implementation vs impact)

| ID | Title | Effort | Impact | Why this order |
|---|---|---|---|---|
| FU-A | Kill per-request image copies + wire real `t_extract_ms` | **S** | Moderate (wall-clock) + unlocks measurement | Mechanical, single file, no algorithm risk; also removes a Known Limitation that currently blinds all future per-stage measurement |
| FU-B | Verification-path allocation trim (SP3.2 revived, profile-gated) | **M** | Medium–high on the worst-case/exhaustion path only | Bounded scope, bit-for-bit constraint already spelled out by SP3.2's original text; measurable via `combos_examined`-normalized µs/combo |
| FU-C | Parallel combination search behind the `rayon` flag (extends H6) | **L** | Large (~cores×) on exhaustion; zero on easy images | Highest risk: ordered find-first semantics must be proven parity-identical; needs a ps-judge Job B decision before any code |

---

## FU-A — Eliminate per-request image copies in `ps-grpc` + wire real `t_extract_ms`

**Motivation (measured):** every request currently carries ~3.7–4.2 ms of
overhead beyond extract+solve. Not all of that is ours (python client, HTTP/2),
but the parts that are, are pure waste, and we currently **cannot see** our own
extraction time in `SolveFromImage` because it's hardcoded:
`ps-grpc/src/service.rs:336-339` — "`t_extract_ms = 0.0` since solve_from_image
handles extraction internally" (also a Known Limitation in
`docs/benchmarks/report.md`).

**Scope (all in `ps-grpc/src/service.rs` + one additive field in `ps-solve`):**

1. **Inline path double-copy** — `input_image.image_data.clone()` at `:131`
   (ExtractCentroids) and `:307` (SolveFromImage). We own the decoded request;
   take the buffer instead of cloning it (`std::mem::take` on the owned message
   field, restructuring the borrow so `input_image` is taken by value). Saves a
   full-frame memcpy (~0.79 MB at 1024×768) per request. `GrayImage::from_raw`
   (`:150`, `:326`) already takes ownership — no copy there.
2. **Shmem path copy** — `mmap.to_vec()` at `:129`/`:305` copies the whole
   frame, defeating the zero-copy purpose of shared memory. Full fix requires a
   borrowed-image path through `ps-detect`'s API (out of scope — `ps-detect`
   untouched). Spec: keep the copy, add a code comment stating why it exists and
   what unblocking it requires, and **measure** whether the shmem path is even
   exercised by any current client before investing more.
3. **Real `t_extract_ms`** — additive `pub t_extract: f64` (seconds, 0.0 default)
   on `ps_solve::Solution`, set by `solve_from_image` around its internal
   detection call (same pattern as SP0.1's `combos_examined`: additive field,
   all construction sites updated, no behavior change). `service.rs`
   SolveFromImage then reports `sol.t_extract * 1000.0` instead of `0.0`, and
   the Known Limitations entry in the harness/report is updated/removed.

**Accuracy gates:** `cargo test --workspace` green; sv6 parity tests green; the
harness parity table identical-green (this touches no math — any parity change
means a bug, STOP). New tests: ExtractCentroids/SolveFromImage results
byte-identical before/after the copy removal on a fixture image;
`t_extract_ms > 0` on a real SolveFromImage.

**Measurement plan:** re-run the harness; record wall-clock and the new
per-stage split in `notes/solve-perf-measurements.md`. Honest expectation:
**~0.3–1 ms/request** from the clone removal (memcpy + allocator pressure) —
moderate, but the real payoff is that per-stage timing becomes trustworthy for
FU-B/FU-C and for the detect-stage work H2 will eventually want.

---

## FU-B — Verification-path allocation trim (SP3.2 revived, now profile-justified)

**Motivation (measured):** the exhaustion path costs ~19 µs/combo (hale_bopp,
8,855 combos, 168 ms, `combo_count` defaults). Two allocation sources run
per-combo on that path:

- `candidate_keys` (`ps-solve/src/lib.rs:232-255`): a fresh `Vec` of
  (2·band+1)⁵ = 243 entries (band=1) is built **and sorted** for every single
  combo. The SP plan marked this "immaterial" — true for the MatchFound path
  (1–2 combos), **not** for the exhaustion path (8,855 rebuilds+sorts).
- The verification block (`lib.rs:300-560`) contains **16 `.collect()` calls**
  that allocate per candidate-key *hit* (nearby-star filtering, catalog-vector
  mapping, matched-pair extraction).

**Scope:** exactly SP3.2's original text, now with its precondition met by
measurement rather than skipped: profile a release exhaustion run first
(`perf`/`samply` on `combo_count` over hale_bopp with default params — SP3.1's
method), then hoist scratch buffers / reuse the `candidate_keys` allocation
(clear+refill instead of realloc; note the *sort* may dominate the alloc — the
profile decides) **only where the profile shows real cost**. Small,
individually gated steps; revert any step the benchmark says didn't help.

**Hard constraint (verbatim from SP3.2):** numerical results bit-for-bit
unchanged — buffer reuse must not reorder floating-point operations; if a
refactor would change summation order, don't do it that way.

**Accuracy gates:** per step: sv6 parity green, `cargo test --workspace` green,
harness parity table identical-green, `combos_examined` unchanged (it counts
iterations, not allocations — any change means a logic bug, STOP).

**Measurement plan:** the metric is **µs/combo on the exhaustion path**
(`combo_count` hale_bopp: t_solve ÷ combos_examined), before/after each step,
recorded in `notes/solve-perf-measurements.md`. Target: meaningful reduction
from 19 µs/combo (set the concrete bar after the profile; don't promise a
number the profile hasn't shown). Client-visible effect: a `solve_timeout`
budget covers proportionally more combos → fewer spurious Timeout results on
hard images; zero effect on easy images (already 1–2 combos).

---

## FU-C — Parallel combination search behind the `rayon` feature flag (extends H6)

**Motivation:** after FU-B, the exhaustion path is still serial. Cores are idle
during the one part of a solve that can take seconds. Cedar-solve is also
serial here, so this is a chance to *lead* the reference on worst-case latency
rather than chase it.

**Scope:** parallelize the `'outer` combination loop in `solve_from_centroids`
(`ps-solve/src/lib.rs:194`) behind the `rayon` cargo feature that **H6 already
specs** (default off, off on mobile per PRD §mobile-runtime — this spec extends
H6's flag with actual parallelism, so sequence it after H6 lands, and do not
duplicate H6's flag/deadline work).

**The load-bearing design problem (needs `ps-judge` Job B before any code):**
the current semantics are "first acceptable match in breadth-first order wins,"
and SP2.1 proved those semantics parity-identical to cedar. A naive
`par_iter().find_any()` breaks that (returns *an* acceptable match, not the
*first*). Spec the ordered variant: split the colex stream into fixed chunks of
K combos; verify chunks in parallel; accept the acceptable match with the
lowest global combo index; cancel outstanding work past that index
(`find_first`-with-chunking, or rayon's `find_first` directly if its
sequential-consistency guarantee is confirmed sufficient). Timeout/cancel
checks move to per-chunk granularity — Job B must also rule on whether that
coarsening (up to K−1 extra combos examined after a cancel) is acceptable and
pick K. `combos_examined` becomes an `AtomicU64` or per-chunk sum — exactness
under early-exit must be specified, or the field documented as
approximate-under-rayon.

**Accuracy gates:** with the flag **off**: byte-identical behavior, all
existing gates (this must be provable from the diff — the serial path stays the
default compile path). With the flag **on**: sv6 parity tests + full harness
parity table must produce the **same matched results** as serial on all 9
images, plus a dedicated determinism test (same input solved twice under rayon
→ identical `Solution` including matched-ID order). Any divergence: STOP.

**Measurement plan:** exhaustion-path wall-clock (hale_bopp `combo_count`,
flag on vs off) and full harness with flag on. Expected: near-linear on
exhaustion (~cores× — this host has 20), ~zero on easy images (1–2 combos
can't parallelize); report both honestly.

**Effort/risk:** largest of the three — touches solve semantics, adds a
concurrency surface, and its correctness argument (ordered find-first ≡ serial
first-match) is exactly the kind of thing `ps-judge` exists to adjudicate.
Do not start FU-C until FU-B's serial per-combo cost is settled, or the
parallel speedup will be measured against an unoptimized baseline.

---

## Non-goals (all three specs)

- `ps-detect` internals (detect is ~1.3–3 ms and at parity with cedar-detect;
  also explicitly out of scope per the standing constraint).
- Accuracy/matching behavior changes of any kind — including "fixing"
  hale_bopp's default-params NoMatch (that's H2, already specced) and the
  stress-image false-positive MATCH_FOUND on tree.jpg (pre-existing,
  accuracy-domain, deserves its own investigation, not a perf task).
- Python-client / harness-side overhead (not product code).
