# Governance & releases

## How these standards change

The standards are **human-ratified**. Agents consume them (read-only in every
rig); they do not amend them. A change to a gate, threshold, or the loop is a
pull request to *this* repo, reviewed and merged by a human, then released under
a semantic version.

- **Patch** (`vX.Y.Z+1`) — clarifications, fixed SHAs, doc wording. No behavior
  change for consumers.
- **Minor** (`vX.Y+1.0`) — a new *non-blocking* gate, a new optional layer, a
  loosened threshold.
- **Major** (`vX+1.0.0`) — a new **blocking** gate or a tightened threshold that
  can turn a previously-green rig red. Consumers adopt on their own schedule.

## How a rig adopts a version

1. The town-config pins the **blessed** version (the one new rigs get by default).
2. A rig imports that version via `git subtree` into `standards/` (real files,
   present after a plain clone — no submodule empty-dir footgun).
3. Adoption is **opt-in per rig and never mid-work**: a rig bumps its pinned
   version deliberately, re-runs its gates, and merges the bump like any change.
4. A staleness check (`standards-sync --check`, surfaced in the patrol digest) is
   **active but advisory** — it reports drift from the blessed version; it does
   not force an upgrade.

## Release log

### v1.0 (tagged 2026-07-11)

- Initial **Tier-2** Rust gate set (`rust/GATES.md`), proven end-to-end against
  the canary: fmt, clippy `-D`, test+coverage (100%), mutation (39/39),
  proptest, fuzz (1e6 runs), differential parity, criterion, cargo-deny (incl.
  licenses), cargo-audit, semver-checks, machete, doc `-D`, compiler-enforced
  unsafe census + Miri, secret scan, MSRV, SBOM.
- Universal gate set (`universal/GATES.md`).
- Reusable workflows (`rust/ci/ci.yml` fast, `rust/ci/deep.yml` nightly).
- Environment overview (`ENVIRONMENT.md`) + rig overlay template.
- Merge gate: green CI → independent **Opus** review → human on critical
  paths/dissent.
- Opus-review invocation mechanism specified (`universal/OPUS-REVIEW.md`, audit
  item S7): PR-triggered CI job → `claude-opus-4-8` → required status check
  `review/opus`; dissent fails the check and routes to a human. Specified only —
  wiring into a consumer is post-v1.0 work.
