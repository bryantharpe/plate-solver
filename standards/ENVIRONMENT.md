# Environment & Verification Overview (reusable core)

> This is the **generic** overview shared by every rig. Each rig subtree-imports
> it and appends a short overlay (`ENVIRONMENT.overlay.template.md`) naming its
> own critical paths, oracle, and budgets; the two render as one document a
> reviewer reads start-to-finish. The verification gates referenced here are
> specified in [`rust/GATES.md`](./rust/GATES.md).

## What this system is

Code in these repositories is produced by an **agent fleet** ("Gas Town"), not by
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
 │    └── standards/  ← this repo, subtree-imported (read-only in the rig)
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

> **Status (per rig, as of 2026-07-11).** Every gate below has been proven to
> fire (see `rust/GATES.md`). Enforcement is now live where stated:
>
> - **plate-solver** — branch protection **ENFORCED**: `rewrite` requires the 9
>   `ci.yml` checks + required review, admins included, no force-push/deletion;
>   `main` is locked read-only. The fast CI (`ci.yml`) is installed and
>   self-arming (cargo jobs skip until a root `Cargo.toml` exists). The
>   independent Opus review (②) is **specified, not yet wired** — see
>   `universal/OPUS-REVIEW.md`; no `review/opus` check exists yet. The deep
>   nightly gates (`deep.yml`) are **not yet installed**.
> - **All other rigs** — TARGET: nothing enforced yet; this ladder is what a rig
>   adopts at onboarding.
>
> Update this notice per-rig as enforcement lands.

This is where "hole-resistant" is enforced. A PR merges only after:

```
PR opened by the author polecat
  │
  ▼  ① FAST Tier-2 CI gates (rust/ci/ci.yml) — must be GREEN
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
  │     • the change touches a CRITICAL PATH (declared in the rig overlay), or
  │     • the Opus review dissents.
  │     Otherwise ② is sufficient and the merge is autonomous.
  │
  ▼  MERGE  (enforcement mechanism: branch protection — required checks +
             required review. ENFORCED on plate-solver as of 2026-07-11;
             TARGET elsewhere)
```

Nightly, the **deep** gates (`rust/ci/deep.yml`) run mutation testing, fuzzing,
Miri, SBOM, and benchmarks. A mutation-score drop or a fuzz crash blocks the next
release.

## Where the human is in the loop

Four points, and only four:

1. **Spec approval** — a bead is not sling-able until its openspec change is
   human-approved. Humans decide *what* is built and *what "correct" means*.
2. **Critical-path / dissent sign-off** — humans review the changes that carry the
   most risk (see the rig overlay) and any change the independent reviewer flags.
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

Nothing reaches `main` that did not (a) implement an approved spec, (b) pass every
mechanized gate, and (c) get reviewed by an independent reasoner. That chain, not
trust in the author, is what makes the output defensible.
