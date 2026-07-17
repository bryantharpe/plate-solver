# agent-standards

Reusable, factory-agnostic **verification standards** for agent-produced code —
the org layer above any individual rig. It answers one question for an outside
reviewer: *how do you know code your agents wrote is at least as good as code a
human would write?* — by pinning every quality/security/test/parity gate to a
tool and a threshold, and proving each one fires against a permanent canary
before it is ever pointed at product code.

## Layout

```
ENVIRONMENT.md                    generic agent loop + gate ladder (reusable)
ENVIRONMENT.overlay.template.md   per-rig overlay (critical paths, oracle, budgets)
                                  + a worked plate-solver example
universal/GATES.md                language-agnostic gates (secrets, review, SBOM, provenance)
rust/
  GATES.md                        the Tier-2 regime + traceability matrix (measured on the canary)
  ci/ci.yml                       reusable FAST workflow (blocks every PR)
  ci/deep.yml                     reusable DEEP workflow (nightly: mutation, fuzz, Miri, SBOM, bench)
  rustfmt.toml  clippy.toml  deny.toml   pinned tool config
  workspace-lints.md              the [workspace.lints] snippet consumers paste into Cargo.toml
  canary/                         permanent proof crate — exercises every gate
RATIFY.md                         human-ratified governance + release log
```

## How a rig consumes this

```sh
# One-time import (real files, present after a plain clone — not a submodule):
git subtree add --prefix standards https://github.com/bryantharpe/agent-standards v1.0 --squash
# Later, to adopt a new blessed version:
git subtree pull --prefix standards https://github.com/bryantharpe/agent-standards v1.1 --squash
```

Then the rig's CI references `standards/rust/ci/ci.yml`, its `Cargo.toml` pulls in
`standards/rust/workspace-lints.md`'s lints, and `deny.toml`/`clippy.toml`/
`rustfmt.toml` are taken from `standards/rust/`. See [`RATIFY.md`](./RATIFY.md)
for versioning and adoption rules.

## Proof it works

Every gate in [`rust/GATES.md`](./rust/GATES.md) has been run green against
[`rust/canary/`](./rust/canary/) with real numbers (100% coverage, 39/39 mutants
caught, 1e6 fuzz runs clean, parity within 1e-9, Miri clean, 0 advisories), and
proven non-vacuous: injecting a bug turns the suite red, and weakening the tests
makes mutation testing surface survivors.
