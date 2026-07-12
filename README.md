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
| `openspec/` | **Source of truth.** Capability specs, PRD, project conventions, and the full change history. Start at `openspec/project.md` and `openspec/PRD.md`. |
| `openspec/specs/` | The seven canonical capability specs the rewrite must satisfy: `math-core`, `star-detection`, `pattern-database`, `database-generation`, `plate-solver`, `grpc-service`, `web-ui`. |
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

**No code exists on this branch.** Nothing here reports implementation progress; the rewrite's
progress is tracked in beads (`ps-*`), not in `openspec/`.

## Getting started

1. Read `openspec/project.md` (conventions) and `openspec/PRD.md` (product requirements).
2. Work capability-by-capability from `openspec/specs/`.
3. Validate against `reference-solutions/` as each capability lands.
4. A CI pipeline (quality / security / test gates) will be added once there is buildable
   code to run it against.
