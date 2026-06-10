---
name: grind-detach
description: Hand off the plate-solver grind loop to a detached, session-proof background runner so it keeps going after you kill/close the current Claude session or terminal. Use when the user wants the grind loop to continue unattended after this session ends. Launches tools/ralph-grind.sh (a fresh `claude -p` per task) inside a detached tmux session. Pairs with the ralph-grind.sh wrapper and the grind skill.
---

# grind-detach — hand the grind loop off to a session-proof runner

Make the plate-solver grind loop continue after the user kills this (or any) Claude session. The
loop itself is `tools/ralph-grind.sh` — one fresh `claude -p` process per task (so auto-compact
never fires mid-task; `plan.md` is the durable cold-restart state). This skill just launches it
inside a **detached tmux session**, which is independent of any Claude session or terminal.

REPO = `/Users/bryant/code/plate-solver`.

## Preconditions — verify BEFORE launching (do not skip)

1. **Clean checkpoint.** Run `git -C <REPO> status --porcelain` and `git -C <REPO> log --oneline -1`.
   There must be **no uncommitted *tracked* task work** — in particular `plan.md` must not be
   modified, and the last in-progress task must already be committed (its commit message visible
   in the log). Untracked helper files (`tools/`, `.claude/skills/`, `Cargo.toml` only if the
   creating task isn't committed yet) are a signal a task is mid-flight. If anything looks
   mid-task, STOP and tell the user to let the running loop finish and COMMIT its current task
   first — handing off mid-task risks orphaned, never-committed work.
2. **No competing loop.** Run `pgrep -fl 'claude --dangerously-skip-permissions'`. If a process is
   still running, the old interactive grind is alive — REFUSE and tell the user to stop it first
   (two loops collide on `plan.md`/git). The wrapper refuses too, but check here for a clear message.

## Launch

From the repo, run:

```
tools/ralph-grind.sh --detach [MAX_ITERATIONS]      # default 60
```

This relaunches the loop in a detached tmux session named `ralph-grind` and returns immediately.
(`tools/ralph-grind.sh --detach` can also be run directly in any terminal — no Claude session
needed at all, which is the most session-proof option.)

## Report to the user

Print exactly how to observe and control it:

- watch live : `tmux attach -t ralph-grind`   (leave it running: Ctrl-b then d)
- tail logs  : `tail -f ~/.cache/ralph-grind/plate-solver/iter-*.log`
- progress   : `git -C /Users/bryant/code/plate-solver log --oneline`
- stop       : `tmux kill-session -t ralph-grind`

Then confirm: the user may now kill this Claude session (and any other) — the loop continues in
tmux until the plan is done / all-blocked / no-progress / max iterations.
