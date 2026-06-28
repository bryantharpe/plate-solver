#!/usr/bin/env bash
#
# ralph-grind.sh — true "Ralph is a bash loop" driver for the plate-solver grind plan.
#
# WHY THIS EXISTS:
#   The built-in /loop (ralph-loop plugin) runs the loop *inside one session* via a Stop
#   hook — it re-feeds the same prompt without clearing context, so context still grows and
#   Claude Code's auto-compact can fire in the MIDDLE of a task. This script instead launches
#   a FRESH `claude -p` process per task. Each task therefore starts at ~zero context, so
#   compaction never happens mid-task. State is durable in plan.md (the plan was authored as
#   the single cold-restart source of truth), so a fresh context each task is lossless.
#
# WHAT EACH ITERATION DOES:
#   one fresh Sonnet orchestrator process -> read plan.md -> do exactly ONE unblocked task to
#   completion (delegate Rust to ps-coder/Qwen, review via ps-judge/Sonnet, run the gate, check
#   the box, append the Run Log, commit work+plan) -> exit. The bash loop then relaunches for
#   the next task until the plan is done / all-blocked / no progress / max iterations.
#
# USAGE:
#   tools/ralph-grind.sh [--detach] [--offline|--online] [MAX_ITERATIONS]   # default 60
#
# OFFLINE MODE (e.g. on a plane, no connectivity):
#   The orchestrator AND ps-judge normally run on real Anthropic Sonnet, which needs the network.
#   --offline (or AUTO when api.anthropic.com is unreachable) switches to a fully-local path:
#     * orchestrator -> qwen3.6-27b (local), ps-coder -> qwen3.6-27b (local), ps-judge -> SKIPPED.
#     * each task is still implemented and its FULL integrity gate is run locally (build + tests +
#       golden-fixture parity all run offline), but instead of frontier review the task is marked
#       JUDGE-PENDING `[~]` and queued in plan.md's `## Judge Queue`, then committed.
#   Next time the loop runs ONLINE, it DRAINS the Judge Queue FIRST: ps-judge reviews each pending
#   task's committed diff vs its AC, promoting `[~]`->`[x]` on PASS (or opening a fix task on FAIL),
#   before any new task is started. So offline work is never un-reviewed — only review-deferred.
#
# IMPORTANT:
#   * Do NOT run this while an interactive `grind` session is also working this repo — two
#     loops will collide on plan.md and git. Stop the interactive session first.
#   * Models/routing match memory `grind-orchestrator-model` + `llm-routing-litellm`:
#     ONLINE: orchestrator = claude-sonnet-4-6-real, ps-coder = qwen3.6-27b, ps-judge = claude-sonnet-4-6-real.
#     OFFLINE: orchestrator = qwen3.6-27b, ps-coder = qwen3.6-27b, ps-judge = deferred; small/background -> local qwen.

set -uo pipefail

# ---- config -----------------------------------------------------------------
REPO="/Users/bryant/code/plate-solver"
ORCH_MODEL_ONLINE="claude-sonnet-4-6-real"   # real Anthropic Sonnet (needs connectivity)
ORCH_MODEL_OFFLINE="qwen3.6-27b"             # local Qwen via LiteLLM (works on a plane)
LOG_DIR="${RALPH_LOG_DIR:-$HOME/.cache/ralph-grind/plate-solver}"

# ---- arg parse --------------------------------------------------------------
#   --detach          run the loop in a detached tmux session, then exit
#   --offline         force OFFLINE mode (local Qwen orchestrator, defer all ps-judge review)
#   --online          force ONLINE mode (real Anthropic orchestrator + judge; default behavior)
#   (no mode flag)    AUTO: probe api.anthropic.com and pick online/offline automatically
# Flags may appear in any order before the optional MAX_ITERATIONS positional arg.
DETACH=0
MODE="auto"
while [ $# -gt 0 ]; do
  case "${1:-}" in
    --detach)  DETACH=1; shift ;;
    --offline) MODE="offline"; shift ;;
    --online)  MODE="online"; shift ;;
    --*) echo "FATAL: unknown flag $1"; exit 1 ;;
    *) break ;;
  esac
done
MAX_ITERS="${1:-60}"
# flag string to forward to the detached re-launch (preserves the chosen mode)
MODE_FLAG=""; [ "$MODE" = "offline" ] && MODE_FLAG="--offline"; [ "$MODE" = "online" ] && MODE_FLAG="--online"

# connectivity probe: 0 (online) if real Anthropic answers within 4s, else 1 (offline).
# curl exit 0 == got an HTTP response (even 401/405 ⇒ reachable); 6/7/28 == DNS/connect/timeout.
connectivity() { curl -m4 -sS -o /dev/null https://api.anthropic.com/v1/messages >/dev/null 2>&1; }

# --detach: relaunch this script inside a detached tmux session, then exit. The tmux server is
# independent of the launching shell, so the loop SURVIVES killing any Claude session or closing
# the terminal. Re-attach any time for live visibility.
if [ "$DETACH" = 1 ]; then
  command -v tmux >/dev/null 2>&1 || { echo "FATAL: --detach requires tmux (brew install tmux)"; exit 1; }
  if pgrep -fl 'claude --dangerously-skip-permissions' >/dev/null 2>&1; then
    echo "REFUSING to detach: a 'claude --dangerously-skip-permissions' process is still running."
    echo "Stop the interactive grind session first (AFTER its current task COMMITS), then retry."
    exit 1
  fi
  if tmux has-session -t ralph-grind 2>/dev/null; then
    echo "tmux session 'ralph-grind' already exists — attach: tmux attach -t ralph-grind"
    exit 1
  fi
  mkdir -p "$LOG_DIR"
  tmux new-session -d -s ralph-grind "cd '$REPO' && '$REPO/tools/ralph-grind.sh' $MODE_FLAG $MAX_ITERS"
  echo "✅ Detached grind loop in tmux session 'ralph-grind' (mode=$MODE, max $MAX_ITERS)."
  echo "   Survives killing any Claude session / closing this terminal."
  echo "   watch  : tmux attach -t ralph-grind        (then detach with: Ctrl-b, then d)"
  echo "   tail   : tail -f $LOG_DIR/iter-*.log"
  echo "   commits: git -C '$REPO' log --oneline"
  echo "   stop   : tmux kill-session -t ralph-grind"
  exit 0
fi

# ---- ONLINE one-iteration prompt --------------------------------------------
# Forces exactly ONE task then exit (overriding grind's "iterate until done"), because the OUTER
# bash loop provides the iteration — each task gets a fresh context.
read -r -d '' PROMPT_ONLINE <<'EOF' || true
You are running ONE iteration of an autonomous implementation loop. The file plan.md in the
current working directory is the SINGLE SOURCE OF TRUTH (Purpose/DoD, Loop Protocol, Guardrails,
Tasks, and the Decisions/Blocked/Run logs).

Do EXACTLY ONE unit of work this run, then STOP. Do not start a second. Do not "keep going". The
process will exit and a fresh one will start next. Concretely:

0. JUDGE QUEUE FIRST. Read plan.md's `## Judge Queue`. If it has ANY unchecked `- [ ]` entry, this
   iteration's ONE unit of work is to DRAIN it — do NOT start a new task this run:
     a. For each pending entry (it names a task ID + that task's exact commit message), locate the
        commit with `git log --grep="<exact commit message>" -n1 --format=%H` and get its diff with
        `git show <sha>`. Hand the diff + that task's Acceptance Criteria to `ps-judge` (Job A).
     b. PASS → change that task's checkbox from `[~]` to `[x]`, and check off `[x]` its Judge Queue
        entry. FAIL → leave the task `[~]`, append a NEW follow-up fix task to the task list with the
        judge's concrete issues + `deps:` on the original, append a Decisions Log note, and check off
        the queue entry (the follow-up task now tracks it). Re-running the gate yourself is fine but
        the code already passed the gate offline; the judge is reviewing AC/architecture.
     c. Independently re-run each judged task's integrity gate to confirm it is still green on HEAD.
     d. Commit plan.md (and only plan.md) with message "chore(grind): drain judge queue (N tasks)".
        Then STOP. The next iteration starts fresh work.
   If the Judge Queue is empty or absent, proceed to step 1 normally.

1. Read plan.md for: Purpose/DoD, ALL Guardrails (in full — load-bearing), the task list
   (checkboxes + deps, to pick the next task), the Decisions Log, the Blocked Log, and only the
   LAST ~5 Run Log entries (the Run Log is append-only history you do NOT need in full — read the
   tail, e.g. `tail -n 40 plan.md`, not the whole file). Do NOT read cited spec/reference docs
   into THIS orchestrator context; delegate any heavy reading to a single `Explore` or `ps-coder`
   subagent and keep raw spec text out of this window (it is re-read every turn and is the main
   token cost).
2. Select the FIRST task whose checkbox is unchecked AND whose `deps:` are all `[x]`, and which
   is NOT already marked BLOCKED. If no such task exists, output exactly NOTHING-TO-DO and stop.
3. Implement that ONE task to its acceptance criteria, obeying EVERY guardrail. Delegate
   mechanical Rust coding to the `ps-coder` subagent and review the result with the `ps-judge`
   subagent exactly as the plan's Loop Protocol / delegation policy requires; use targeted
   Read/Grep or a single `Explore` subagent for heavy reading (keep raw text out of this context).
4. Run the task's integrity gate YOURSELF (never weaken/ignore/stub it). If it cannot pass this
   run, follow the plan's don't-stall rule: mark the task BLOCKED, append a Blocked Log entry
   (reason + recommended fix), commit the plan, and STOP.
5. On green: check the box `[x]`, append the one-line Run Log entry, and commit the work AND
   plan.md together with the task's EXACT commit message, staging explicit in-scope paths only
   (never `git add -A`, never push, never commit to main/docs branches).
6. STOP.

Model policy (already wired via the LiteLLM router): this orchestrator process is
claude-sonnet-4-6-real (real Anthropic Sonnet); ps-coder = qwen3.6-27b (local); ps-judge =
claude-sonnet-4-6-real (real Anthropic Sonnet — a rigorous frontier reviewer).
EOF

# ---- OFFLINE one-iteration prompt -------------------------------------------
# No connectivity: orchestrator + coder are local Qwen, ps-judge is UNAVAILABLE. The full integrity
# gate still runs locally; frontier review is DEFERRED via the Judge Queue and `[~]` task state.
read -r -d '' PROMPT_OFFLINE <<'EOF' || true
You are running ONE iteration of an autonomous implementation loop in OFFLINE mode (no network /
no Anthropic). The file plan.md in the current working directory is the SINGLE SOURCE OF TRUTH.

CRITICAL OFFLINE CONSTRAINTS:
 * `ps-judge` (real Anthropic Sonnet) is UNREACHABLE this run — do NOT invoke it, and do NOT invoke
   the built-in `Explore` subagent (its background calls route to unreachable Anthropic and will
   hang/fail). Do heavy reading with targeted Read/Grep or by delegating to `ps-coder` (local Qwen).
 * You (the orchestrator) and `ps-coder` are BOTH the local model qwen3.6-27b. There is no frontier
   review this run; review is DEFERRED and will happen on the next ONLINE run.
 * The integrity gate (cargo build/test + golden-fixture parity) is fully LOCAL and MUST still run
   and pass — it is your only correctness signal offline. Never weaken/stub/skip it.

Do EXACTLY ONE task this run, then STOP. Concretely:

1. Read plan.md: Purpose/DoD, ALL Guardrails (in full), the task list (checkboxes + deps), the
   Decisions Log, the Blocked Log, the `## Judge Queue`, and the LAST ~5 Run Log entries
   (`tail -n 40 plan.md`, not the whole file).
2. Select the FIRST task that is unchecked `[ ]`, NOT BLOCKED, whose `deps:` are each either `[x]`
   OR `[~]` (a `[~]` dep is coded + gate-green, just not yet frontier-judged — fine to build on
   offline), AND whose Acceptance Criteria do NOT require an architectural decision delegated "to
   ps-judge (Job B)". SKIP any task that needs a ps-judge architectural decision — it must wait for
   connectivity; move to the next eligible task. If no eligible task exists, output exactly
   OFFLINE-NOTHING-TO-DO and stop.
3. Implement that ONE task to its AC, obeying EVERY guardrail. Delegate mechanical Rust to
   `ps-coder` (local Qwen). YOU make any needed implementation decision (record non-obvious ones in
   the Decisions Log, tagged "(offline, unreviewed)"); do not defer routine decisions.
4. Run the task's integrity gate YOURSELF. If it cannot pass, follow the don't-stall rule: mark the
   task BLOCKED, append a Blocked Log entry (reason + fix), commit the plan, and STOP.
5. On green — DO NOT mark `[x]` and DO NOT invoke ps-judge. Instead:
     a. Mark the task checkbox `[~]` (coded + gate-green, JUDGE-PENDING).
     b. Append an entry to `## Judge Queue`: `- [ ] <TASK-ID> — commit "<exact commit message>" — coded offline <YYYY-MM-DD>, gate green, awaiting ps-judge`.
     c. Append a one-line Run Log entry noting "OFFLINE — gate green, judge deferred".
     d. Commit the work AND plan.md together with the task's EXACT commit message (explicit in-scope
        paths only; never `git add -A`, never push, never commit to main/docs branches).
6. STOP.
EOF

# ---- env (mirrors the documented launch recipe) -----------------------------
export ANTHROPIC_SMALL_FAST_MODEL="${ANTHROPIC_SMALL_FAST_MODEL:-claude-haiku-4-5}"
# shellcheck disable=SC1090
source "$HOME/mac-llm-env/scripts/claude-code-env.sh"
export PATH="$HOME/.cargo/bin:$PATH"   # cargo/rustc not on default PATH (see Decisions Log)

# ---- preflight --------------------------------------------------------------
cd "$REPO" || { echo "FATAL: cannot cd to $REPO"; exit 1; }
[ -f plan.md ] || { echo "FATAL: plan.md not found in $REPO"; exit 1; }
mkdir -p "$LOG_DIR"

if pgrep -fl 'claude --dangerously-skip-permissions' >/dev/null 2>&1; then
  echo "⚠️  WARNING: another 'claude --dangerously-skip-permissions' process is running."
  echo "   If that is an interactive grind on this repo, STOP it first — concurrent loops will"
  echo "   collide on plan.md and git. Ctrl-C now to abort, or wait 8s to proceed anyway."
  sleep 8
fi

# ---- resolve mode (online vs offline) ---------------------------------------
if [ "$MODE" = "auto" ]; then
  if connectivity; then MODE="online"; else MODE="offline"; fi
  echo "🔎 AUTO mode probe: api.anthropic.com $([ "$MODE" = online ] && echo reachable || echo UNREACHABLE) → $MODE"
fi
if [ "$MODE" = "offline" ]; then
  ORCH_MODEL="$ORCH_MODEL_OFFLINE"
  PROMPT="$PROMPT_OFFLINE"
  export ANTHROPIC_SMALL_FAST_MODEL="qwen3.6-27b"   # keep all background/small calls local offline
  echo "✈️  OFFLINE: orchestrator=$ORCH_MODEL, ps-judge DEFERRED (tasks land as [~] JUDGE-PENDING)."
else
  ORCH_MODEL="$ORCH_MODEL_ONLINE"
  PROMPT="$PROMPT_ONLINE"
fi

stamp() { date "+%Y-%m-%d %H:%M:%S"; }
# Run-id: timestamp-based suffix so concurrent/sequential runs don't overwrite each other's logs.
# Format: run-2026-06-28T08-01-11 — collides only if you start two runs in the same second.
RUN_ID="run-$(date '+%Y-%m-%dT%H-%M-%S')"
LOG_DIR="$LOG_DIR/$RUN_ID"
# NB: `grep -c` prints "0" AND exits 1 on no match, so `grep -c ... || echo 0` would double-print
# ("0\n0") and break arithmetic. Capture into a var and default-if-empty instead (empty only if
# plan.md is unreadable). pending_count is legitimately 0 in normal online runs — the hot path.
_count() { local n; n=$(grep -c "$1" plan.md 2>/dev/null) || true; echo "${n:-0}"; }
done_count()    { _count '^- \[x\]'; }
open_count()    { _count '^- \[ \]'; }
pending_count() { _count '^- \[~\]'; }   # coded, gate-green, judge-deferred
commit_count()  { local n; n=$(git rev-list --count HEAD 2>/dev/null) || true; echo "${n:-0}"; }

echo "=== ralph-grind start $(stamp) | mode=$MODE | model=$ORCH_MODEL | max=$MAX_ITERS | logs=$LOG_DIR ==="
echo "    tasks: $(done_count) done / $(pending_count) judge-pending / $(open_count) open"

# ---- the loop ---------------------------------------------------------------
for ((i=1; i<=MAX_ITERS; i++)); do
  open=$(open_count); pend=$(pending_count)
  if [ "$open" -eq 0 ] && [ "$pend" -eq 0 ]; then
    echo "✅ All task checkboxes are [x] (none judge-pending). Done at $(stamp)."
    break
  fi
  if [ "$MODE" = "offline" ] && [ "$open" -eq 0 ] && [ "$pend" -gt 0 ]; then
    echo "✈️  OFFLINE: no open tasks left; $pend task(s) are JUDGE-PENDING. Reconnect and run"
    echo "   ONLINE to drain the Judge Queue (ps-judge review). Stopping at $(stamp)."
    break
  fi

  # progress is "done + judge-pending": offline a completed task moves [ ]→[~], not [ ]→[x].
  p_before=$(( $(done_count) + pend )); c_before=$(commit_count)
  log="$LOG_DIR/iter-$(printf '%03d' "$i").log"
  echo "── iteration $i  $(stamp)  ($(done_count) done, $pend pending, mode=$MODE) → $log"

  # Fresh process == fresh context. Headless, autonomous.
  claude -p "$PROMPT" \
      --model "$ORCH_MODEL" \
      --dangerously-skip-permissions \
      2>&1 | tee "$log"

  p_after=$(( $(done_count) + $(pending_count) )); c_after=$(commit_count)

  if grep -qE 'NOTHING-TO-DO|OFFLINE-NOTHING-TO-DO' "$log"; then
    echo "🟦 Orchestrator reported NOTHING-TO-DO (done, all blocked, or — offline — all remaining"
    echo "   work needs connectivity). Stopping."
    break
  fi
  if [ "$c_after" -eq "$c_before" ] && [ "$p_after" -eq "$p_before" ]; then
    echo "🛑 No new commit and no forward progress this iteration."
    echo "   Likely a BLOCK that didn't commit, or the run errored. Inspect $log. Stopping."
    break
  fi
  echo "   advanced: progress ${p_before}→${p_after}, commits ${c_before}→${c_after}"
done

echo "=== ralph-grind end $(stamp) | $(done_count) done / $(pending_count) judge-pending / $(open_count) open ==="
echo "Recent commits:"; git log --oneline -5
