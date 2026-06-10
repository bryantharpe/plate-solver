---
name: forge
description: Interview the user to produce a structured, self-contained plan.md that an autonomous loop can later execute to completion. Grills them branch-by-branch into a Purpose + definition-of-done, a numbered Loop Protocol, six categories of Guardrails, dependency-ordered Tasks with checkable acceptance criteria, and append-only Decisions/Blocked/Run logs. Use when the user wants to plan a multi-step project as a resumable loop, create a "plan file" / "grind file" / "loop plan", turn a goal into an executable task list, or set up work that survives session restarts. Pairs with the `grind` skill, which executes the plan forge produces.
---

# forge — author an executable plan.md

You turn a fuzzy goal into a **single, self-contained `plan.md`** that the `grind` skill (or any
executor, including a fresh session) can run to completion without you in the room. The plan file is
the *only* durable state — so 95% of the value here is getting its structure right and its content
unambiguous. A vague plan produces a stalled loop; a sharp plan runs itself.

Read `STRUCTURE.md` (bundled next to this file) — it is the canonical spec for every section. Use
`plan-template.md` (also bundled) as the skeleton you fill in. This file is the *interview protocol*
that gets you the content to fill it.

## How to grill

Interview relentlessly, **one question at a time**, walking down the decision tree and resolving
dependencies between answers as you go (the `grill-me` discipline). For every question, lead with your
**recommended answer** and a one-line why; let the user accept or redirect. If a question can be
answered by reading the codebase/files instead of asking, go read — don't make the user tell you what
you can find. Use `AskUserQuestion` for genuine forks with discrete options; use plain prose questions
for open ones.

Do not write the file until the load-bearing sections are unambiguous. The two things you must not let
stay fuzzy: the **definition of done** and each task's **acceptance criteria**. Everything else can
take a sensible default; these two cannot.

### Order of the interview

Work top-down through the structure, because later sections depend on earlier ones.

1. **Purpose + definition of done (do this first, hardest).** Pin the observable stop condition before
   anything else. Push until it's *checkable* — "X test passes", "report Y exists", "all boxes checked",
   "metric ≥ threshold" — not "improve X" or "make it better". If the user can't state done observably,
   the project isn't ready to loop yet; help them make it observable or narrow scope until it is. Capture
   what + why + done in 2–3 sentences.

2. **Decompose into tasks.** Get the work as a list, then:
   - Order by dependency (each task's deps point backward to earlier IDs).
   - Group into phases (`### Phase N — name`).
   - Force **atomicity**: each task must be completable and committable in one sitting. If a task can't be
     named by a single commit message, split it. Tracer-bullet / vertical slices beat horizontal layers —
     each task should leave the project working and committed.
   - For each task, nail the six fields (ID, name, deps, body, AC, commit). The AC is where you grill
     hardest — demand an objective check. The commit message is the atomicity test.

3. **Guardrails (fill all six categories).** Start from defaults and have the user confirm/adjust:
   - **Git** — branch (default: a feature branch, never `main`); commit-per-task; push cadence; no force-push.
   - **Integrity gates** — the exact test/validate/build command that must pass before each commit. State
     the iron rule explicitly: never weaken a gate to make it pass.
   - **Data/scope discipline** — what must never happen to data or scope (leaks, out-of-scope edits,
     destructive ops without confirmation, silent scope creep).
   - **Cost controls** — bound anything expensive (API/compute spend, prefer local/cheap, batch, no
     unbounded retries).
   - **Don't-stall** — blocked task → mark `BLOCKED`, log it, move to the next unblocked task.
   - **Context hygiene** — plan.md is the only durable state; checkpoint to disk after every task;
     re-read on restart; delegate heavy reading to subagents so raw source stays out of the main loop;
     one task per iteration to bound footprint; record autonomous decisions and proceed on best default.
   Ask: "Will this run autonomously (proceed on best judgment, log decisions) or pause for approval at
   forks?" — this sets the tone of the context-hygiene + decisions guardrails.

4. **Loop Protocol.** Usually the canonical six steps from the template; only customize if this project
   needs a different cadence (e.g. a convergence loop that stops after N no-improvement rounds — see
   below). Confirm the stop condition wording matches the definition of done from step 1.

5. **Seed the logs.** Leave Decisions/Blocked empty (append-only); seed the Run Log with one line:
   `<date> — plan authored — <N> tasks queued; loop starting at <first ID>`.

### Convergence loops (a common variant)

Some goals aren't a fixed task list but "iterate until quality bar met" (tuning, refactoring to a
metric, eval-driven iteration). For these, the final task is itself a bounded loop: *"While <metric>
below <bar> AND improving over the last 2 rounds: diagnose → smallest fix → re-run verification →
re-score. Stop after: bar reached, OR 2 consecutive rounds with no improvement, OR N rounds total."*
Always give it a hard round cap and a no-improvement cutoff so it can't spin forever. (This is exactly
how the reference plan drove an eval subject to parity.)

## Writing the file

- Default location: `plan.md` in the project root, unless the user names another path or a `plans/`
  convention exists. Ask if ambiguous.
- Fill `plan-template.md` with the interview results. Keep task rows on one line each in the six-field
  format. Prefer load-bearing sentences over bulk — the executor re-reads this every iteration.
- After writing, show the user the task list + definition of done and the guardrails, and confirm. Make
  any final edits.
- Tell them how to run it: invoke the **`grind`** skill on this plan (and that wrapping it in `/loop`
  gives unattended, restart-surviving autonomy).

## Quality bar for the plan you produce

- Definition of done is observable and matches the loop's stop condition.
- Every task has objective, checkable acceptance criteria and a single commit message.
- Deps point backward; "the first unblocked task" is always unambiguous.
- All six guardrail categories are filled (or explicitly "n/a — reason").
- The file is self-sufficient: a cold session reading only this file knows the goal, the rules, what's
  done, what's next, and when to stop.
