# The `plan.md` structure (canonical spec)

This is the single source of truth for the file `forge` writes and `grind` executes. It is
domain-agnostic: the same shape drives a code migration, a research sweep, a writing project, an
infra rollout, or anything decomposable into dependency-ordered tasks.

The design goal is **durable, resumable, single-file state**. The plan file is the *only* thing that
survives a session restart or context compaction. Everything an executor needs to pick up cold —
what "done" means, how to behave, what's left, what was decided, what's blocked — lives in this one
file. No external memory, no chat history dependency.

---

## Top of file: title + provenance

```
# <Project name>: <one-line objective>

_Authored <YYYY-MM-DD>. This file is the ONLY durable state for this loop. Every iteration re-reads
it from disk; it must be self-sufficient for a cold restart._
```

The provenance line is not decoration — it tells any executor (including a fresh session) that this
file is authoritative and self-contained.

---

## 1. Purpose (2–3 sentences)

Three things, no more:
- **What** — the objective in one clause.
- **Why** — the reason it matters (anchors trade-offs when tasks conflict).
- **Definition of done** — the *observable* stop condition. Not "improve X" but "X passes Y", "report Z
  exists", "all checkboxes checked". This is the most important sentence in the file: the loop halts on
  it. If you can't state it observably, the project isn't ready to loop.

## 2. Loop Protocol (numbered)

The exact procedure an executor repeats. Keep it literally numbered so it survives paraphrase. The
canonical six steps (customize wording, keep the spine):

```
1. Re-read this file from disk (it is the only durable state).
2. Pick the first task whose checkbox is unchecked AND whose deps are all checked.
3. Implement it fully in-session.
4. Mark the checkbox [x], append a Run Log entry, and commit with the task's commit message.
5. <Push / publish step, if any.>
6. Stop condition: if all checkboxes are checked, write the final STATUS report and exit.
   If the next task is blocked, mark it BLOCKED and continue. Otherwise loop to step 1.
```

## 3. Guardrails (bullets — apply EVERY iteration)

Standing rules that hold regardless of which task is running. Six categories; every plan fills all six
(write "n/a" only with a reason). These are invariants, not suggestions.

- **Git rules** — which branch; commit cadence (commit-per-task is the default); push cadence; "never
  commit to main / never force-push" type prohibitions.
- **Integrity gates** — what must pass before a commit (tests, linters, schema validation, build). The
  iron rule: **never weaken a gate to make it pass.** A failing gate blocks the commit, not the gate.
- **Data / scope discipline** — what must NOT happen to data or scope: no leaking held-out/eval data,
  no editing out-of-scope files, no silent scope creep, no destructive ops without confirmation.
- **Cost controls** — bound expensive operations: prefer cheap/local work, cap API/compute spend, batch
  where possible, no unbounded retries.
- **Don't-stall rule** — if a task can't proceed, mark it `BLOCKED`, append to the Blocked Log with a
  reason + recommended fix, and move to the next unblocked task. The loop never spins on one task.
- **Context hygiene** — this file is the only durable state. Flush progress to it *before* context fills
  (checkpoint after every task, not at the end). Re-read it on restart. Never hold unflushed work only
  in context. When autonomous and a genuine decision arises, record it in the Decisions Log and proceed
  on the most defensible default rather than stalling for a human.

## 4. Tasks (dependency-ordered, grouped into phases)

The work, decomposed. Group into `### Phase N — <name>` blocks ordered by dependency. Within a phase,
each task is ONE checkbox row carrying six fields:

```
- [ ] <ID> — <Name> — deps: <ID, ID | none> — <body: what to do, 1–3 clauses> — AC: <objective, checkable acceptance criteria> — commit: "<commit message>"
```

Field rules:
- **ID** — short, stable, unique (`T1`, `A3`, `MIG-04`). Referenced by `deps:` and the logs.
- **Name** — a few words.
- **deps:** — IDs that must be `[x]` before this task is eligible, or `none`. This is what makes
  "pick the first unblocked task" well-defined. Order tasks so deps always point backward.
- **body** — what to actually do. Concrete enough to act on without re-asking.
- **AC (acceptance criteria)** — how the executor *knows* it's done, objectively. The #1 failure mode is
  vague AC ("works better"). Demand a check: a command that exits 0, a file that exists, a number that
  clears a threshold, a test that's green. If the AC isn't checkable, the task isn't ready.
- **commit** — the exact commit message to use when the task lands. Pre-writing it forces task atomicity:
  if you can't name one commit for it, the task is too big — split it.

Keep tasks **atomic** (one commit each) and **vertical** (each leaves the project in a working,
committed state). A task that can't be completed and committed in one sitting is too large.

## 5. Decisions Log (append-only)

```
## Decisions Log
- <YYYY-MM-DD> — <decision> — <rationale>
```

Every non-obvious choice made *in lieu of asking a human* — especially during autonomous runs. This is
what lets a human audit the run after the fact and what stops the executor from stalling on every fork.
Append-only; never rewrite history.

## 6. Blocked Log (append-only)

```
## Blocked Log
- <task ID> — <reason it's blocked> — <recommended fix / who can unblock>
```

Written by the don't-stall rule. A task marked `BLOCKED` in the task list gets a row here.

## 7. Run Log (append-only)

```
## Run Log
- <YYYY-MM-DD> — <task ID> — <result> — <what's next>
```

One line per completed (or blocked) iteration. This is the heartbeat — it lets a fresh session see what
just happened and what the next move is, without trusting memory.

---

## Invariants (why this shape works)

1. **Single durable state.** One file holds goal + procedure + rules + work + history. Lose the session,
   keep the file, keep the project.
2. **Observable done.** The stop condition is checkable, so the loop terminates rather than wandering.
3. **Unblocked-first selection + explicit deps.** "What do I do next" is mechanical, not a judgment call.
4. **Atomic, committed tasks.** Every iteration ends in a clean, working, version-controlled state — safe
   to stop or crash at any boundary.
5. **Gates that can't be softened.** Quality is non-negotiable per iteration; you can't trade correctness
   for progress.
6. **Don't-stall + Decisions Log.** The loop makes forward progress autonomously and leaves an audit trail
   instead of either freezing or making silent unrecorded choices.
7. **Append-only logs.** History is evidence; it is added to, never rewritten.
