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
#   tools/ralph-grind.sh [MAX_ITERATIONS]      # default 60
#
# IMPORTANT:
#   * Do NOT run this while an interactive `grind` session is also working this repo — two
#     loops will collide on plan.md and git. Stop the interactive session first.
#   * Models/routing match memory `grind-orchestrator-model` + `llm-routing-litellm`:
#     orchestrator = claude-sonnet-4-6-real (real Anthropic Sonnet), ps-coder = qwen3.6-27b (local, $0),
#     ps-judge = claude-sonnet-4-6-real (real Anthropic Sonnet), small/background -> local qwen.

set -uo pipefail

# ---- config -----------------------------------------------------------------
REPO="/Users/bryant/code/plate-solver"
ORCH_MODEL="claude-sonnet-4-6-real"
LOG_DIR="${RALPH_LOG_DIR:-$HOME/.cache/ralph-grind/plate-solver}"

# ---- arg parse: optional --detach to run the loop in a detached tmux session ----
DETACH=0
if [ "${1:-}" = "--detach" ]; then DETACH=1; shift; fi
MAX_ITERS="${1:-60}"

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
  tmux new-session -d -s ralph-grind "cd '$REPO' && '$REPO/tools/ralph-grind.sh' $MAX_ITERS"
  echo "✅ Detached grind loop in tmux session 'ralph-grind' (max $MAX_ITERS)."
  echo "   Survives killing any Claude session / closing this terminal."
  echo "   watch  : tmux attach -t ralph-grind        (then detach with: Ctrl-b, then d)"
  echo "   tail   : tail -f $LOG_DIR/iter-*.log"
  echo "   commits: git -C '$REPO' log --oneline"
  echo "   stop   : tmux kill-session -t ralph-grind"
  exit 0
fi

# One-iteration prompt. Forces exactly ONE task then exit (overriding grind's "iterate until
# done"), because the OUTER bash loop provides the iteration — each task gets a fresh context.
read -r -d '' PROMPT <<'EOF' || true
You are running ONE iteration of an autonomous implementation loop. The file plan.md in the
current working directory is the SINGLE SOURCE OF TRUTH (Purpose/DoD, Loop Protocol, Guardrails,
Tasks, and the Decisions/Blocked/Run logs).

Do EXACTLY ONE task this run, then STOP. Do not start a second task. Do not "keep going". The
process will exit and a fresh one will start for the next task. Concretely:

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

stamp() { date "+%Y-%m-%d %H:%M:%S"; }
done_count() { grep -c '^- \[x\]' plan.md 2>/dev/null || echo 0; }
open_count() { grep -c '^- \[ \]' plan.md 2>/dev/null || echo 0; }
commit_count() { git rev-list --count HEAD 2>/dev/null || echo 0; }

echo "=== ralph-grind start $(stamp) | model=$ORCH_MODEL | max=$MAX_ITERS | logs=$LOG_DIR ==="
echo "    tasks: $(done_count) done / $(open_count) open"

# ---- the loop ---------------------------------------------------------------
for ((i=1; i<=MAX_ITERS; i++)); do
  if [ "$(open_count)" -eq 0 ]; then
    echo "✅ All task checkboxes are [x]. Done at $(stamp)."
    break
  fi

  d_before=$(done_count); c_before=$(commit_count)
  log="$LOG_DIR/iter-$(printf '%03d' "$i").log"
  echo "── iteration $i  $(stamp)  ($d_before done) → $log"

  # Fresh process == fresh context. Headless, autonomous, Sonnet orchestrator.
  claude -p "$PROMPT" \
      --model "$ORCH_MODEL" \
      --dangerously-skip-permissions \
      2>&1 | tee "$log"

  d_after=$(done_count); c_after=$(commit_count)

  if grep -q 'NOTHING-TO-DO' "$log"; then
    echo "🟦 Orchestrator reported NOTHING-TO-DO (done or all remaining blocked). Stopping."
    break
  fi
  if [ "$c_after" -eq "$c_before" ] && [ "$d_after" -eq "$d_before" ]; then
    echo "🛑 No new commit and no newly-completed task this iteration — no forward progress."
    echo "   Likely a BLOCK that didn't commit, or the run errored. Inspect $log. Stopping."
    break
  fi
  echo "   advanced: done ${d_before}→${d_after}, commits ${c_before}→${c_after}"
done

echo "=== ralph-grind end $(stamp) | $(done_count) done / $(open_count) open ==="
echo "Recent commits:"; git log --oneline -5
