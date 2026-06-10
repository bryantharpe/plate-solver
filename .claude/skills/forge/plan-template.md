# <Project name>: <one-line objective>

_Authored <YYYY-MM-DD>. This file is the ONLY durable state for this loop. Every iteration re-reads it from disk; it must be self-sufficient for a cold restart._

## Purpose

<2–3 sentences. WHAT this project achieves, WHY it matters, and the observable DEFINITION OF DONE that halts the loop.>

## Loop Protocol

1. Re-read this file from disk (it is the only durable state).
2. Pick the first task whose checkbox is unchecked AND whose deps are all checked.
3. Implement it fully in-session.
4. Mark the checkbox `[x]`, append a Run Log entry, and commit with the task's commit message.
5. <Push / publish step — e.g. "push to origin/<branch>", or delete this line if not applicable.>
6. Stop condition: if all checkboxes are checked, write `<STATUS-REPORT>.md` and exit. If the next task is blocked, mark it BLOCKED and continue. Otherwise loop to step 1.

## Guardrails (apply every iteration)

- **Git:** branch is `<branch>`. Commit after every task with its specified message. <Push cadence.> Never commit to `main`. Never force-push.
- **Integrity gates:** run `<test/validate command>` before every commit; if it fails, fix before proceeding. Never weaken a gate to make it pass.
- **Data/scope discipline:** <what must never happen to data or scope — e.g. no leaking held-out data, no out-of-scope edits, no destructive ops without confirmation>.
- **Cost controls:** <bound expensive ops — prefer local/cheap work; cap spend; batch; no unbounded retries>.
- **Don't-stall:** if a task is blocked, mark it `BLOCKED`, append to the Blocked Log with a reason + recommended fix, and move to the next unblocked task.
- **Context hygiene:** this file is the only durable state. Flush progress to it before context fills (checkpoint after every task). Survive restarts by re-reading it. Record autonomous decisions in the Decisions Log and proceed on the best default rather than stalling.

## Tasks

### Phase A — <phase name>

- [ ] T1 — <Name> — deps: none — <body> — AC: <objective, checkable criteria> — commit: "<message>"
- [ ] T2 — <Name> — deps: T1 — <body> — AC: <objective, checkable criteria> — commit: "<message>"

### Phase B — <phase name>

- [ ] T3 — <Name> — deps: T2 — <body> — AC: <objective, checkable criteria> — commit: "<message>"

## Decisions Log

- (append-only) <YYYY-MM-DD> — <decision> — <rationale>

## Blocked Log

- (append-only) <task ID> — <reason> — <recommended fix>

## Run Log

- (append-only) <YYYY-MM-DD> — <task ID> — <result> — <what's next>
