# plate-solver
Rust implementation of the full tetra3 lost in space algorithm for star field identification.

A from-scratch **Rust** reimplementation of the tetra3/cedar "lost-in-space" plate-solving
pipeline — star detection → pattern-database lookup → attitude (RA/Dec/Roll/FOV/distortion)
recovery — delivered over **gRPC** and embeddable on **mobile** (iOS/Android).

## Documentation

The design is specified as an [OpenSpec](https://github.com/Fission-AI/OpenSpec) documentation
set under [`openspec/`](./openspec/), validated with `openspec validate --strict`:

- **[`openspec/PRD.md`](./openspec/PRD.md)** — product requirements (problem, users, goals,
  non-functional budgets, success metrics, milestones).
- **[`openspec/project.md`](./openspec/project.md)** — shared context, conventions, glossary,
  Rust workspace/dependency decisions, and the reference-documentation map.
- **[`openspec/STATUS.md`](./openspec/STATUS.md)** — review index and feature map.
- **[`openspec/changes/`](./openspec/changes/)** — one change per feature (in dependency order):
  `feat-01-foundation-math-core`, `feat-02-star-detection`, `feat-03-pattern-database`,
  `feat-04-database-generation`, `feat-05-plate-solver`, `feat-06-grpc-service`,
  `feat-07-mobile-runtime`. Each carries a proposal, specs (requirements + scenarios), a design,
  and a task list.

The reference implementations being re-implemented (Python tetra3, Python cedar-solve, Rust
cedar-detect) and their rebuild-level docs live under
[`reference-solutions/`](./reference-solutions/) (read-only source of truth).

To browse: `openspec list`, `openspec show <change>`, or `openspec view`.
