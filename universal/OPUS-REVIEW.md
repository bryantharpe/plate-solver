# Independent Opus review — invocation mechanism (U3)

> **Status: SPECIFIED — not yet wired.** As of 2026-07-11 no consumer repo runs
> this job and no `review/opus` status check exists anywhere. This document is
> the buildable spec for the U3 gate ("every change was independently reviewed"),
> closing audit item S7: the docs previously claimed the gate with no defined
> mechanism. Remove this notice per-rig when the job is installed and the check
> is required by branch protection.

## What it is

A CI job that runs on every pull request targeting a protected branch, sends the
change to an **Opus** (cloud-Claude) model for adversarial review, and reports
the verdict as a commit status check named **`review/opus`**. Opus is a
different model lineage than the qwen/glm authors, which is the whole point of
U3: the reviewer and the author are not the same reasoner.

## Trigger

- `pull_request` events (opened, synchronize, reopened) whose **base branch is
  protected** (e.g. `rewrite`, `main`). Every new push to the PR re-runs the
  review against the updated diff — the check attaches to the head SHA, so a
  stale approval cannot carry over to new commits.
- The job runs in the consumer rig's own CI (alongside the fast Tier-2 gates in
  `rust/ci/ci.yml`), not in this standards repo.

## Inputs sent to the reviewer

The job assembles one review prompt from three parts, all taken from the PR's
own checkout (never from the author's claims):

1. **The diff** — `git diff <base>...<head>` for the PR, plus the PR title and
   body.
2. **The spec** — the approved openspec change / bead description the PR claims
   to implement (the provenance chain requires the PR to name it).
3. **The standards** — the rig's materialized `standards/` subtree (at minimum
   `universal/GATES.md`, the language-layer `GATES.md`, and the rig overlay's
   critical-path list), so the reviewer judges against the ratified rules, not
   its general taste.

## Model and endpoint

- **Model: `claude-opus-4-8`** (or the OpenRouter alias
  `anthropic/claude-opus-4.8`). An OpenRouter-compatible chat-completions
  endpoint is acceptable; this matches the proven `tools/judge` precedent
  (plate-solver's `openrouter-judge.py`: stdin prompt → OpenRouter →
  `temperature: 0` → stdout verdict).
- Direct Anthropic API is equally acceptable. What is **not** acceptable is any
  same-lineage substitute (qwen/glm reviewing qwen/glm output) — that silently
  voids U3 even if the check goes green.
- **Credentials**: the API key lives in a repo/org **secret**
  (`OPENROUTER_API_KEY` or `ANTHROPIC_API_KEY`), exposed only to this job.
  Standard fork caveat applies: secrets are unavailable to fork PRs, so
  fork-origin PRs fail closed (check stays pending/failing → human review).

## Outputs

1. **Required status check `review/opus`** on the PR head SHA:
   - `success` — reviewer approves: no correctness, security, spec-conformance,
     or standards violations found.
   - `failure` — reviewer **dissents** (any finding it labels blocking), or the
     review could not be completed (API error, missing spec reference,
     oversized diff). Fail closed, never open.
2. **A review comment** on the PR carrying the full verdict: findings, severity,
   and the explicit APPROVE / DISSENT line the status was derived from. The
   comment is the audit artifact; the status is the enforcement.

## Enforcement semantics

- Branch protection on the target branch lists `review/opus` as a **required
  status check**. A dissent is therefore a failing check: the PR cannot merge
  until either (a) the author addresses the findings and a fresh review passes,
  or (b) a **human** reviews the dissent and overrides via the existing
  required-review approval path (U4). Dissent = human required — the fleet
  never argues its way past the reviewer.
- This check is *in addition to* the fast Tier-2 gates, not a substitute: green
  mechanized gates get the PR *to* review, never *through* it.

## Non-goals

- This job does not fix code, push commits, or approve GitHub reviews — it only
  reports. Merging remains gated by branch protection.
- It does not replace human sign-off on critical paths (U4); the rig overlay's
  critical-path list still routes those changes to a human even on APPROVE.

## Implementation checklist (for the wiring change, not this spec)

- [ ] Reusable workflow `universal/ci/opus-review.yml` (actions pinned by SHA,
      `permissions: contents: read, statuses: write, pull-requests: write`).
- [ ] Prompt template embedding diff + spec + standards with explicit
      APPROVE/DISSENT output contract.
- [ ] Install in consumer rig CI; add `review/opus` to required checks.
- [ ] Prove one end-to-end dissent → human override on the canary before
      declaring the gate ENFORCED.
