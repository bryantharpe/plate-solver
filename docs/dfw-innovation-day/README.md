# DFW Innovation Day — Tokenomics session

A 10-minute, discussion-led session on AI cost optimization, anchored on this
repository as the worked example.

| File | What it is |
|------|------------|
| [`tokenomics-deck.html`](./tokenomics-deck.html) | The deck. Single self-contained file — open it in any browser, no build, no network. |
| [`talking-track.md`](./talking-track.md) | The 10-minute script, timing marks, Q&A backup, and the Monday rehearsal checklist. |

## Running the deck

Open `tokenomics-deck.html` in a browser. Everything is inlined; it works offline.

| Key | Action |
|-----|--------|
| `←` `→` `Space` | Navigate |
| `S` | Speaker notes (the talking track, per slide) |
| `T` / `R` | Start-pause / reset the 10-minute talk clock |
| `G` | Overview grid |
| `1`–`9` | Jump to slide |
| `?` | All shortcuts |

Print to PDF for a backup copy — each slide breaks onto its own page.

## The argument

The session does **not** cover token pricing mechanics. It argues that AI
economics are driven by architecture, using this repo as the evidence:

1. **The product is real** — a star-field plate solver that localizes a photograph
   of the night sky in 1.8 ms, benchmarked against an independent Python reference.
2. **No human wrote the implementation** — it was built by an agent fleet against
   written specs, with humans in the loop at exactly four points.
3. **It was built twice** — v1 (`v1-original`) and the spec-only rebuild (`main`),
   which is a natural A/B on cost.
4. **Three decisions moved the bill** — route work by size (small → local model,
   large → cloud relay), keep the author and reviewer on different model lineages,
   and treat the frontier tier as a budget rather than a default.
5. **Twelve deterministic gates cost zero tokens** — and in an end-to-end audit of
   one merged change, ten of the twelve turned out to be hygiene while three
   carried the actual weight.
6. **The cheap reviewer earned its keep, measurably** — across 29 merged changes
   the independent judge ran 56 review rounds and sent 8 back, and all 8 were
   correctness bugs rather than style. Roughly 10 further attempts were abandoned
   before ever reaching a verdict, so the deterministic gates and the integration
   step killed more work than the reviewer did.

## Two things to finish before presenting

**1 · Spend figures (slide 8).** Every cost number is a marked placeholder — the Grafana LLM
consumption data was not available when the deck was built. Search the HTML for
`is-pending` to find them. They render with a visible `PENDING` chip and dashed
borders specifically so an unfilled figure cannot be mistaken for a real one.

**2 · Brand palette.** `slalom.com` was unreachable from the build environment
(blocked by egress policy), so the colors are brand-adjacent rather than sampled.
Every color in the deck derives from the token block at the top of the HTML —
correcting the palette is a single-block edit.

## Provenance

Every non-placeholder figure in the deck is reproducible from this repository at
the `v1-original` tag and `main`, or from the pull-request record on GitHub. Slide 13 lists each claim with the command or
file it came from.
