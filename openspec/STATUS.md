# Documentation Status — Plate Solver (Rust)

_Generated 2026-06-09 for the OpenSpec documentation milestone (M0). This is a review index for
the spec set; it is not itself an OpenSpec artifact._

## What this is

A complete, validated OpenSpec documentation set specifying a from-scratch **Rust**
reimplementation of the tetra3/cedar "lost-in-space" plate solver — star detection → pattern-
database lookup → attitude (RA/Dec/Roll/FOV/distortion) recovery — delivered over **gRPC** and
embeddable on **mobile**. No implementation code is written yet; this is the contract to build
against. The product "why/what" is in [`PRD.md`](./PRD.md); shared context, conventions, and the
glossary are in [`project.md`](./project.md).

## Definition of done — met

- `openspec/` contains `project.md`, `PRD.md`, `STATUS.md`, and **7 active changes**.
- **Every** change passes `openspec validate <change> --strict` (exit 0) and shows **4/4
  artifacts** (proposal, specs, design, tasks) under `openspec status`.
- Totals: **77 requirements / 122 scenarios** across the 7 capabilities.

## Feature map (dependency / implementation order)

| # | Change | Capability | Crate | Reference doc | Reqs/Scenarios |
|---|---|---|---|---|---|
| 1 | `feat-01-foundation-math-core` | `math-core` | `ps-core` | doc 02 (+01) | 19 / 31 |
| 2 | `feat-02-star-detection` | `star-detection` | `ps-detect` | doc 04 | 11 / 18 |
| 3 | `feat-03-pattern-database` | `pattern-database` | `ps-db` | doc 05 §6–7, doc 02 §6 | 10 / 14 |
| 4 | `feat-04-database-generation` | `database-generation` | `ps-dbgen` | doc 05 | 10 / 15 |
| 5 | `feat-05-plate-solver` | `plate-solver` | `ps-solve` | doc 06 (+02 §8–10) | 11 / 19 |
| 6 | `feat-06-grpc-service` | `grpc-service` | `ps-grpc` | doc 07 | 8 / 13 |
| 7 | `feat-07-mobile-runtime` | `mobile-runtime` | `ps-mobile` | docs 04/06/08 perf | 8 / 12 |

Build order: `math-core` → `star-detection` → `pattern-database` → `database-generation` →
`plate-solver` → `grpc-service` → `mobile-runtime`. Each change carries its own `proposal.md`,
`specs/<capability>/spec.md`, `design.md`, and `tasks.md`.

## Scope decisions (from review)

- **cedar throughout** — detection (cedar-detect), DB generation (lattice fields), and the solver
  all follow the cedar variant, which strictly supersedes the original tetra3.
- **Reference-only / non-goals:** tetra3's simpler detector (doc 03) and per-anchor DB
  enumeration (doc 05 §5.1); partial-sky databases; tracking mode; fisheye lens models;
  >8-bit detection.
- **gRPC = full `PlateSolver`** service (`ExtractCentroids`, `SolveFromCentroids`,
  `SolveFromImage`, `GetInfo`), reusing cedar-detect's `Image`/`ImageCoord` message shapes.
- **Parity is a tested contract** — scenarios assert numerical parity vs the Python reference
  within stated tolerances (RA/Dec arcsec, centroids ±0.1 px, identical matched catalog IDs).

## How to review

```sh
openspec list                                  # the 7 changes
openspec show feat-05-plate-solver             # a change (proposal + specs + design + tasks)
openspec validate feat-05-plate-solver --strict
openspec status --change feat-05-plate-solver  # 4/4 artifacts
openspec view                                  # interactive dashboard
```

Read order for a reviewer: [`PRD.md`](./PRD.md) → [`project.md`](./project.md) → the changes in
the dependency order above. Each `spec.md` requirement cites the reference doc it derives from.

## Next steps (post-documentation)

Implement the crates in dependency order, turning each change's `tasks.md` into code and each
`#### Scenario:` into a test, holding the parity tolerances as the correctness gate. Archive a
change with `openspec archive <change>` once its capability is implemented and its specs move to
`openspec/specs/`.
