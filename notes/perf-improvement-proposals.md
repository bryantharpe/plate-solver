# Performance improvement proposals — solve & detect (2026-07-06)

_Three concrete, code-grounded improvements to speed up solve or detect, spanning
the main solve logic and the gRPC integration. Written after the SP0.1–SP4 and
FU-A/FU-B efforts landed; grounded in the measurements recorded in
`notes/solve-perf-measurements.md` (FUB.1 profile, FUB.3 baseline) and in the
current code. **Status: proposals only — nothing here is beaded or implemented.**_

## Context: where the time goes today

Post-FU-B baseline (FUB.3, 2026-07-05):

- **MatchFound path is essentially solved** — 8/9 corpus images match on combo
  #1–2, `t_solve` ≈ 0.07 ms median, 1.43× vs cedar_flow.
- **The exhaustion path (NoMatch / weak-signal) is the remaining solve cost** —
  hale_bopp walks 8,855 combos in ~65–67 ms (≈7.6 µs/combo). The FUB.1/FUB.2
  profiles attribute **~66% of that to `ps-db`** (`lookup_pattern` ~51%,
  `nearby_stars` ~15%) — outside the scope of every effort so far. This is what
  a client's `solve_timeout` budget actually buys on hard images.
- **ps-solve-scope allocation levers are exhausted** — FUB.2 attempted both
  named trims and honestly reverted them by measurement. Don't re-litigate.
- Detect is at parity with cedar-detect (1.06×) and `ps-detect` internals were
  a standing non-goal for prior efforts; proposal 3 touches only its API
  surface (storage genericity), not its algorithms.

---

## Proposal 1 — Trim `ps_db::lookup::lookup_pattern`, the single largest solve cost (~51% of the exhaustion path)

**The measured case (FUB.1):** the exhaustion path performs **2,151,765**
`lookup_pattern` calls per hale_bopp solve (243 candidate keys × 8,855 combos)
at ~15.6 ns each ≈ **~33.6 ms of the 65 ms total**. Only 0.13% of lookups ever
produce a candidate slot — the cost is almost entirely hashing + probing, i.e.
random memory access. This was explicitly flagged in FUB.1/FUB.3 as "the
highest-leverage new exhaustion-path lever… deserves its own spec" and never
picked up.

**Concrete steps (`ps-db/src/lookup.rs`, `ps-db/src/layout.rs`/loader):**

1. **Fuse the two probe arrays into one.** Every probe iteration reads
   `db.key_hashes[slot]` (u16) *and* `db.largest_edge[slot]` (f16) — two
   independent random accesses into two large parallel arrays
   (`lookup.rs:72`), so a cold probe costs two cache misses instead of one.
   Build (at load time — no on-disk format change needed) an interleaved
   `Vec<(u16, u16)>` of `(key_hash, largest_edge_bits)`; the empty-slot check,
   the 16-bit pre-filter, and the FOV pre-filter then all hit the same 4-byte
   pair on one cache line. Expected: up to ~2× fewer misses on the dominant
   probe loop.
2. **Batch and prefetch across the 243 candidate keys.** The caller
   (`ps-solve/src/lib.rs:264`) issues the 243 lookups one at a time; each
   starts with a dependent chain of hash → index → load. Compute all 243
   `hash_index`es up front (pure function of the key, no memory traffic), then
   probe with software prefetch (`core::arch::…::_mm_prefetch` /
   `prefetch_read_data`) issued 1–2 keys ahead, hiding the miss latency behind
   hash computation for the next key.
3. **Small mechanical trims while in there:** hoist `coarse_fov_rad.unwrap()`
   and the filter constants out of the probe loop (`lookup.rs:85-86`); return
   candidates into a caller-provided scratch `&mut Vec<usize>` (or a
   `SmallVec`) so the rare hit path doesn't allocate.
4. **Sibling target, same spec:** `ps_db::nearby_stars` (~15% of exhaustion,
   `ps-db/src/lib.rs:189`) — reuse a scratch buffer for the
   `collect` + `sort_unstable`, and evaluate kiddo's `within` vs
   `within_unsorted`+sort. Smaller win, same parity surface, same bead.

**Gates (same regime as SP/FU):** candidate slot lists bit-identical on a
fixture sweep; `combos_examined` = 8,855 invariant on hale_bopp defaults; sv6
parity + harness parity identical-green; metric is µs/combo (`combo_count`)
before/after each step, revert what doesn't measure.

**Honest expectation:** the probe loop is memory-bound, so steps 1+2 are the
first levers with real headroom against the ~33.6 ms; even a 30–50% reduction
in lookup cost is ~10–17 ms off the 65 ms exhaustion path (~15–25%) — more
than everything FU-B could reach combined. Zero effect on easy images.

---

## Proposal 2 — FU-C: parallel combination search behind the `rayon` flag (the ~cores× lever)

**This is the already-specced FU-C** (`notes/solve-perf-followups-spec.md`
§FU-C) — recorded in FUB.3 as one of two remaining levers, blocked on explicit
user approval (user decision 2026-07-04) and a ps-judge Job B ruling on ordered
find-first semantics. It is *the* large remaining lever: after proposal 1, the
exhaustion path is still serial while every core but one idles through the one
part of a solve that can take seconds. Cedar-solve is serial here too — this is
a chance to lead the reference on worst-case latency, not chase it.

**Design problem to settle first (per the spec):** current semantics are
"first acceptable match in breadth-first order wins," proven parity-identical
to cedar in SP2.1. A naive `par_iter().find_any()` breaks that. The spec'd
shape: split the colex combo stream into fixed chunks of K; verify chunks in
parallel; accept the acceptable match with the lowest global combo index;
cancel work past it (`find_first`-with-chunking). Timeout checks coarsen to
per-chunk; `combos_examined` becomes atomic or documented
approximate-under-rayon.

**Scope guards:** default-off cargo feature (off on mobile per PRD
§mobile-runtime); flag-off must be provably byte-identical from the diff;
flag-on must produce the same matched results on all 9 images plus a
determinism test (same input twice → identical `Solution`).

**Honest expectation:** near-linear on exhaustion (~cores×; ~20 on the current
benchmark host — 65 ms → single-digit ms, and a 5 s `solve_timeout` budget
covers ~cores× more combos, directly cutting spurious Timeout results on hard
images). ~Zero on easy images (1–2 combos can't parallelize). Sequence it
*after* proposal 1 so the speedup isn't measured against an unoptimized serial
baseline.

---

## Proposal 3 — True zero-copy shmem in `ps-grpc` via a borrowed-storage image API in `ps-detect`

**The gap FU-A deliberately left:** FU-A removed the inline-path
`image_data.clone()`, but the shared-memory path still copies the **entire
frame** out of the mmap on every request — `mmap.to_vec()` at
`ps-grpc/src/service.rs:144` (ExtractCentroids) and `:328` (SolveFromImage),
each with a code comment saying exactly why: `GrayImage::from_raw` needs an
owned `Vec`, and changing `ps-detect`'s API was out of scope then. That copy
(~0.79 MB at 1024×768, more at 5 MP) defeats the entire purpose of the shmem
path — the path a high-rate camera client (cedar-server style) would use, where
per-frame memcpy + allocator pressure is pure per-request overhead in the
measured ~3.7–4.2 ms/request overhead band.

**Concrete steps:**

1. **Genericize `ps-detect`'s entry points over pixel storage.**
   `get_stars_from_image` already takes `&GrayImage` — the ownership
   requirement is an API artifact, not algorithmic. `image::ImageBuffer` is
   already generic over its container: accept
   `ImageBuffer<Luma<u8>, C> where C: Deref<Target = [u8]>` (or an internal
   `&[u8]` + dims view) in `detect.rs`, `binning.rs`, `gate.rs`, `noise.rs`.
   Internals (binning output, blob formation) keep producing owned buffers —
   only the *input* image becomes borrowable. No algorithm change; detect
   parity fixtures must stay byte-identical.
2. **Wrap the mmap directly in `ps-grpc`.** Replace `mmap.to_vec()` with
   `ImageBuffer::from_raw(w, h, &mmap[..])`, keeping the mmap alive for the
   request duration. Delete the two "zero-copy is out of scope" comments.
3. **Measure before over-investing (per FU-A's own caveat):** confirm with the
   harness/clients whether the shmem path is exercised today; if not, land the
   API genericization (it also benefits any future in-process embedder that
   already owns pixel buffers) and benchmark with a synthetic shmem client at
   1024×768 and 5 MP.

**Gates:** detect parity fixtures byte-identical (this is an API-surface
change, not a detect-algorithm change — any centroid diff is a bug, STOP);
ExtractCentroids/SolveFromImage byte-identical results on fixture images via
both inline and shmem paths; `cargo test --workspace` green.

**Honest expectation:** ~0.3–1 ms/request at 1 MP (memcpy + page-fault +
allocator pressure), scaling with resolution — the same magnitude FU-A's
inline-path fix targeted, but on the path built for throughput. At 5 MP
(~5 MB/frame) it's several ms/frame, which at camera rates is the difference
between the copy being noise and being the bottleneck.

---

## Suggested ordering

1. **Proposal 1** first — largest measured serial win, pure `ps-db` scope,
   no concurrency risk, and it fixes the baseline FU-C should be measured
   against.
2. **Proposal 3** in parallel if desired — orthogonal surface (`ps-detect`
   API + `ps-grpc`), no overlap with proposal 1's files.
3. **Proposal 2 (FU-C)** last, after user approval and the ps-judge Job B
   semantics ruling, measured against the post-proposal-1 serial baseline.

## Non-goals (carried forward)

- Accuracy/matching behavior changes — including hale_bopp's default-params
  `SolveFromImage` NoMatch (that's feat-10/H2, specced separately) and the
  tree.jpg stress false-positive (accuracy-domain, needs its own
  investigation).
- Loosening any tolerance, `#[ignore]`-ing any test, or weakening any parity
  gate, ever.
- Python-client / harness-side overhead (not product code).
