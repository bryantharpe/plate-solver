# plate-solver — Gastown rewrite

This branch (`rewrite`) is a **from-scratch reimplementation** of plate-solver, driven
entirely by the specifications in [`openspec/`](openspec/). The original v1 implementation
has been removed here so the rewrite starts clean.

> **The original is preserved.** The complete v1 codebase lives on `main` and is frozen
> at the immutable tag [`v1-original`](../../releases/tag/v1-original) (`185128e`).
> Recover it any time with `git checkout v1-original`. This branch will be merged on top
> of the original once the rewrite is ready.

## What's in this branch (and only this)

| Path | Role |
|------|------|
| `openspec/` | **Source of truth.** Capability specs, PRD, and project conventions. Start at `openspec/project.md` and `openspec/PRD.md`. |
| `openspec/specs/` | The six canonical capability specs the rewrite must satisfy, in dependency order: `math-core`, `star-detection`, `pattern-database`, `database-generation`, `plate-solver`, `grpc-service`. (`mobile-runtime` is the seventh capability, still an open change under `openspec/changes/`.) |
| `proto/` | The gRPC interface contracts (`plate_solver.proto`, `cedar_detect.proto`) — relocated out of the old crate as standalone specs. |
| `reference-solutions/` | **The oracle.** Vendored reference outputs (cedar-solve / tetra3 / cedar-detect) that parity / differential tests validate the rewrite against. Kept deliberately so the rewrite can prove behavioral equivalence, not just compile. |

## What was intentionally removed

Everything that is *implementation* rather than *specification*: all `ps-*` crates, the
Cargo workspace and lockfile, `rust-toolchain.toml`, the `tools/` and `notes/` directories,
the v1 planning docs, and the `.claude/` tooling. These are rebuilt from scratch.

Also removed: every artifact that reported **v1's implementation status** — `openspec/STATUS.md`,
`openspec/IMPLEMENTATION-STATUS.md`, `CODEBASE-REVIEW.md`, and the two changes written against
v1's now-deleted code (`feat-09-eval-harness`, whose tasks were checked off against a `tools/parity/`
harness that no longer exists, and `feat-10-solve-from-image-detect-params`, a patch for a defect
in deleted code). They described code that does not exist on this branch, and an agent reading them
would build against an API that was never here. Their surviving *requirements* were folded into
`openspec/specs/` rather than dropped — see the `plate-solver` and `grpc-service` specs on detection
parameters and image-estimated noise.

Also removed, and this is the important one: **`openspec/changes/archive/`** — v1's seven archived
changes, holding its architecture rationale (`design.md`) and its exact dependency-ordered task
breakdown (`tasks.md`). It was *useful*, and it was removed anyway. See below.

Along with it went the places v1's architecture had leaked into the specs themselves:
`project.md`'s crate→capability table (which handed over the module split and the library picks),
and the `web-ui` spec — written retroactively to describe an already-built harness, citing its
source files thirteen times, and never part of the product path in the first place.

**No code exists on this branch.** Nothing here reports implementation progress; the rewrite's
progress is tracked in beads (`ps-*`), not in `openspec/`.

## The constraint: specs are the only input

This rewrite is deliberately run as if **no prior implementation had ever existed** — because
that is the realistic condition. On a real project you inherit a specification and a system to
match; you do not inherit your own previous attempt's design decisions and task list.

So the build must be derivable from **`openspec/specs/` + `reference-solutions/` alone.**

- `openspec/specs/` — the six capability specs. The requirements, and the `#### Scenario:` blocks
  that are the acceptance criteria.
- `reference-solutions/` — the external oracle. Legitimate: it is the system being
  re-implemented and the parity contract, the equivalent of the legacy system or paper you would
  have at work.

What the specs deliberately do **not** say is how to build it. They name capabilities, not
modules. The crate structure, the internal interfaces, and the dependency choices are for the
implementation to determine; `project.md` §5 lists the binding constraints — the ones fixed by
parity, by the proto contracts, or by the PRD — and nothing beyond them. Where the fleet lands
differently from v1 is a result, not a defect.

`changes/archive/` was neither of those. It was the record of *our own previous build's* internal
choices — the crate split, the decomposition, the order. Keeping it would have quietly changed
the question from *"are these specs sufficient to build the system?"* to *"can an agent follow
v1's build plan?"* — and the second question is already answered, because v1 followed it.

It is preserved on `main` and at tag [`v1-original`](../../releases/tag/v1-original), and it has a
real use *after* the fact: diff what gets built from the specs against what v1 did, and the
differences are the finding. But it must be invisible while the work is running, because a rule
that says "don't read the answer key" is not a rule anyone can enforce.

One v1 lesson was kept, and the way it was kept is the point: v1 hardcoded `noise_estimate = 1.0`
in `solve_from_image`, which is why `hale_bopp.jpg` failed to solve. That did not survive as
history or as a note — it survives as a **requirement and a regression scenario in the
`grpc-service` and `plate-solver` specs**. Lessons enter through the spec, or not at all.

## Getting started

1. Read `openspec/project.md` (conventions) and `openspec/PRD.md` (product requirements).
2. Work capability-by-capability from `openspec/specs/`.
3. Validate against `reference-solutions/` as each capability lands.
4. A CI pipeline (quality / security / test gates) will be added once there is buildable
   code to run it against.
# test
