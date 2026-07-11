# Environment & Verification Overview — plate-solver

> **Assembled document.** This is the agent-standards **core** (subtree-imported
> at [`standards/`](./standards/), pinned at **v1.0**) concatenated with the
> **plate-solver overlay**, so a reviewer reads one document start-to-finish.
> The verification gates referenced here are specified in
> [`standards/rust/GATES.md`](./standards/rust/GATES.md) and
> [`standards/universal/GATES.md`](./standards/universal/GATES.md). To update:
> bump the subtree pin, then re-assemble; do not hand-edit `standards/`.

## What this system is

Code in this repository is produced by an **agent fleet** ("Gas Town"), not by
a single person typing. The point of this document is to make that process fully
legible: what writes the code, what reviews it, where a human is required, and
what must be green before anything merges — so an outside architect can audit the
*process*, not just the diff.

The design principle throughout: **the author never grades its own homework.**
Every change is checked by mechanized gates it cannot influence and by an
independent reviewer of a different model lineage than the author.

## Topology

```
town root  (shared filesystem + git ancestor of everything)
 ├── mayor/         dispatch: turns approved work into sized jobs
 ├── <rig>/         one product repo (its own git remote), containing:
 │    ├── polecats/   ephemeral worker worktrees (the authors)
 │    ├── refinery/   integration worktree (opens/merges PRs)
 │    ├── witness/    monitoring
 │    └── standards/  ← agent-standards, subtree-imported (read-only in the rig)
 └── daemon         heartbeat scheduler + supervisors (deacon/boot)
```

A **rig** is a single product repo with its own remote. A **polecat** is a
short-lived worker with its own git worktree (its real working repo is the nested
`sandbox/` checkout). The **refinery** integrates finished work.

## The inner loop (one unit of work)

```
bead (a spec'd task, human-approved)
  → Mayor sizes it and slings it to a polecat
    → polecat reads: the bead + the openspec spec + the inherited standards/
      → implements against the spec
        → runs the fast gates locally
          → commits, opens a PR
            → gt done
```

A polecat only ever works one bead at a time, against a written spec, with the
standards materialized in-tree so it cannot "forget" them.

## The outer loop (the town)

```
Mayor evaluates the ready queue
  → size-aware dispatch (small=local model, large=cloud relay), with concurrency caps
    → daemon heartbeat spawns workers
      → refinery integrates merged work
        → witness/deacon/boot supervise liveness
          → Mayor re-evaluates the queue
```

The outer loop is what keeps many inner loops running without a human babysitting
each one. Caps and supervision are backstops, not quality gates — quality is the
gate ladder below.

## The verification loop (the gate ladder)

This is where "hole-resistant" is enforced. A PR merges only after:

```
PR opened by the author polecat
  │
  ▼  ① FAST Tier-2 CI gates (.github/workflows/ci.yml) — must be GREEN
  │     fmt · clippy -D · test+coverage · doc · msrv · machete ·
  │     semver-checks · cargo-deny(4) · secret-scan · differential/parity
  │
  ▼  ② INDEPENDENT REVIEW — authoritative
  │     an Opus (cloud-Claude) adversarial review pass. Opus is a DIFFERENT
  │     model lineage than the qwen/glm authors, which removes the
  │     self-selection-bias objection: the reviewer and author are not the
  │     same reasoner.
  │
  ▼  ③ HUMAN SIGN-OFF — only when required
  │     • the change touches a CRITICAL PATH (declared in the overlay below), or
  │     • the Opus review dissents.
  │     Otherwise ② is sufficient and the merge is autonomous.
  │
  ▼  MERGE  (enforcement mechanism: branch protection — required checks +
             required review; live on this repo, see enforcement status below)
```

Nightly, the **deep** gates (`standards/rust/ci/deep.yml`) run mutation testing,
fuzzing, Miri, SBOM, and benchmarks. A mutation-score drop or a fuzz crash blocks
the next release.

## Enforcement status — this rig, as of 2026-07-11

Every claim below is true at time of writing; update this section as wiring
lands, and never let it claim more than what is enforced.

| Layer | Status |
|-------|--------|
| Branch protection, `rewrite` | **ENFORCED** — 9 required checks from `ci.yml` + required review; enforced for admins; force-push and deletion disabled. |
| Branch protection, `main` | **LOCKED** read-only — frozen at v1-original + PR #10 (two test files + one `#[doc(hidden)]` test accessor). |
| Fast Tier-2 CI (① — `ci.yml`) | **INSTALLED, self-arming** — detect + secret-scan run now; the cargo jobs skip-satisfy until a root `Cargo.toml` exists, then fire automatically. Armed, not yet firing. |
| Independent Opus review (②) | **SPECIFIED ONLY** — mechanism in [`standards/universal/OPUS-REVIEW.md`](./standards/universal/OPUS-REVIEW.md); no `review/opus` status check exists yet, and it is not in the required-checks list. Until wired, ② happens out-of-band or not at all — do not read this doc as claiming it runs. |
| Deep nightly gates (`deep.yml`) | **NOT INSTALLED.** |
| Standards subtree | `standards/` pinned at **agent-standards v1.0**. |

## Where the human is in the loop

Four points, and only four:

1. **Spec approval** — a bead is not sling-able until its openspec change is
   human-approved. Humans decide *what* is built and *what "correct" means*.
2. **Critical-path / dissent sign-off** — humans review the changes that carry the
   most risk (see the overlay below) and any change the independent reviewer flags.
3. **Escalations** — infrastructure failures (data plane down, dispatch broken)
   page a human; the fleet does not "push through" a broken foundation.
4. **Release + standards blessing** — humans cut releases and bless which pinned
   version of these standards a rig adopts.

Everything else — routine implementation and its merge — runs autonomously behind
the gate ladder.

## The provenance trail (why every line is auditable)

Every merged line traces backward through an unbroken chain:

```
approved openspec change  →  bead  →  polecat branch  →  PR
   →  green Tier-2 CI run  →  independent Opus review  →  (human sign-off if required)
      →  merge commit
```

Nothing reaches a protected branch that did not (a) implement an approved spec,
(b) pass every mechanized gate, and (c) get reviewed by an independent reasoner.
That chain, not trust in the author, is what makes the output defensible.

---

# Rig overlay

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
