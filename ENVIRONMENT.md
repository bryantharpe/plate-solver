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
  │     semver-checks · cargo-deny(4) · secret-scan ·
  │     e2e-eval (differential parity vs cedar: E1 centroids · E2 pixels · E3 gRPC) ·
  │     e2e-canary (proof the e2e gate can fail)
  │
  ▼  ② INDEPENDENT REVIEW — authoritative
  │     an adversarial review by the `judge` role (glm-5.2, Zhipu), a DIFFERENT
  │     model lineage than every author seat (coder-small and coder-large, both
  │     kimi/Moonshot), which removes the self-selection-bias objection: the
  │     reviewer and author are not the same reasoner. The verdict is published
  │     as the required status check `review/judge`; absence fails closed.
  │
  ▼  ③ HUMAN SIGN-OFF — only when required
  │     • the change touches a CRITICAL PATH (declared in the overlay below),
  │     • the judge dissents until the retry bound is exhausted, or
  │     • a human overrides a dissent (recorded, with reason — never a bypass).
  │     Otherwise ② is sufficient and the merge is autonomous.
  │
  ▼  MERGE  (enforcement mechanism: branch protection — required checks;
             human review only via ③, not as a blanket rule; live on this
             repo, see enforcement status below)
```

Nightly, the **deep** gates (`standards/rust/ci/deep.yml`) run mutation testing,
fuzzing, Miri, SBOM, and benchmarks. A mutation-score drop or a fuzz crash blocks
the next release.

## Enforcement status — this rig, as of 2026-10-18

Every claim below is true at time of writing; update this section as wiring
lands, and never let it claim more than what is enforced.

The Gas Town fleet described above (polecats, refinery, the `judge` reviewer and
the town-side guards) is no longer in use. Code now reaches `main` through pull
requests opened under the maintainer's own GitHub account, by the maintainer or by
an AI coding agent working as the maintainer. The rows below describe what GitHub
itself enforces; nothing town-side runs.

| Layer | Status |
|-------|--------|
| Branch protection, `main` | **ENFORCED** — **13 required checks**: every `ci.yml` job, each pinned to the GitHub Actions app so no other app can satisfy it. Enforced for admins, so no account can merge a PR with a red or missing check, including the maintainer's account, which agents also push with. Force-push and deletion are disabled. No review is required: a single account authors and merges, so a review requirement could not tell a human from an agent. Re-applied 2026-10-18; until then `main` had no protection at all, although this table claimed 10 required checks. `main` is the one working branch; v1 remains reachable under tag **`v1`**. |
| Independent review (② — `review/judge`) | **RETIRED** with the fleet. No reviewer runs, and `review/judge` is not a required check. PRs #87–#90 (2026-10-17) received no verdict. |
| Fast Tier-2 CI (① — `ci.yml`) | **ENFORCED** — all 13 checks run on every PR and are required: `detect`, `rustfmt`, `clippy`, `test-coverage`, `docs`, `msrv`, `cargo-machete`, `cargo-deny`, `secret-scan`, `provenance`, `hold`, `e2e-eval` and `e2e-canary`. `hold` is a manual brake: it is red while a PR carries the `hold:human` label. |
| Authoring integrity (`integrity` check) | **RETIRED** 2026-10-18. The job failed closed until a town-side guard labeled the PR, and nothing applies those labels any more, so it was red on every PR. Removed from `ci.yml`. |
| End-to-end eval (`e2e-eval`, `e2e-canary`) | **ENFORCED** — both are required checks on `main`, pinned to GitHub Actions. The oracle-integrity step, the owned-test-target and tree-untouched checks, the scorecard judgement and the canary all block merges, alongside the E1–E3 assertions that also run inside `test-coverage`. Changes to the oracle and the gate (the paths in `.github/CODEOWNERS`) have no separate enforcement: with no second account to require a review from, they rely on the maintainer reading the diff. Armed unless the Actions variable `E2E_ARMED` is `false-until-YYYY-MM-DD` and that date has not passed (for a from-specs rebuild that cannot solve yet; the score is still published and the attestation says DISARMED; `test-coverage` is not disarmed by it). Added 2026-10-17 after the end-to-end audit (`docs/audit-e2e-parity.md`): `main` had passed every gate above, with a judge APPROVE on each PR, and solved 0 of the 21 reference images. |
| Merge-record integrity | **RETIRED** with the fleet: the cron guard audited bead records, which no longer exist. The repository ruleset protecting `refs/heads/polecat/**` work branches is still active, but nothing creates those branches. |
| Deep nightly gates (`deep.yml`) | **NOT INSTALLED.** |
| Standards subtree | `standards/` pinned at **agent-standards v1.0** (v1.1 — the judge-role U3 — is released upstream; adoption is a deliberate subtree bump, not yet taken, so `standards/universal/OPUS-REVIEW.md` here still shows the v1.0 Opus mechanism). |

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
   →  green Tier-2 CI run  →  independent `judge` review  →  (human sign-off if required)
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

Enforced by the `e2e-eval` job, in three tiers, on the 21 sample images in
`reference-solutions/cedar-solve/examples/data/medium_fov/`, against results captured
from cedar-solve (`crates/plate-solver/tests/data/cedar_parity.json`):

- **E1 centroid parity** — the contract as written: same status, identical matched
  catalog IDs, RA/Dec, roll and FOV within 5″ — once without an FOV estimate and once
  with a user-style one (nearest degree, ±1°), each against its own cedar answers.
  `scripts/ci/cedar-goldens.py --check` re-derives every golden from the vendored
  cedar and pins the inputs they were derived from.
- **E2 image entry point** — JPEG → `solve_from_image`: identical to the composed
  detect → solve path, and within 10″ of cedar (roll 60″, FOV 15″, ≥90% of cedar's
  stars matched). Looser than E1 because this path solves with every detected star,
  while the goldens used the brightest 30; identical IDs are E1's job.
- **E3 gRPC service** — the same through `SolveFromImage`, `ExtractCentroids` and
  `SolveFromCentroids` with the real database: wire = library (status, attitude, FOV,
  distortion, stars), image = composed path, within E2's tolerances of cedar, and
  `hale_bopp` at sigma 8 is `MATCH_FOUND`.

One caveat on independence: the centroid lists the goldens start from were produced
by this repo's own detector (binning 2, `normalize_rows`, hot pixels). The goldens are
cedar's answers to those centroids; detection at those settings is not yet held to
cedar-detect (star-detection's parity test covers binning 1).

`e2e-canary` proves the gate can fail and that each check bites: ten probes, each
nudging one golden just past one tolerance or dropping one star ID, and five bugs
re-injected from `main` must each turn the tier that owns them red.

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
- the oracle and its gate — `reference-solutions/**`, the goldens under
  `crates/*/tests/data/`, the e2e tests (`reference_parity.rs`, `image_parity.rs`,
  `e2e_service.rs`), `scripts/ci/**`, `.github/**`, and whatever decides what runs
  under a test's name: every `Cargo.toml`, `build.rs`, `.cargo/**` and the toolchain
  pin. A change here can make a broken product look green without touching product
  code (`.github/CODEOWNERS`).

### Gate deltas from the org baseline

- Parity scenarios are **blocking on every PR** (not just nightly) for `ps-core`
  and `ps-solve` — parity is the product's correctness definition here. Implemented
  as `e2e-eval` + `e2e-canary` (2026-10-17).
- Coverage floor raised to **90%** for `ps-core`.

### Performance / resource budgets (asserted by the perf gate)

- Detection: **< 10 ms per 1 M pixels** on RPi-4B-class hardware.
- Solve: ~**10 ms** desktop-class excluding extraction; per-platform mobile
  budgets fixed in `mobile-runtime`.
- Database **memory-mappable** with bounded peak RAM on device.
