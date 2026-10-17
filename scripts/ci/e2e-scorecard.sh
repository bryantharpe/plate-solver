#!/usr/bin/env bash
# e2e-scorecard.sh DIR — turn the e2e tiers' scorecards into evidence, and judge them.
#
# Reads DIR/E1.json, DIR/E2.json and DIR/E3.json, which the eval tests write themselves
# (before asserting, so a red run keeps its numbers). Renders one table to the job
# summary and sets `measured` for the attestation.
#
# A TIER IS GREEN ONLY IF ITS SCORECARD SAYS SO, CHECKED HERE: the right shape, an empty
# failure list, finite worst-case numbers, and the tier's own conditions (every image
# cedar solves solved, every identity check true). That is judged independently of the
# test's asserts, so a test that forgot to record a failure still cannot paint a row
# green. A missing scorecard is a failure too: "did not run" if the tier never started,
# "crashed" (with its panic) if it died before writing one.
#
# EXIT: 0 = all three tiers green · 1 = otherwise.

set -uo pipefail

dir="${1:?usage: e2e-scorecard.sh DIR}"
summary="${GITHUB_STEP_SUMMARY:-/dev/stdout}"
status=0
rows=()
parts=()
details=()

shape='(.tier|type) == "string" and (.cases|type) == "number" and (.expected_solved|type) == "number"
       and (.solved|type) == "number" and (.failures|type) == "array"'
# serde_json writes NaN and infinity as null: a worst case that is not a number is red.
finite='[.worst_pos_arcsec, .worst_roll_arcsec, .worst_fov_arcsec] | all(type == "number")'
declare -A green=(
  [E1]='.cases == 21 and .solved == .expected_solved and .identical_ids == .expected_solved
        and .hinted.solved == .expected_solved and .hinted.identical_ids == .expected_solved'
  [E2]='.cases == 21 and .solved == .expected_solved and .composed_identical == .cases
        and (.worst_cedar_recall | type) == "number" and .worst_cedar_recall >= 0.9'
  [E3]='.cases == 21 and .solved == .expected_solved and .matches_library == .cases
        and .matches_composed == .cases and .hale_bopp == "MATCH_FOUND"'
)

for tier in E1 E2 E3; do
  f="$dir/$tier.json"
  log="$dir/$tier.log"
  if [[ ! -s "$f" ]]; then
    if [[ -s "$log" ]]; then
      why="$(grep -m1 -A1 'panicked at' "$log" | tail -1 | cut -c1-160)"
      echo "::error::$tier crashed before writing its scorecard: ${why:-see the $tier step log}"
      rows+=("| $tier | **crashed** | ${why//|/\\|} | | | |")
      parts+=("$tier CRASHED")
    else
      echo "::error::$tier wrote no scorecard — the tier did not run, so it cannot have passed"
      rows+=("| $tier | **did not run** | | | | |")
      parts+=("$tier DID NOT RUN")
    fi
    status=1
    continue
  fi
  if ! jq -e "$shape" "$f" >/dev/null 2>&1; then
    echo "::error::$tier scorecard is malformed (missing or mistyped fields)"
    rows+=("| $tier | **malformed scorecard** | | | | |")
    parts+=("$tier MALFORMED")
    status=1
    continue
  fi
  verdict=green
  if ! jq -e "(.failures | length) == 0 and ($finite) and (${green[$tier]})" "$f" >/dev/null 2>&1; then
    verdict=red
    status=1
    n="$(jq '.failures | length' "$f")"
    details+=("**$tier** — $n recorded failure(s); checked conditions: \`${green[$tier]//$'\n'/ }\`")
    while IFS= read -r line; do details+=("- \`$line\`"); done < <(jq -r '.failures[:20][]' "$f")
  fi
  if ! row="$(jq -r --arg verdict "$verdict" '
    def f(p): if type == "number" then (. * pow(10; p) | round) / pow(10; p) | tostring else "null" end;
    "| \(.tier) \(.name) | \(.solved)/\(.expected_solved) | " +
    (if .tier == "E1" then "\(.identical_ids)/\(.expected_solved) identical star sets · \(.hinted.identical_ids)/\(.expected_solved) with an FOV hint"
     elif .tier == "E2" then "\(.composed_identical)/\(.cases) identical to the composed path · " +
       (if .solved > 0 then "\((.worst_cedar_recall * 100) | round)% of cedar stars" else "no solves to compare" end)
     else "\(.matches_library)/\(.cases) = library · \(.matches_composed)/\(.cases) = composed · hale_bopp \(.hale_bopp)" end) +
    " | " +
    # With nothing solved there is no worst error: a 0″ here would read as agreement.
    (if .solved == 0 then "— (nothing solved)"
     elif .tier == "E1" then "\(.worst_pos_arcsec | f(4))″ / \(.worst_roll_arcsec | f(4))″ / \(.worst_fov_arcsec | f(4))″"
     else "\(.worst_pos_arcsec | f(2))″ / \(.worst_roll_arcsec | f(1))″ / \(.worst_fov_arcsec | f(2))″" end) +
    " | " + (if .solve_ms_median then "\(.solve_ms_median | f(1)) ms" else "—" end) +
    " | \(.failures | length)" + (if $verdict == "red" then " **red**" else "" end) + " |"' "$f")"; then
    row="| $tier | **scorecard unreadable** | | | | |"
    status=1
  fi
  rows+=("$row")
  if [[ "$verdict" == red ]]; then
    parts+=("$(jq -r '"\(.tier) RED — \(.solved)/\(.expected_solved) solved, \(.failures | length) failure(s)"' "$f")")
  else
    parts+=("$(jq -r '
      def f(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
      if .tier == "E1" then "E1 \(.identical_ids)/\(.expected_solved) identical to cedar, \(.hinted.identical_ids)/\(.expected_solved) with FOV hints (worst \(.worst_pos_arcsec | f(4))″)"
      elif .tier == "E2" then "E2 \(.solved)/\(.expected_solved) from pixels (worst \(.worst_pos_arcsec | f(2))″)"
      else "E3 \(.solved)/\(.expected_solved) over gRPC (worst \(.worst_pos_arcsec | f(2))″)" end' "$f")")
  fi
done

joined="$(IFS='|'; echo "${parts[*]}")"
measured="${joined//|/ · }"
if [[ -n "${E2E_DISARMED_UNTIL:-}" ]]; then measured="DISARMED until $E2E_DISARMED_UNTIL — $measured"; fi

{
  echo "### e2e-eval — the product against the cedar oracle"
  echo
  echo "| Tier | Solved | Agreement | Worst Δ (position / roll / FOV) | Median solve | Failures |"
  echo "|---|---|---|---|---|---|"
  printf '%s\n' "${rows[@]}"
  echo
  if ((${#details[@]})); then
    echo "<details open><summary>Failures</summary>"
    echo
    printf '%s\n' "${details[@]}"
    echo
    echo "</details>"
  fi
  if [[ -n "${E2E_DISARMED_UNTIL:-}" ]]; then
    echo "> **DISARMED until ${E2E_DISARMED_UNTIL}** (\`vars.E2E_ARMED\`): reported, not enforced."
  fi
} >>"$summary"

echo "measured: $measured"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "measured=$measured" >>"$GITHUB_OUTPUT"; fi
exit "$status"
