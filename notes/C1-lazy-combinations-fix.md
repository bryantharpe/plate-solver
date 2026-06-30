# C1 — Lazy combinations iterator (fix plan, not yet applied)

> Status: **Proposed, not applied.** Save point — return here to implement.
> Parent: [`CODEBASE-REVIEW.md`](../CODEBASE-REVIEW.md) concern C1.
> Branch: `feat/rust-implementation` @ `8932c62` (verify file still matches before editing).

## The problem

`combinations_4(n) -> Vec<[usize; 4]>` (`ps-solve/src/lib.rs:582-594`) eagerly
materializes **every** 4-combination before the solve loop consumes them at
`ps-solve/src/lib.rs:190`.

`num_pattern_centroids` is bounded by `verification_stars_per_fov`, which the
bundled `default_database.npz` ships as **150** (confirmed from `props_packed`:
`verification_stars_per_fov = 150`, `num_patterns = 1_010_981`). Worst case:

```
C(150,4) = 150·149·148·147 / 24 = 20,258,775 combos
20,258,775 × 32 B ([usize;4] on 64-bit) = 648,280,800 B ≈ 618 MiB
```

held for the whole solve. Even at n=100 it's ~125 MiB. The reference (cedar-solve)
uses `itertools.combinations`, which is **lazy** and abandons on timeout — it
never pays this. On a phone (the stated build target) this is an OOM on the
flagship path; on the test machine it's the heavy footprint behind the `sv6`
solves.

### Why eager is unnecessary (behavioral equivalence)
The solve loop checks timeout/cancel **per combo** and `break 'outer`s out
(`ps-solve/src/lib.rs:192-207`). A lazy iterator yields the same sequence, so
results, timeout, and cancel semantics are identical. Only the allocation
disappears, and a timed-out solve additionally stops generating combos it would
never reach.

The `candidate_keys: Vec<([u32;5], i64)>` built inside the loop
(`ps-solve/src/lib.rs:228`) is per-combo and small (3^5 = 243 entries at
`band = ceil(0.002 · 250) = 1`), so it is **not** a concern — only the outer
combinations Vec is.

## Proposed fix: a lazy `Combinations4` iterator

Same lexicographic order as the old function, zero allocation, replaces the
eager `Vec` at the call site. Drop in where `combinations_4` currently lives
(`ps-solve/src/lib.rs:582-594`):

```rust
/// Lazily yields all 4-element combinations of `[0, n)` in lexicographic order.
///
/// Replaces an eager `Vec<[usize; 4]>` that allocated `C(n,4) · 32 bytes` up front
/// (≈618 MiB at `n = 150`, the bundled DB's `verification_stars_per_fov`). Yields
/// the identical sequence to the old `combinations_4`, so timeout/cancel checks in
/// the solve loop fire at the same cadence with no allocation.
struct Combinations4 {
    n: usize,
    cur: [usize; 4],
    done: bool,
}

impl Combinations4 {
    fn new(n: usize) -> Self {
        Self { n, cur: [0, 1, 2, 3], done: n < 4 }
    }
}

impl Iterator for Combinations4 {
    type Item = [usize; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let out = self.cur;
        // Advance to the next lexicographic combination. `cur[k]` may range up to
        // `n - 4 + k`; find the rightmost position that can still increase, bump it,
        // and reset the trailing positions to the minimal valid values.
        let mut k: i32 = 3;
        while k >= 0 {
            let kk = k as usize;
            if self.cur[kk] < self.n - 4 + kk {
                self.cur[kk] += 1;
                for j in (kk + 1)..4 {
                    self.cur[j] = self.cur[kk] + (j - kk);
                }
                return Some(out);
            }
            k -= 1;
        }
        self.done = true;
        Some(out) // the final combination is still yielded
    }
}

/// Lazy 4-combinations of `[0, n)`, lexicographic. Returns an iterator (no allocation).
fn combinations_4(n: usize) -> Combinations4 {
    Combinations4::new(n)
}
```

### Correctness notes
- `done = n < 4` short-circuits the empty case AND guards `n - 4 + kk` from
  underflow (when `n < 4`, `next()` returns `None` before computing it).
- Verified the increment logic by hand for n=5: produces
  `[0,1,2,3] → [0,1,2,4] → [0,1,3,4] → [0,2,3,4] → [1,2,3,4]` then `None` —
  exactly the sequence the old nested loops produced.
- Yields exactly `C(n,4)` items: the last combination is returned with `done`
  set, the next call returns `None`.

## Edits required

1. **`ps-solve/src/lib.rs:582-594`** — replace the `combinations_4` fn body with
   the iterator above (struct + impls + thin constructor fn keeping the same
   name `combinations_4`).
2. **`ps-solve/src/lib.rs:190`** — no change needed; `for combo in
   combinations_4(num_pattern_centroids)` already drives any `Iterator`, and
   `'outer` / `break 'outer` work identically.
3. **Tests `combinations_4_basic` and `combinations_4_five_elements`**
   (`ps-solve/src/lib.rs:836`, `:845`) — they compare against `Vec`:

   ```rust
   // before
   assert_eq!(combinations_4(0), Vec::<[usize; 4]>::new());
   let combos = combinations_4(5);
   assert_eq!(combos.len(), 5);
   assert_eq!(combos[0], [0, 1, 2, 3]);
   ```
   ```rust
   // after
   assert_eq!(combinations_4(0).collect::<Vec<_>>(), Vec::<[usize; 4]>::new());
   let combos: Vec<[usize; 4]> = combinations_4(5).collect();
   assert_eq!(combos.len(), 5);
   assert_eq!(combos[0], [0, 1, 2, 3]);
   ```

   Collecting in the test also guards against someone re-introducing an
   eager `Vec`-returning type.

## Alternative considered (rejected)
Inlining 4 nested `for` loops at the call site (also lazy, zero allocation, no
new type). Rejected: it adds 4 indentation levels to the ~360-line loop body
(`:190-548`) and smears combination logic into the solver. The iterator keeps
the body flat and stays unit-testable.

## Verification gate (run after applying)
```bash
cargo test -p ps-solve          # 17 pass + 1 ignored (sv6 parity must still match)
cargo clippy -p ps-solve        # 0 errors
cargo test --workspace          # full 182 pass / 0 fail / 1 ignored
```
The sv6 parity tests (`sv6_solve_from_image_parity`, `sv6_solve_from_centroids_parity`)
are the behavioral guard: RA/Dec within 10 arcsec and 19/19 catalog IDs must still
hold — they prove laziness didn't change the search order or results.

## Net effect
- Peak transient allocation in `solve_from_centroids` drops from ~618 MiB
  (n=150) to a few stack words, with no change to solve results, status codes,
  timeout, or cancel behavior.
- A timed-out solve stops paying for combinations it'll never reach.
- Two small test edits; nothing else in the crate changes.

## Follow-ups this fix does NOT cover (separate concerns, see CODEBASE-REVIEW.md)
- C6: per-combo `Instant::now()` timeout check runs up to 20M times — same as
  before, not a regression. Could batch (check every K combos) as a later opt.
- C6: RPC deadline still not wired to `cancel_flag`.
- The candidate_keys inner Vec is small and per-combo; left as-is.