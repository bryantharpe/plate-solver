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

## Enforcement status — this rig, as of 2026-07-12

Every claim below is true at time of writing; update this section as wiring
lands, and never let it claim more than what is enforced.

| Layer | Status |
|-------|--------|
| Branch protection, `main` | **ENFORCED** — **10 required checks** (`ci.yml`'s 9 + `review/judge`); enforced for admins; force-push and deletion disabled. Auto-merge (squash) enabled so an approved, green PR merges itself. A repository ruleset additionally blocks deletion of `refs/heads/polecat/**` work branches for every actor (added 2026-07-12 after hq-91n, where a failed merge deleted the only remote copy of the work). Blanket required review removed 2026-07-12 by ratified decision (checks-only merges). `main` became the one working branch on 2026-07-12: the orphan `rewrite` branch converged into it (merge `f93eaf8`, tree identical to the rewrite tip) and was deleted; v1 remains reachable as the merge's second parent and under tag **`v1`**. |
| Independent review (② — `review/judge`) | **ENFORCED and AUTONOMOUS** — a PR whose head SHA lacks a passing `review/judge` status cannot merge; absence fails closed. The reviewer (`judge` role, glm-5.2 — a different lineage than every author seat) runs town-side; only the verdict is published. A town-side sweep (cron, every 10 min) reviews every open PR that lacks a verdict, arms auto-merge on APPROVE, and escalates a DISSENT to a human once per PR; a dissent is answered by pushing a fix, which re-triggers review on the new head SHA. Human override is a recorded status with login + reason (U4), never a bypass. Proven on canary PR #16 and live on PR #18 (first fleet code: 9 CI gates + judge APPROVE → merged). The originally-planned in-formula refinery dissent loop is not possible on the current gt build — patrols render from the binary's embedded formulas (gastown#4472) — which is why review runs as a sweep outside the refinery. |
| Fast Tier-2 CI (① — `ci.yml`) | **ENFORCED, firing** — all 9 checks run on every PR since the workspace `Cargo.toml` landed (PR #18); first full green run on PR #18/#19 (including a real `cargo-machete` catch, resolved as an oracle codegen false-positive with an explicit ignore). |
| End-to-end eval (`e2e-eval`, `e2e-canary`) | **PARTLY ENFORCED** — the E1–E3 tests are ordinary integration tests, so their assertions already block merges through `test-coverage`. What is not yet enforced is everything only these two jobs check: the oracle-integrity step, the owned-test-target and tree-untouched checks, the scorecard judgement, and the canary. They become ENFORCED when both are added to `main`'s required checks (pinned to GitHub Actions) and code-owner review is required. Armed unless the Actions variable `E2E_ARMED` is `false-until-YYYY-MM-DD` and that date has not passed (for a from-specs rebuild that cannot solve yet; the score is still published and the attestation says DISARMED; `test-coverage` is not disarmed by it). Added 2026-10-17 after the end-to-end audit (`docs/audit-e2e-parity.md`): `main` had passed every gate above, with a judge APPROVE on each PR, and solved 0 of the 21 reference images. |
| Merge-record integrity | **ENFORCED by audit** — a cron guard (every 15 min) re-derives every recent "merged" bead claim from `origin/main` and PR state: a record with no backing commit or PR is reopened, labeled, and escalated; a vanished work branch, a disabled deletion ruleset, or disabled auto-merge also alarm. Exists because the work-tracking layer can record merges it never verified (hq-91n, upstream gastown#4472) — the git remote, not the ledger, is the source of truth. |
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
