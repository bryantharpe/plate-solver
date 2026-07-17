# Rig overlay — template

> A rig's overlay is short. It names only what is *specific* to this rig; the
> generic loop and gate ladder come from the inherited [`ENVIRONMENT.md`](./ENVIRONMENT.md).
> On subtree import, the assembly step concatenates the core + this overlay into
> a single `ENVIRONMENT.md` at the rig root that reads start-to-finish.

## Rig: `<name>`

**One-line purpose:** <what this rig builds>

### Independent oracle (the differential-test reference)

<What external, independently-produced reference the parity gate (#8) validates
against, and the tolerance that counts as "matching".>

### Critical paths (require human sign-off on change)

List the files/areas where a merge needs a human, not just green CI + Opus review:

- `<path>` — <why it's critical>

### Gate deltas from the org baseline

Only list where this rig *tightens* or *names an override*. Tightening needs no
justification; loosening requires a named, reviewed override here.

- <e.g. coverage threshold raised to 90%>

### Performance / resource budgets

<Any latency/memory/binary-size budgets the perf gate (#9) asserts.>

---

# Example — filled overlay for the plate-solver rig

## Rig: `plate-solver`

**One-line purpose:** a from-scratch Rust reimplementation of the tetra3/cedar
"lost-in-space" star-field plate solver, delivered over gRPC and embeddable on
mobile via UniFFI.

### Independent oracle

The vendored `reference-solutions/` (tetra3 / cedar-solve / cedar-detect). The
parity gate asserts the Rust pipeline matches the Python reference within the
contract tolerances: **RA/Dec within a few arcseconds, centroids within ±0.1 px,
identical matched catalog IDs.** These references were not produced by the code
under test, so agreement is external validation.

### Critical paths (human sign-off on change)

- `ps-core/**` — the numerical-parity correctness contract (angle convention,
  hashing, Wahba/SVD attitude). A silent change here can pass tests yet break
  parity.
- any `unsafe` block, anywhere — audited-site census must stay explicit.
- `proto/**` and the UniFFI/FFI surface — public API + the `(x,y)↔(y,x)`
  boundary; breaking consumers or the coordinate convention is high-blast-radius.
- `ps-db` / `ps-dbgen` DB format & serialization — on-disk/mmap format is a
  compatibility contract.
- any change to the dependency graph (`Cargo.toml` / `Cargo.lock`).

### Gate deltas from the org baseline

- Parity scenarios are **blocking on every PR** (not just nightly) for `ps-core`
  and `ps-solve` — parity is the product's correctness definition here.
- Coverage floor raised to **90%** for `ps-core`.

### Performance / resource budgets (asserted by the perf gate)

- Detection: **< 10 ms per 1 M pixels** on RPi-4B-class hardware.
- Solve: ~**10 ms** desktop-class excluding extraction; per-platform mobile
  budgets fixed in `mobile-runtime`.
- Database **memory-mappable** with bounded peak RAM on device.
