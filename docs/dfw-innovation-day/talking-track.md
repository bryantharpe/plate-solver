# Tokenomics — 10-minute talking track

**DFW Innovation Day · Bryan Tharpe & Kiran**

Deck: [`tokenomics-deck.html`](./tokenomics-deck.html) — open it in a browser.
The same notes below are built into the deck: press **S** for the speaker-notes
panel, **T** to start the talk clock.

**Core message:** this is not about the price of tokens. It's about how
architecture and model routing change AI economics.

---

## Before you start

| | |
|---|---|
| **Format** | Discussion-led. The deck is scaffolding, not the point. Slides 2 and 8 are demo moments. |
| **Total** | 10:00, then open discussion |
| **Clock** | Press `T` in the deck. It turns amber at 8:30 and red past 10:00. |
| **Two live moments** | The plate-solver web UI (slide 2) and the Grafana board (slide 8). Both have screenshot fallbacks — if a build fails at 9am, nothing breaks. |
| **Must fill in first** | The four spend tiles on slide 8. Search the HTML for `is-pending`. They are visibly marked PENDING so nothing fake can be shown as real. |

**Timing marks** — glance at these, don't obsess:

| At | Slide | Budget |
|----|-------|--------|
| 0:00 | 1 · Title | 30s |
| 0:30 | 2 · What it does | 80s |
| 1:50 | 3 · Nobody wrote it | 50s |
| 2:40 | 4 · Built twice | 85s |
| 4:05 | 5 · Route by size | 85s |
| 5:30 | 6 · The cheapest reviewer | 50s |
| 6:20 | 7 · What the judge caught | 70s |
| 7:30 | 8 · What it cost | 80s |
| 8:50 | 9 · Three moves | 50s |
| 9:40 | 10 · Discussion | 20s |

Slides 11–13 are Q&A backup and are not timed.

---

## 1 · Title — 0:00 (30s)

Kiran and I are **not** going to talk about the price of tokens. Everybody has
seen the price-per-million table, and it is the least interesting number in this
conversation.

What we want to show you is a system we built, what it cost to build, and the two
or three **architectural** decisions that moved that cost by an order of magnitude.

Ten minutes. Mostly demo. Then let's argue about it.

---

## 2 · What it does — 0:30 (80s)

> **SWITCH TO LIVE?** If the app is up, drive it live: drop the image in, set FOV
> to `11`, hit Solve. Otherwise the screenshots are the real output and nobody
> will know the difference.

This is a plate solver. You hand it a photograph of the night sky — no GPS, no
timestamp, no metadata, nothing — and it tells you exactly where the camera was
pointed. It's how a telescope figures out where it is, and it's the same class of
problem as "lost in space" navigation for a satellite.

*[Point at the numbers.]* Right Ascension 230.67 degrees, Declination 11.04. It
matched **47 catalog stars** against a database of a million patterns. It did that
in **1.8 milliseconds**.

*[Point at the overlay.]* Every green ring is a real star it identified by
Hipparcos catalog ID.

That's a real product — Rust, gRPC service, web UI, runs on a Raspberry Pi. It
benchmarks about one and a half times faster than the Python reference it's cloned
from, and six and a half times faster than the original.

Hold that in your head, because here's the actual point.

---

## 3 · Nobody wrote it — 1:50 (50s)

I didn't write it. **Nobody** wrote it.

This was built by a fleet of AI agents running against written specifications.
There's a dispatcher that sizes the work, ephemeral workers that each take one
task, and an integration process that opens the pull requests.

Humans touch it in exactly four places: approving what gets built, signing off on
the highest-risk paths, handling escalations, and cutting releases.

So the question stops being "can AI write code." You're looking at the code. It
works, it's fast, and it passes parity against an independent reference
implementation.

The question becomes: **what did that cost, and what would make it cost less?**

---

## 4 · Built twice — 2:40 (85s)

Here's why I can answer that with something better than a vibe. We built this
system **twice**.

Version one: about five weeks, 144 commits, seven crates, roughly fourteen
thousand six hundred lines of Rust. Mixed — me, Claude, some agents. And **no
CI**. The quality gate was me running `cargo test`.

Then we deleted the implementation. Kept the specs, kept the reference oracle,
threw away all the code — and every artifact that described *how* version one had
been built, including our own architecture notes and task breakdown. The rule was:
the rebuild has to be derivable from the spec alone.

Version two: **ten days**, 54 commits, six crates, under ten thousand lines.
Roughly a third of the calendar time, two-thirds of the code, same capability,
same parity tests.

> **Be honest here — this lands better if you volunteer the caveat.**

Some of that delta is "the second time you build anything is faster." That's real
and I'm not going to pretend otherwise. But note the direction of the verification
story: version one had *zero* CI. Version two **cannot merge** without ten
required checks and an independent model review. Less time, less code, more proof.

---

## 5 · Route by size — 4:05 (85s)

**This is the slide that actually matters.** If they remember one, it's this one.

Nothing in the fleet asks "which model is best." It asks: **what is the cheapest
thing that can do this job, and be caught if it's wrong?**

**One — route by size.** The dispatcher sizes every unit of work before it assigns
it. Small work goes to a local model on our own hardware — marginal token cost is
zero, we're paying for electricity. Only large work goes out to a cloud relay. And
most work is small.

**Two — the author never grades its own homework.** The code is written by Kimi
models. The review is done by GLM, a completely different model lineage. That's a
correctness decision first, but it has a cost consequence: a cheap independent
reviewer catches things you'd otherwise pay a frontier model to catch.

**Three — the expensive tier is a budget, not a default.** Opus gets spent where
judgment genuinely decides the outcome: adversarial review and process audits.
Deliberately, not by reflex.

That's the whole architecture. Route by size. Separate author from reviewer. Spend
the expensive tier on purpose.

---

## 6 · The cheapest reviewer — 5:30 (50s)

There's a fourth thing, and it's free.

Before *any* model reviews anything, twelve mechanized gates run: formatting,
linting with warnings as errors, tests with a coverage floor, docs, minimum Rust
version, unused dependencies, semantic versioning, four license and vulnerability
scans, secret scanning, and a parity test against the Python reference.

Those cost **zero tokens. Forever.** Every defect a compiler catches is a defect
you don't pay a model to find — and more importantly, one you don't pay a model to
*argue with you about*.

We audited one merged change end to end. Twelve green checkmarks. **Ten of them
were hygiene.** Three carried the actual weight. And one of those three had to be
run by hand.

That's worth knowing. Green isn't proof — and knowing which gates are load-bearing
tells you where model spend is buying you something real.

The general principle: push verification as far down the cost curve as it will go.
Deterministic before probabilistic. Cheap model before expensive model. Model
before human.

---

## 7 · What the judge caught — 6:20 (70s)

Fair question at this point: did the cheap reviewer actually earn its keep, or is
it theatre? We can answer that, because every verdict it ever issued is on the
pull requests.

**56 review rounds across 29 merged changes. It sent 8 back.** That's 28% of
changes, 14% of rounds.

> **Don't read all eight aloud — pick two.**

The two I'd pick: a **dimensional error** that computed field of view about 850
times wrong, and an **off-by-one in a floating-point exponent** that decoded every
half-precision subnormal at half its value.

The point is **not one of the eight was style.** No naming, no formatting — eight
for eight were correctness. These are exactly the bugs a human reviewer skims past
at four in the afternoon.

**Now the number that's bigger than the judge.** Roughly ten more attempts were
abandoned before they ever got a verdict — one task burned six pull requests
before one of them landed. So the deterministic gates and the integration step
killed *more* work than the reviewer did. That's the tier-0 argument from the last
slide, paying off, measured.

> **Volunteer the caveat — someone will ask.**

On one change the judge returned APPROVE and DISSENT on the **same commit**, 46
seconds apart. It is not deterministic. On another it returned nothing 27 times
running and a human had to sign off. So "8" is a sample, not a constant — and
that's an argument for cheap reviewers being *redundant*, not for trusting one.

### The eight, for reference

| PR | What it caught |
|----|----------------|
| #54 | Infinite loop when the hash table fills — a denial of service |
| #55 | Dimensional error — field of view computed ~850× wrong |
| #66 | Assert too tight (1e-12) — would panic on real catalog data |
| #73 | Off-by-one exponent — every f16 subnormal decoded at half value |
| #76 | Search-radius regression — silently returns nothing past a cutoff |
| #77 | Wrong variable returned — search bound used as the FOV estimate |
| #79 | Dead loop — iterated over keys it never passed to the database |
| #84 | Unit heuristic breaks — a 2° field reported as 114.6° |

---

## 8 · What it cost — 7:30 (80s)

> **SWITCH TO GRAFANA.** This is the part to drive live if the board is up.

You can't optimize what you can't see, so we instrumented it.

*[Walk the panels.]* Tokens by model. Spend by seat. Cost per merged change.

Two things I'd point at.

**First, the distribution.** Most of the *volume* is on the cheap tier — but the
*spend* concentrates in a small number of expensive calls. Those are the ones
worth scrutinising, and you cannot find them without this board.

**Second, the trend.** Cost per merged change, over time. That is the number I
actually manage.

Not price per million tokens — **cost per unit of delivered work**. Those are very
different metrics, and only one of them is a business metric.

---

## 9 · Three moves — 8:50 (50s)

If you take three things back to your team:

**Route by size.** Not everything needs the frontier model, and most things don't.
Classify the work before you dispatch it — a router is cheaper than a better model.

**Verify deterministically first.** Every check a compiler, a linter, a schema or
a test can perform is a check you never pay for again. Find your parity oracle:
the external thing that proves the output is right without asking a model.

**Instrument per unit of delivered work.** Cost per merged change, cost per
resolved ticket, cost per document processed. If your dashboard shows tokens,
you're measuring the input. Measure the output.

None of this is about the price of tokens. All of it is about architecture.

---

## 10 · Discussion — 9:40 (20s)

Three questions we'd genuinely like to argue about:

1. Where are you paying frontier prices for work a small model could do?
2. What's your parity test — the deterministic check that proves the output is
   right without asking a model?
3. Do you know your cost per unit of delivered work? Not your token bill.

> **Read them, then stop talking.** Let the room fill the silence. If nobody bites,
> go to the middle one — the parity-test question is the one people have the
> strongest opinions about.

---

## Q&A backup

### "What goes wrong?" → slide 11

- **Runaway context burn.** One worker ran 72 minutes and 1,410 commands, consumed
  100% of its context window, and produced **zero lines of code**. Auto-compaction
  wasn't firing on that seat, so an agent that started reading never stopped.
  *Fix: an explicit context budget in every task brief. Cost control is a
  prompt-level control, not just a routing one.*
- **The answer key problem.** A worker restored an earlier implementation
  byte-for-byte in 90 seconds — and **every gate approved it**. Cheap, fast, and
  worthless as a measurement. *Fix: an anti-recovery clause in the brief, and the
  reason we deleted our own architecture notes before the rebuild.*

Both clauses in every task brief today were bought with a failed run. Your first
spend on an agent fleet is on learning which failure modes cost money.

### "How do you trust code no human wrote?" → slide 12

You don't trust the author. You make the author's output falsifiable by something
it cannot influence. The vendored Python reference was not produced by the code
under test, so agreement with it is external validation, not self-certification:
RA/Dec within a few arcseconds, centroids within ±0.1 px, identical catalog IDs,
90% coverage floor on the numerical core.

### "Where did that number come from?" → slide 13

Every figure with the command or file it came from. All reproducible from this
repo at `v1-original` and `main`. The one row marked PENDING is the Grafana spend
data — the only number in the deck that isn't derivable from the repository.

### Likely pushback, and honest answers

**"Isn't the second build always faster?"** — Yes, partly. Volunteer this before
they raise it (it's in the slide-4 script). The defensible claim isn't "agents made
it 3× faster"; it's that the *shape of the spend* changed and the verification got
stronger at the same time.

**"Cheap models write bad code."** — Sometimes. That's what the gate ladder and the
independent reviewer are for. The bet isn't that the cheap model is as good; it's
that cheap model + deterministic verification beats expensive model alone, per
dollar.

**"What about the local hardware cost?"** — Real, and it's capex against a variable
bill. Worth saying plainly rather than claiming local is free.

**"Does this work outside code?"** — The routing and verification pattern
generalizes; the parity oracle is the hard part. Ask them what their oracle would
be — that's the discussion.

---

## Rehearsal checklist — Monday morning

- [ ] Fill the four spend tiles on slide 8 from Grafana (search `is-pending`)
- [ ] Drop a Grafana panel screenshot into the slot on slide 8, or confirm the
      live board loads
- [ ] Decide live vs. screenshots for slide 2 and, if live, pre-warm the solver
- [ ] Swap the brand tokens if the palette is off (top of the HTML, one block —
      `slalom.com` was unreachable from the build environment, so the colors are
      brand-adjacent, not sampled)
- [ ] Run it once end-to-end with the clock (`T`) — the target is landing on slide
      10 at 9:40
- [ ] Agree who takes which slides, and who fields which pushback
