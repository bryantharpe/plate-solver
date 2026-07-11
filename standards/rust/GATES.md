# Rust verification gates (Tier 2 — "hole-resistant")

This is the mechanized floor every Rust rig inherits. Each row is a
**best-practice claim** an outside reviewer might challenge, the **gate** that
enforces it, its **threshold**, **when** it runs, and the **result measured on
the canary** (`rust/canary/`) — the permanent proof crate that exercises every
gate. If a gate ever stops working, the canary goes red before any real rig does.

The split is deliberate: **fast** gates block every PR (seconds–minutes); **deep**
gates run nightly and gate releases (mutation, fuzz, Miri, SBOM, bench).

> **Read the canary numbers correctly.** They are **gate-proofs** — evidence each
> gate fires and measures — on ~150 lines of purpose-built code. They are NOT
> product-quality metrics and do not predict threshold achievability on real
> codebases. A rig quotes its own measured numbers, never the canary's.

## Traceability matrix

| # | Claim a reviewer would challenge | Gate / tool | Threshold | When | Canary result |
|---|----------------------------------|-------------|-----------|------|---------------|
| 1 | "Style is consistent" | `cargo fmt --check` (`rustfmt.toml`) | zero diffs | fast | ✅ clean |
| 2 | "No lint debt / footguns" | `cargo clippy --all-targets --all-features` | `-D warnings` | fast | ✅ clean |
| 3 | "It's actually tested" | `cargo test` (unit + integration + doctests) | all pass | fast | ✅ 7 unit + 3 property + 4 unit + 1 parity + 3 doctest |
| 4 | "Tests reach the code" | `cargo llvm-cov` | `--fail-under-lines 80` | fast | ✅ **100.0%** lines / regions / functions |
| 5 | "Tests actually *catch* bugs" | `cargo mutants` | 0 surviving viable mutants | deep | ✅ **39/39 caught, 0 missed** (1 unviable) |
| 6 | "Invariants hold, not just examples" | `proptest` | properties hold | fast | ✅ 3 properties (range, symmetry, unit-length) |
| 7 | "Parsers don't panic/UB on bad input" | `cargo fuzz` | no crash in smoke run | deep | ✅ **1,000,000 runs, 0 crashes** |
| 8 | "Matches an independent reference" | differential test vs golden fixture | within tolerance | fast | ✅ 6 cases within 1e-9 rad of a Python-math oracle (runtime-independent; see fixture note) |
| 9 | "It's fast / no perf regressions" | `criterion` (+ regression tracker) | tracked vs base | deep | ✅ `angular_separation` ≈ **11.25 ns** |
| 10 | "No known-vulnerable deps" | `cargo deny check advisories` | none (yanked = deny) | fast | ✅ ok |
| 11 | "No banned/duplicate/wildcard deps" | `cargo deny check bans` | wildcards denied | fast | ✅ ok |
| 12 | "Deps come from trusted sources" | `cargo deny check sources` | crates.io / path only | fast | ✅ ok |
| 13 | "License-clean" | `cargo deny check licenses` | permissive allow-list | fast | ✅ ok (allow-list curated) |
| 14 | "Advisory DB is fresh" | `cargo audit` (scheduled) | 0 vulnerabilities | deep | ✅ 0 vulns / 94 deps / 1159 advisories |
| 15 | "No dead dependencies" | `cargo machete` | none unused | fast | ✅ none |
| 16 | "Public API is stable / semver-correct" | `cargo semver-checks` | no undeclared break | fast | ✅ 196 checks pass |
| 17 | "Docs build, links resolve, API documented" | `cargo doc` + `missing_docs=deny` | `-D warnings` | fast | ✅ clean |
| 18 | "`unsafe` is audited and sound" | compiler census + `miri` | census = declared; Miri clean | fast+deep | ✅ **1 audited site**; Miri clean |
| 19 | "No committed secrets" | `ripsecrets` / gitleaks | none | fast | ✅ clean |
| 20 | "Builds on the oldest supported Rust" | MSRV build @ `1.96.0` | builds | fast | ✅ clean |
| 21 | "We can enumerate what we ship" | `cargo cyclonedx` (SBOM) | artifact produced | deep | ✅ CycloneDX per crate |

## Why the non-obvious gates matter

- **Mutation testing (#5)** is the answer to *"100% coverage doesn't mean the
  tests assert anything."* cargo-mutants perturbs the code and checks the suite
  goes red. A surviving mutant is a line that's executed but not *checked*.
- **The differential test (#8)** is the strongest single card **when the oracle is
  authorship-independent** (in the plate-solver rig: tetra3/cedar, written by other
  people) — then agreement is external validation, not self-consistency. The
  canary's own oracle is runtime-independent only (Python vs Rust, same author);
  it proves the mechanism, not the independence.
- **The `unsafe` census (#18) is compiler-enforced, not tool-dependent.**
  `canary-core` uses `#![forbid(unsafe_code)]` (census provably 0). `canary-parse`
  uses `#![deny(unsafe_code)]`, so every `unsafe` site must carry an explicit
  `#[allow(unsafe_code)]` — the audited set is exactly
  `grep -rnE '^\s*#\[allow\(unsafe_code\)\]' src` (one site). This is stronger
  than cargo-geiger, which can silently stop running; Miri then proves the one
  site sound.

## The gates are not vacuous (negative proof)

Two experiments on the canary, both reverted:

- **Inject a real bug** — changing the `acos` domain clamp from `[-1, 1]` to
  `[-1, 0.9]` made `sep_identical_is_zero` **FAIL**. Regressions are caught.
- **Weaken the tests** — disabling just the exact-value unit tests dropped the
  mutation score from 100% to **22/34**, with the survivor
  `replace angular_separation -> 1.0`. The mutation gate detects weak tests, not
  just weak code.

## The org baseline is a floor

A rig may *tighten* any gate (higher coverage/mutation thresholds, `pedantic`
clippy, longer fuzz campaigns, stricter license set) freely. It may *loosen* a
gate only via a named, reviewed override recorded in the rig's overlay — never
silently.

## Tier-3 upgrade path (out of scope here, documented for later)

SLSA build-provenance attestation, sigstore-signed commits & tags, `cargo-vet`
dependency trust, reproducible-build verification, harden-runner egress control,
and published coverage/mutation/parity dashboards. Adopt when a rig needs
supply-chain-grade guarantees.
