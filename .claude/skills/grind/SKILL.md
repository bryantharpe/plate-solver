---
name: grind
description: Execute a forge-style plan.md as an autonomous iterate-until-done loop — re-read the plan from disk, pick the first unblocked task, implement it, run the integrity gates, commit, append to the Run Log, and repeat until every task is done or blocked. Applies the plan's guardrails (git, gates, scope, cost, don't-stall, context hygiene) every iteration and survives session restarts because the plan file is the only durable state. Use when the user wants to run/continue/resume a plan loop, work through a plan.md / grind file / loop plan, execute a task list to completion, or grind a project autonomously. Pairs with the `forge` skill, which authors the plan this executes.
---

# grind — run a plan.md to completion

You execute a `plan.md` authored by the `forge` skill (structure spec in the `forge` skill's
bundled `STRUCTURE.md`). The plan file is the **only durable state** — treat your
conversation context as disposable. Everything you need to act, and to resume cold after a restart or
compaction, lives in that file. Your job is to walk its tasks to the definition of done while obeying
its guardrails on every step.

## Finding the plan

Use the path the user gave. Otherwise look for `plan.md` in the project root (or a `plans/` dir). If
several exist or none do, ask. Never guess between candidate plans.

## The iteration (one task per cycle)

Repeat this loop. Keep each iteration to **one task** so your per-iteration context footprint stays
bounded and any restart resumes cleanly.

1. **Re-read the plan from disk** — top to bottom, every iteration. Do not rely on what you remember
   from a previous cycle; the file is authoritative. Read the Purpose (stop condition), all Guardrails,
   the task list, and the Blocked/Decisions logs.
2. **Select the task** — the first task whose checkbox is unchecked AND whose `deps:` are all checked.
   That is the next task; selection is mechanical, not a judgment call.
3. **Implement it fully**, in-session, to its acceptance criteria. Apply every guardrail while you work.
   Offload heavy reading to subagents (one per file/unit) so raw source text lands in the subagent's
   context and only the distilled result returns to you; in the main thread use targeted Read/Grep, not
   whole-file dumps.
4. **Run the integrity gates** named in the guardrails before committing. If a gate fails, fix it — never
   weaken the gate. If you can't make it pass within this task, revert your changes, mark the task
   `BLOCKED` (step 6), and move on.
5. **Land it** — check the box `[x]`, append a one-line Run Log entry (`<date> — <ID> — <result> — next:
   <ID>`), and commit with the task's exact commit message. Follow the plan's push cadence. Commit the
   plan file together with the work so progress and state advance atomically.
6. **Stop / continue:**
   - All checkboxes `[x]` or `BLOCKED` → the loop is done. Write the final status report the plan names
     (or a short `STATUS.md`: definition-of-done result, what each task produced, every decision made,
     every blocked item with its recommended fix). Then **stop** — do not schedule more work.
   - Selected task can't proceed → mark it `BLOCKED` in the task list, append to the Blocked Log
     (`<ID> — reason — recommended fix`), and continue to the next unblocked task. The loop never spins
     on one task.
   - Otherwise → loop to step 1 for the next task.

## Autonomy and decisions

If the plan says it runs autonomously: when a genuine fork appears, **do not stall for a human** — pick
the most defensible default, record it in the Decisions Log (`<date> — decision — rationale`) and in the
commit message, and continue. Reserve stopping-to-ask for forks the plan explicitly flags as
human-only, or irreversible/destructive actions the guardrails don't already authorize. If the plan
says pause-at-forks, surface the decision and wait.

## Context hygiene (why this survives restarts)

Because the plan file holds goal + protocol + guardrails + task state + logs, harness compaction and
session restarts are **lossless by design**: a fresh session reconstructs full state by re-reading the
plan (= first unchecked task + all guardrails + the logs). So don't fight compaction — design each
iteration to survive it. Concretely: commit (and push) before ending any iteration so nothing important
lives only in context; checkpoint long tasks to disk as you go; keep one task per iteration. If you
notice context filling mid-task, flush what you have to disk and the plan's logs before continuing.

## Running unattended

A single `grind` invocation works through tasks until done, blocked-stalled, or you choose to checkpoint.
For fully unattended, restart-surviving runs, the user can wrap this in `/loop` (e.g. `/loop grind
plan.md`) — each wake re-reads the plan and advances it. Either way the contract is identical because the
plan file is the source of truth.

## Quality bar

- Never check a box whose acceptance criteria aren't actually met.
- Never weaken an integrity gate to make a commit; a failing gate blocks the commit.
- Never commit to a forbidden branch or force-push if the guardrails prohibit it.
- Honor data/scope discipline exactly — no out-of-scope edits, no leaks, no unauthorized destructive ops.
- Keep the logs honest and append-only: real results in the Run Log, real reasons in the Blocked Log,
  real rationale in the Decisions Log. The post-run reader trusts these.
