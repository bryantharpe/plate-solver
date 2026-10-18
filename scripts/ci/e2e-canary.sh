#!/usr/bin/env bash
# e2e-canary.sh — can the e2e gate still fail, and does each of its checks still bite?
#
# WHY THIS EXISTS
# ---------------
# A gate that cannot fail is decoration. main shipped 156 green tests that could not fail; the e2e gate must not
# become the 157th. Two families of mutants, one at a time, each required to turn the
# e2e tier that owns it RED, then put back:
#
#   BUGS    re-injected from the audit (docs/audit-e2e-parity.md): realistic failures.
#   PROBES  one golden value nudged just past one tolerance, or one star ID dropped:
#           each is caught by exactly one check, so loosening that tolerance, or
#           deleting that check, lets the probe survive and turns this job red.
#
# A mutant counts as CAUGHT only if its tier ran and recorded a failure (for a probe,
# on the case it edited). A crash, a build error or a missing scorecard proves nothing
# and stops the canary. Anchors fail closed: if a refactor moves one, update it in the
# same PR. Probes need no rebuild: the tests read the fixture at run time.
#
# EXIT: 0 = every mutant caught (or gate disarmed) · 1 = a mutant survived ·
#       2 = cannot run (baseline red, anchor missing, mutant does not build or crashed).

set -uo pipefail

if [[ "${E2E_ARMED:-}" =~ ^false-until-([0-9]{4}-[0-9]{2}-[0-9]{2})$ ]] \
  && [[ ! "$(date -u +%F)" > "${BASH_REMATCH[1]}" ]]; then
  echo "e2e-eval is DISARMED until ${BASH_REMATCH[1]}; a canary for a disarmed gate proves nothing."
  [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "measured=n/a — e2e gate DISARMED until ${BASH_REMATCH[1]}" >>"$GITHUB_OUTPUT"
  exit 0
fi

FIXTURE=crates/plate-solver/tests/data/cedar_parity.json
summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
work="${E2E_CANARY_WORK:-}"
own_work=0
if [[ -z "$work" ]]; then work="$(mktemp -d)"; own_work=1; fi
mkdir -p "$work"
restore_file=""
restore() {
  if [[ -n "$restore_file" && -f "$work/orig" ]]; then cp "$work/orig" "$restore_file"; fi
  restore_file=""
}
cleanup() { restore; if ((own_work)); then rm -rf "$work"; fi; }
trap cleanup EXIT

run_tier() { # TIER REPORT_DIR [extra cargo args...]
  local tier="$1" report="$2"
  shift 2
  case "$tier" in
    E1) E2E_REPORT_DIR="$report" cargo test --release --locked -p plate-solver --test reference_parity "$@" ;;
    E2) E2E_REPORT_DIR="$report" cargo test --release --locked -p plate-solver --test image_parity "$@" ;;
    E3) E2E_REPORT_DIR="$report" cargo test --release --locked -p grpc-service --test e2e_service "$@" ;;
    *) echo "unknown tier $tier" >&2; return 2 ;;
  esac
}

# Baseline: an unmutated tree must be green on every tier, or nothing below proves anything.
for t in E1 E2 E3; do
  if ! run_tier "$t" "$work/baseline" >"$work/baseline-$t.log" 2>&1; then
    echo "::error::baseline $t is red on the unmutated tree — the canary cannot prove anything"
    tail -20 "$work/baseline-$t.log"
    exit 2
  fi
done
case0="$(jq -r '.cases[0].image' "$FIXTURE")"

caught=0
total=0
rows=()
judge() { # ID TIER FINDING FILE REQUIRE_CASE — run the tier on the mutated tree and rule
  local id="$1" tier="$2" finding="$3" file="$4" need_case="$5" evidence card="$work/$1/$2.json"
  if run_tier "$tier" "$work/$id" >"$work/$id.log" 2>&1; then
    echo "::error::SURVIVED: $id ($finding) — $tier stayed GREEN"
    evidence="**survived**"
  elif ! jq -e '(.failures | length) > 0' "$card" >/dev/null 2>&1; then
    echo "::error::$id ($finding): $tier failed without recording a failure (crash?) — not evidence"
    tail -20 "$work/$id.log"
    exit 2
  elif [[ -n "$need_case" ]] && ! jq -e --arg c "$need_case" '.failures | any(startswith($c))' "$card" >/dev/null; then
    echo "::error::$id ($finding): $tier went red, but not on the case the probe edited"
    jq -r '.failures[:5][]' "$card"
    exit 2
  else
    caught=$((caught + 1))
    evidence="red: $(jq -r '"\(.solved)/\(.expected_solved) solved, \(.failures | length) failure(s): \(.failures[0] | .[0:90])"' "$card")"
  fi
  echo "$id → $tier: $evidence"
  rows+=("| $id | $finding | \`${file}\` | $tier | ${evidence//|/\\|} |")
}

probe() { # ID TIER JQ_FILTER WHAT — nudge one golden, run, restore
  local id="$1" tier="$2" filter="$3" what="$4"
  total=$((total + 1))
  cp "$FIXTURE" "$work/orig"
  restore_file="$FIXTURE"
  if ! jq "$filter" "$work/orig" >"$FIXTURE"; then
    echo "::error::$id: probe filter failed on the fixture — update the canary"
    exit 2
  fi
  judge "$id" "$tier" "$what" "$FIXTURE" "$case0"
  restore
}

mutate() { # ID TIER FILE OLD NEW FINDING — re-inject one audited bug, run, restore
  local id="$1" tier="$2" file="$3" old="$4" new="$5" finding="$6"
  total=$((total + 1))
  cp "$file" "$work/orig"
  restore_file="$file"
  if ! python3 - "$file" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1:4]
src = open(path, encoding="utf-8").read()
n = src.count(old)
if n != 1:
    sys.exit(f"anchor matched {n} times (expected exactly 1)")
open(path, "w", encoding="utf-8").write(src.replace(old, new, 1))
PY
  then
    echo "::error::$id ($finding): anchor not found in $file — update the canary with the refactor"
    exit 2
  fi
  if ! run_tier "$tier" "$work/$id" --no-run >"$work/$id-build.log" 2>&1; then
    echo "::error::$id ($finding): the mutant does not build — a build error is not a catch"
    tail -20 "$work/$id-build.log"
    exit 2
  fi
  judge "$id" "$tier" "$finding" "$file" ""
  restore
}

# PROBES — one value past one tolerance (on case 0; worst real errors are 0.003" in E1,
# 5.4" position / 30" roll / 14.9" FOV in E2 and E3, so each nudge clears them).
probe P1 E1 '.cases[0].cedar.dec += 6/3600' 'E1 position: golden Dec +6″ (tolerance 5″)'
probe P2 E1 '.cases[0].cedar.roll += 6/3600' 'E1 roll: golden roll +6″ (tolerance 5″)'
probe P3 E1 '.cases[0].cedar.fov += 6/3600' 'E1 FOV: golden FOV +6″ (tolerance 5″)'
probe P4 E1 '.cases[0].cedar.hip |= .[1:]' 'E1 identical stars: one star dropped from the golden'
probe P5 E1 '.cases[0].cedar_hinted.dec += 6/3600' 'E1 FOV-hint pass: hinted golden Dec +6″'
probe P6 E2 '.cases[0].cedar.dec += 20/3600' 'E2 position: golden Dec +20″ (tolerance 10″)'
probe P7 E2 '.cases[0].cedar.roll += 120/3600' 'E2 roll: golden roll +120″ (tolerance 60″)'
probe P8 E2 '.cases[0].cedar.hip += [1, 2, 3]' 'E2 recall: three stars cedar never matched'
probe P9 E3 '.cases[0].cedar.dec += 20/3600' 'E3 position: golden Dec +20″ (tolerance 10″)'
probe P10 E3 '.cases[0].cedar.fov += 30/3600' 'E3 FOV: golden FOV +30″ (tolerance 15″)'

# BUGS — re-injected from the audit.
mutate B1 E1 crates/plate-solver/src/candidates.rs \
  $'fov_estimate: ctx.fov_estimate,\n            fov_max_error: ctx.fov_max_error,' \
  $'fov_estimate: Some(ctx.fov_initial),\n            fov_max_error: Some(ctx.match_max_error),' \
  'S1: FOV filter always on, with the pattern tolerance as its tolerance in degrees'
mutate B2 E1 crates/plate-solver/src/candidates.rs \
  'fov_max_error: ctx.fov_max_error,' \
  'fov_max_error: Some(ctx.match_max_error),' \
  'S1, hint path only: the pattern tolerance used as the FOV tolerance'
mutate B3 E2 crates/plate-solver/src/solve.rs \
  'map(|s| (s.y, s.x))' \
  'map(|s| (s.x, s.y))' \
  'S2: (x, y) handed to a solver that speaks (y, x)'
mutate B4 E3 crates/grpc-service/src/server.rs \
  $'x: s.x,\n                    y: s.y,' \
  $'x: s.y,\n                    y: s.x,' \
  'G1: ExtractCentroids swaps x and y on the wire'
mutate B5 E3 crates/plate-solver/src/refine.rs \
  'let d = rotation.rotate(*cat);' \
  'let d = *cat;' \
  'S5: FOV fitted against sky-frame catalog vectors'

{
  echo "### e2e-canary — every mutant must turn its tier red"
  echo
  echo "| Mutant | What it breaks | File | Tier | Result |"
  echo "|---|---|---|---|---|"
  printf '%s\n' "${rows[@]}"
} >>"$summary"

measured="$caught/$total mutants caught (10 assertion probes, 5 audited bugs)"
echo "measured: $measured"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "measured=$measured" >>"$GITHUB_OUTPUT"; fi
[[ "$caught" == "$total" ]]
