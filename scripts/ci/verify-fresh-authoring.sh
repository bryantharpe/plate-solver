#!/usr/bin/env bash
# verify-fresh-authoring.sh — does this PR's code differ from every prior
# implementation of the same work?
#
# WHY THIS EXISTS
# ---------------
# hq-judge-01. On 2026-07-12, PR #22 contained a byte-identical restore of an
# earlier implementation, recovered from a stale polecat branch. It passed:
#
#   detect · rustfmt · clippy · msrv · docs · test-coverage · cargo-deny ·
#   cargo-machete · secret-scan          — all SUCCESS
#   review/judge                          — APPROVE, zero blocking findings
#
# Ten green checks and an independent judge, on 231 lines produced in 90 seconds
# that were a copy. Every gate we own asks "is this code GOOD?". None of them
# asks "did this agent WRITE it?" — and a copied implementation is, by
# construction, indistinguishable from a well-written one on that question.
#
# For an ordinary product that is fine; reuse is a virtue. For the plate-solver
# rewrite — whose entire purpose is to measure whether a local model can build
# from specs — a copied answer silently VOIDS the experiment while showing 10/10
# green. That is worse than a failure, because it looks like a success.
#
# WHAT THIS CANNOT DO
# -------------------
# It cannot prevent recovery, only detect it. Prior implementations stay
# reachable from main's history FOREVER, and deliberately so: merge-record-audit
# greps commit messages for bead IDs, so the history must be preserved. Pruning
# branches does not help — the reverted merges are still there. Isolation is
# impossible by construction; cleanliness can only be measured after the fact.
# This is a detector, not a wall. Do not describe it as one.
#
# WHAT COUNTS AS "CODE"
# ---------------------
# Product code ONLY. The repo also vendors `reference-solutions/` (the 81 MB
# parity oracle) and `standards/` (the canary reference project). Those carry
# ~20 .rs files that are byte-identical in every commit — comparing them would
# make every PR look like a copy of every other, and the gate would be noise.
#
# EXIT: 0 = fresh (or no product code touched) · 1 = matches a prior · 2 = cannot run.

set -uo pipefail

PRIORS_FILE="${PRIORS_FILE:-.github/provenance-priors.txt}"
BASE_REF="${BASE_REF:-origin/main}"
HEAD_REF="${HEAD_REF:-HEAD}"
# A fresh implementation of a tightly-specified formula can legitimately land
# close to a previous one. Below this many changed lines we WARN, we do not fail:
# a gate that punishes correctness gets switched off, and then it protects nothing.
NEAR_LINES="${NEAR_LINES:-20}"

# Product code, layout-agnostic: run 1 put the crate at ps-core/, run 3 chose
# crates/ps-core/. The next author will choose something else again, and that is
# their call to make — so match on FILE KIND, never on a fixed directory.
CODE=(
  ':(glob)**/*.rs'
  ':(glob)**/Cargo.toml'
  ':(glob)**/Cargo.lock'
  ':(exclude)reference-solutions/**'
  ':(exclude)standards/**'
)

say()  { echo "$*"; }
fail() { echo "::error::$*"; }
warn() { echo "::warning::$*"; }

command -v git >/dev/null 2>&1 || { say "FATAL: git not found"; exit 2; }
git rev-parse --git-dir >/dev/null 2>&1 || { say "FATAL: not a git repo"; exit 2; }

# ── Did this PR touch product code at all? ───────────────────────────────────
# Gate on the DIFF, not on the tree. `detect.has_code` tests whether a root
# Cargo.toml EXISTS, which has been permanently true since the first crate
# landed — it never distinguished a spec-only diff, it distinguished a
# pre-first-crate branch, and that era is over. A docs- or spec-only PR must
# sail through here, and it must do so by REPORTING SUCCESS, not by being
# skipped: a required check whose workflow is filtered away never reports at
# all, and the PR hangs forever on "Expected — waiting for status".
merge_base="$(git merge-base "$BASE_REF" "$HEAD_REF" 2>/dev/null)" || {
  say "FATAL: no merge-base between $BASE_REF and $HEAD_REF (shallow clone? needs fetch-depth: 0)"; exit 2; }

touched="$(git diff --name-only "$merge_base" "$HEAD_REF" -- "${CODE[@]}" 2>/dev/null)"
if [[ -z "$touched" ]]; then
  say "No product code changed in this PR — provenance is not applicable."
  say "PASS"
  exit 0
fi

say "Product code changed by this PR:"
sed 's/^/  /' <<<"$touched"
say ""

# ── Collect the priors ───────────────────────────────────────────────────────
# Explicit list, plus auto-discovery of anything main has REVERTED. A revert is
# the strongest possible signal that an implementation both existed and was
# withdrawn — exactly the thing an agent might go digging for — and relying on a
# human to remember to add it to a text file is how this gate quietly rots.
priors=()
if [[ -f "$PRIORS_FILE" ]]; then
  while read -r line; do
    line="${line%%#*}"; line="$(tr -d '[:space:]' <<<"$line")"
    [[ -z "$line" ]] && continue
    if sha="$(git rev-parse --verify --quiet "${line}^{commit}")"; then
      priors+=("$sha")
    else
      warn "prior '$line' from $PRIORS_FILE is not reachable — ignoring (shallow clone?)"
    fi
  done < "$PRIORS_FILE"
fi

while read -r rsha; do
  [[ -z "$rsha" ]] && continue
  # The reverted implementation is the second parent of the reverted merge, or
  # the commit itself if it was a squash. Take whatever resolves.
  for cand in "${rsha}^2" "$rsha"; do
    if sha="$(git rev-parse --verify --quiet "${cand}^{commit}")"; then priors+=("$sha"); break; fi
  done
done < <(git log "$BASE_REF" --grep='^Revert ' --format='%H' 2>/dev/null | while read -r h; do
           # the SHA the revert names, not the revert commit itself
           git log -1 --format='%b' "$h" 2>/dev/null | grep -oE '\b[0-9a-f]{7,40}\b' | head -1
         done)

# The tip of the base branch is itself a prior implementation: re-running a bead
# whose code is already ON main is exactly the run-4 scenario.
if sha="$(git rev-parse --verify --quiet "${BASE_REF}^{commit}")"; then priors+=("$sha"); fi

# dedupe, and never compare a commit against itself
uniq_priors=()
head_sha="$(git rev-parse "$HEAD_REF")"
for p in "${priors[@]:-}"; do
  [[ -z "$p" || "$p" == "$head_sha" ]] && continue
  for seen in "${uniq_priors[@]:-}"; do [[ "$seen" == "$p" ]] && continue 2; done
  uniq_priors+=("$p")
done

if [[ ${#uniq_priors[@]} -eq 0 ]]; then
  say "No prior implementations to compare against — nothing to recover, nothing to check."
  say "PASS"
  exit 0
fi

say "Comparing against ${#uniq_priors[@]} prior implementation(s):"
say ""

# ── Compare ──────────────────────────────────────────────────────────────────
verdict=0
for p in "${uniq_priors[@]}"; do
  subject="$(git log -1 --format='%h %s' "$p" 2>/dev/null | cut -c1-72)"

  if git diff --quiet "$p" "$HEAD_REF" -- "${CODE[@]}" 2>/dev/null; then
    fail "PROVENANCE FAILURE: this PR's product code is BYTE-IDENTICAL to $subject"
    say  "  This is a restore, not an authoring. The run is void."
    say  "  If you believe this is a false positive, do not remove the gate —"
    say  "  say so on the PR and let a human decide."
    verdict=1
    continue
  fi

  changed="$(git diff --numstat "$p" "$HEAD_REF" -- "${CODE[@]}" 2>/dev/null | awk '{a+=$1+$2} END{print a+0}')"
  if (( changed < NEAR_LINES )); then
    warn "near-identical to $subject — only $changed line(s) differ (threshold $NEAR_LINES)"
    say  "  NOT a failure: a fresh implementation of a tight spec can land close to a prior one."
    say  "  Flagged for a human to eyeball, nothing more."
  else
    say "  ok — $changed lines differ from $subject"
  fi
done

say ""
if (( verdict == 0 )); then say "PASS — this PR's code is distinct from every known prior implementation."
else                         say "FAIL — this PR reproduces prior work."; fi
exit $verdict
