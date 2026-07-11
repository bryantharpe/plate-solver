# Universal gates (language-agnostic)

These apply to **every** rig regardless of language. The Rust layer
(`../rust/GATES.md`) is the first concrete implementation; a future `go/`, `ts/`,
or `python/` layer implements the same universal gates with that ecosystem's
tools.

| # | Claim | Gate | Threshold |
|---|-------|------|-----------|
| U1 | "No committed secrets" | secret scanner (ripsecrets / gitleaks) in CI + pre-commit | none |
| U2 | "CI is not itself a supply-chain hole" | third-party actions pinned by commit SHA; least-privilege `permissions:`; `concurrency` cancel | enforced in review |
| U3 | "Every change was independently reviewed" | Opus (cloud-Claude) adversarial review, different lineage than the author | required before merge |
| U4 | "Risky changes had a human" | branch protection: required checks + required review; human sign-off on critical paths / reviewer dissent | enforced |
| U5 | "Dependencies are updated, not rotting" | Dependabot / Renovate | PRs kept current |
| U6 | "We can enumerate what we ship" | SBOM generation (CycloneDX / SPDX) | artifact per release |
| U7 | "Every line traces to an approved spec" | provenance chain: spec → bead → PR → CI → review → merge | auditable |
| U8 | "Builds are reproducible" | locked dependency manifests; pinned toolchain | `--locked` / lockfile committed |

## Notes

- **U3 is the anti-self-selection-bias rule.** Authors are local qwen/glm models;
  the authoritative reviewer is Opus. Same-lineage review (a model reviewing its
  own family's output) is explicitly *not* sufficient. The invocation mechanism
  (CI job → Opus → required status check `review/opus`) is specified in
  [`OPUS-REVIEW.md`](./OPUS-REVIEW.md) — specified, not yet wired in any consumer.
- **U2 and U8** are what stop the verification pipeline from becoming the attack
  surface — an unpinned action or a wildcard dependency silently defeats every
  other gate.
- The **Tier-3** universal upgrades (SLSA provenance attestation, sigstore
  signing, DCO sign-off, egress-controlled runners) layer on top of these.
