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

## Build & Run

### Prerequisites

```bash
# Rust toolchain (stable, ≥1.83)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# protoc (required for ps-grpc gRPC service)
brew install protobuf        # macOS
# apt-get install -y protobuf-compiler  # Ubuntu/Debian
```

### Build and test

```bash
cargo build --workspace          # build all crates
cargo test --workspace           # run all 182 tests (≈60 s)
cargo fmt --check                # verify formatting
cargo clippy --workspace         # lint (24 style warnings, 0 errors)
```

### Run the gRPC plate-solver server

```bash
# Build a star-pattern database from HIP/TYC catalogs
cargo run -p ps-dbgen -- \
  --hip path/to/hip_main.dat \
  --tyc path/to/tyc2.dat \
  --output ps_database.bin

# Start the gRPC server (default: 127.0.0.1:50051)
cargo run -p ps-grpc -- --database ps_database.bin --address 127.0.0.1:50051
```

The server implements the `PlateSolver` gRPC service (`ps-grpc/proto/plate_solver.proto`) and
is wire-compatible with the `cedar-detect` protocol for `ExtractCentroids`.

For full build details, parity outcomes per feature, and fixture-recapture instructions, see
[`openspec/IMPLEMENTATION-STATUS.md`](./openspec/IMPLEMENTATION-STATUS.md).
