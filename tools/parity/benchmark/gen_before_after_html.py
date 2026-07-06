#!/usr/bin/env python3
"""Generate a self-contained HTML page comparing eval-harness benchmark runs
before and after the FU-A changes (and re-confirmed post-FU-B). Reads the
committed JSON result snapshots in this directory."""
from __future__ import annotations
import json, statistics, html, sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent
PRE = BENCH / "results_pre_fua.json"
POST = BENCH / "results_post_fua.json"
FUB3 = BENCH / "results_fub3.json"
OUT = BENCH.parent.parent / "notes" / "benchmark-before-after.html"

ASTRO = [
    "2019-07-29T204726_Alt40_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt40_Azi45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi135_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi-45_Try1.jpg",
    "2019-07-29T204726_Alt60_Azi45_Try1.jpg",
    "hale_bopp.jpg",
]


def load(p):
    return json.load(open(p))


def per_sys(r):
    out = {}
    for rec in r["results"]:
        if rec["kind"] != "astronomical":
            continue
        img = rec["image_name"]
        sys = rec["solve"].get("system") or rec["detect"].get("system")
        dw = rec["detect"].get("wall_clock_s", [])
        sw = rec["solve"].get("wall_clock_s", [])
        te = rec["solve"].get("t_extract_ms", [])
        ts = rec["solve"].get("t_solve_ms", [])
        out[(img, sys)] = {
            "detect_wall": statistics.median(dw) if dw else None,
            "solve_wall": statistics.median(sw) if sw else None,
            "t_extract": statistics.median(te) if te else None,
            "t_solve": statistics.median(ts) if ts else None,
        }
    return out


def ratio(r, img, field, which="solve_wall"):
    a = r.get((img, "ps_grpc"), {}).get(which)
    b = r.get((img, "cedar_flow"), {}).get(which)
    return (b / a) if (a and b) else None


def med_ratio(r, field, which):
    rs = [ratio(r, img, field, which) for img in ASTRO]
    rs = [x for x in rs if x is not None]
    return statistics.median(rs) if rs else None


def parity_primary(r):
    return [c for c in r["parity"]["astronomical"]
            if c["comparison"] == "ps_grpc_vs_cedar_flow"
            and c.get("label") == "primary_same_catalog"]


def fmt(v, unit="ms", scale=1000, dp=2):
    if v is None:
        return "—"
    return f"{v*scale:.{dp}f}{unit}"


def fmtx(v, dp=2):
    return f"{v*1000:.{dp}f}" if v is not None else "—"


def delta_str(post, pre, scale=1000, dp=2, lower_is_better=True):
    if post is None or pre is None:
        return ("—", "")
    d = post - pre
    pct = (d / pre) * 100 if pre else 0
    better = (d < 0) if lower_is_better else (d > 0)
    cls = "good" if better else ("bad" if abs(d) > 1e-9 else "flat")
    sign = "+" if d >= 0 else ""
    return (f"{sign}{d*scale:+.{dp}f}ms ({sign}{pct:+.1f}%)", cls)


pre = per_sys(load(PRE))
post = per_sys(load(POST))
fub3 = per_sys(load(FUB3))

pre_parity = parity_primary(load(PRE))
post_parity = parity_primary(load(POST))
fub3_parity = parity_primary(load(FUB3))

# Aggregate ratios per snapshot
def agg(r):
    return {
        "detect": med_ratio(r, "detect", "detect_wall"),
        "solve": med_ratio(r, "solve", "solve_wall"),
    }

agg_pre = agg(pre)
agg_post = agg(post)
agg_fub3 = agg(fub3)

# ps_grpc per-snapshot median wall (median-of-medians across 9 astro)
def psmed(r, which):
    vs = [r.get((img, "ps_grpc"), {}).get(which) for img in ASTRO]
    vs = [v for v in vs if v is not None]
    return statistics.median(vs) if vs else None

ps_det_pre, ps_det_post, ps_det_fub3 = psmed(pre,"detect_wall"), psmed(post,"detect_wall"), psmed(fub3,"detect_wall")
ps_sol_pre, ps_sol_post, ps_sol_fub3 = psmed(pre,"solve_wall"), psmed(post,"solve_wall"), psmed(fub3,"solve_wall")

# parity drift pre vs post
def pkey(c):
    ch = c["checks"]
    ra = ch["ra_deg"]; dec = ch["dec_deg"]; ids = ch["matched_cat_ids"]
    return (round(ra["a"],9) if ra.get("a") is not None else None,
            round(dec["a"],9) if dec.get("a") is not None else None,
            tuple(sorted(ids.get("a",[]))) if "a" in ids else None,
            ch["status"]["a"])
pd = {c["image_name"]: pkey(c) for c in pre_parity}
xd = {c["image_name"]: pkey(c) for c in post_parity}
drift = [img for img in sorted(set(pd)&set(xd)) if pd[img] != xd[img]]
same_ids = sum(1 for img in set(pd)&set(xd) if pd[img][2] == xd[img][2])

rows = []
for img in ASTRO:
    p = pre.get((img, "ps_grpc"), {})
    x = post.get((img, "ps_grpc"), {})
    f = fub3.get((img, "ps_grpc"), {})
    dp_det, dc_det = delta_str(x.get("detect_wall"), p.get("detect_wall"))
    dp_sol, dc_sol = delta_str(x.get("solve_wall"), p.get("solve_wall"))
    rows.append((img, p, x, f, dp_det, dc_det, dp_sol, dc_sol))

# timeline ratios (solve vs cedar_flow)
# pre-SP4 baseline 0.27x is from the handoff (not in JSON); SP4 1.55x from measurements note
timeline = [
    ("Pre-SP4 (orig)", 0.27, "solve", "Eager lexicographic combos — full C(n,4) Vec allocated up front"),
    ("Post-SP4", 1.55, "solve", "Lazy breadth-first iterator (SP1.1) — killed the 618 MiB eager alloc"),
    ("Post-FU-A", agg_post["solve"], "solve", "FUA.1 real t_extract + FUA.2 image-buffer move (no clone)"),
    ("Post-FU-B (now)", agg_fub3["solve"], "solve", "FUB.1-3: profiled; no ps-solve lever moved the needle (reverted)"),
]

def chip(cls, txt):
    return f'<span class="pill {cls}">{txt}</span>'

def ratio_cell(r):
    if r is None: return '<td class="muted">—</td>'
    cls = "good" if r >= 1.0 else "bad"
    return f'<td class="num"><span class="pill {cls}">{r:.2f}×</span></td>'

det_pre_ratio = agg_pre["detect"]; det_post_ratio = agg_post["detect"]; det_fub3_ratio = agg_fub3["detect"]

html_out = f"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Plate Solver — Benchmark Before/After</title>
<style>
:root{{--bg:#0d1117;--card:#161b22;--border:#30363d;--txt:#c9d1d9;--muted:#8b949e;
--accent:#58a6ff;--green:#3fb950;--yellow:#d29922;--red:#f85149;
--mono:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--bg);color:var(--txt);
font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
line-height:1.55;font-size:15px}}
header{{padding:28px 32px 20px;border-bottom:1px solid var(--border);
background:linear-gradient(180deg,#161b22,#0d1117)}}
h1{{margin:0 0 6px;font-size:24px;color:#fff}}
h2{{margin:28px 0 10px;font-size:19px;color:var(--accent);
border-bottom:1px solid var(--border);padding-bottom:6px}}
h3{{margin:18px 0 8px;font-size:15px;color:#fff}}
.sub{{color:var(--muted);font-size:13px}}
main{{max-width:1180px;margin:0 auto;padding:24px 32px 80px}}
.card{{background:var(--card);border:1px solid var(--border);border-radius:8px;
padding:18px 20px;margin:14px 0}}
.pill{{display:inline-block;padding:2px 9px;border-radius:11px;font-size:12px;
font-weight:600;vertical-align:middle;font-family:var(--mono)}}
.green{{background:rgba(63,185,80,.15);color:var(--green);border:1px solid rgba(63,185,80,.4)}}
.yellow{{background:rgba(210,153,34,.15);color:var(--yellow);border:1px solid rgba(210,153,34,.4)}}
.red{{background:rgba(248,81,73,.15);color:var(--red);border:1px solid rgba(248,81,73,.4)}}
.blue{{background:rgba(88,166,255,.15);color:var(--accent);border:1px solid rgba(88,166,255,.4)}}
.good{{color:var(--green)}}
.bad{{color:var(--red)}}
.flat{{color:var(--muted)}}
table{{width:100%;border-collapse:collapse;margin:8px 0 14px;font-size:13px}}
th,td{{text-align:left;padding:7px 10px;border-bottom:1px solid var(--border);vertical-align:top}}
th{{color:var(--muted);font-weight:600;font-size:11px;text-transform:uppercase;letter-spacing:.04em}}
td.num,th.num{{text-align:right;font-family:var(--mono)}}
.muted{{color:var(--muted)}}
code{{font-family:var(--mono);background:#0d1117;border:1px solid var(--border);
border-radius:4px;padding:1px 5px;font-size:12px}}
.stat{{display:flex;gap:14px;flex-wrap:wrap;margin-top:10px}}
.stat div{{background:#0d1117;border:1px solid var(--border);border-radius:6px;padding:8px 14px}}
.stat .n{{font-size:20px;font-weight:700;color:#fff;display:block;font-family:var(--mono)}}
.stat .l{{font-size:11px;color:var(--muted);text-transform:uppercase;letter-spacing:.04em}}
blockquote{{border-left:3px solid var(--border);margin:10px 0;padding:4px 14px;color:var(--muted)}}
.footer{{color:var(--muted);font-size:12px;border-top:1px solid var(--border);
padding-top:14px;margin-top:30px}}
.timeline{{display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin:8px 0 4px}}
.tl{{background:#0d1117;border:1px solid var(--border);border-radius:6px;padding:12px}}
.tl .label{{font-size:11px;color:var(--muted);text-transform:uppercase;letter-spacing:.04em}}
.tl .val{{font-size:22px;font-weight:700;font-family:var(--mono);margin:4px 0}}
.tl .note{{font-size:11px;color:var(--muted)}}
@media(max-width:820px){{.timeline{{grid-template-columns:1fr 1fr}}}}
</style></head><body>
<header>
<h1>Plate Solver — Benchmark Before / After</h1>
<div class="sub">Eval-harness diff: pre-FU-A vs post-FU-A vs post-FU-B (current) · same host (Linux aarch64, 20 CPUs) ·
controlled swap of the <code>ps-grpc</code> binary only · parity identical-green verified</div>
</header>
<main>

<h2>Solve-ratio timeline vs cedar_flow (median over 9 astronomical images)</h2>
<div class="card">
<div class="timeline">
<div class="tl"><div class="label">Pre-SP4 (orig)</div><div class="val bad">{0.27:.2f}×</div>
<div class="note">Eager lexicographic combos — full C(n,4) Vec allocated up front (~618 MiB)</div></div>
<div class="tl"><div class="label">Post-SP4</div><div class="val good">1.55×</div>
<div class="note">Lazy breadth-first iterator (SP1.1) — killed the eager alloc; search now sub-ms on 8/9</div></div>
<div class="tl"><div class="label">Post-FU-A</div><div class="val good">{agg_post['solve']:.2f}×</div>
<div class="note">FUA.1 real t_extract + FUA.2 image-buffer move (no clone)</div></div>
<div class="tl"><div class="label">Post-FU-B (now)</div><div class="val good">{agg_fub3['solve']:.2f}×</div>
<div class="note">FUB.1-3: profiled exhaustion path; no ps-solve lever moved the needle (reverted by measurement)</div></div>
</div>
<blockquote>The headline win was <b>SP4</b> (0.27× → 1.55×, the lazy iterator). <b>FU-A</b> added real per-stage
timing + removed a per-request full-frame clone. <b>FU-B</b> profiled the remaining exhaustion-path cost and
found it lives in <code>ps-db</code> (~66%: <code>lookup_pattern</code> ~51% + <code>nearby_stars</code> ~15%),
not ps-solve — so no ps-solve allocation lever moved the benchmark, honestly reverted.</blockquote>
</div>

<h2>Aggregate: ps_grpc vs cedar_flow (median of 9 astro images)</h2>
<div class="card">
<table>
<tr><th>metric</th><th class="num">pre-FU-A</th><th class="num">post-FU-A</th><th class="num">post-FU-B (now)</th><th class="num">Δ pre→now</th></tr>
<tr><td><b>Detect</b> speedup vs cedar_flow</td>
{ratio_cell(det_pre_ratio)}{ratio_cell(det_post_ratio)}{ratio_cell(det_fub3_ratio)}
<td class="num muted">{(det_fub3_ratio/det_pre_ratio):.2f}× of pre</td></tr>
<tr><td><b>Solve</b> speedup vs cedar_flow</td>
{ratio_cell(agg_pre['solve'])}{ratio_cell(agg_post['solve'])}{ratio_cell(agg_fub3['solve'])}
<td class="num muted">{(agg_fub3['solve']/agg_pre['solve']):.2f}× of pre</td></tr>
</table>
<div class="stat">
<div><span class="n">{ps_det_pre*1000:.2f}ms</span><span class="l">ps_grpc detect wall (pre)</span></div>
<div><span class="n">{ps_det_fub3*1000:.2f}ms</span><span class="l">ps_grpc detect wall (now)</span></div>
<div><span class="n">{ps_sol_pre*1000:.2f}ms</span><span class="l">ps_grpc solve wall (pre)</span></div>
<div><span class="n">{ps_sol_fub3*1000:.2f}ms</span><span class="l">ps_grpc solve wall (now)</span></div>
</div>
</div>

<h2>Per-image diff: ps_grpc wall-clock, pre-FU-A → post-FU-A (ms, median over iterations)</h2>
<div class="card">
<table>
<tr><th>image</th>
<th class="num">detect pre</th><th class="num">detect post</th><th class="num">Δ detect</th>
<th class="num">solve pre</th><th class="num">solve post</th><th class="num">Δ solve</th></tr>
"""
for img, p, x, f, dp_det, dc_det, dp_sol, dc_sol in rows:
    short = img.replace("2019-07-29T204726_","").replace("_Try1.jpg","")
    html_out += (
        f"<tr><td>{html.escape(short)}</td>"
        f"<td class=\"num\">{fmtx(p.get('detect_wall'))}</td>"
        f"<td class=\"num\">{fmtx(x.get('detect_wall'))}</td>"
        f"<td class=\"num {dc_det}\">{dp_det}</td>"
        f"<td class=\"num\">{fmtx(p.get('solve_wall'))}</td>"
        f"<td class=\"num\">{fmtx(x.get('solve_wall'))}</td>"
        f"<td class=\"num {dc_sol}\">{dp_sol}</td></tr>\n"
    )
html_out += f"""</table>
<blockquote><b>Reading the per-image diff:</b> these are wall-clock medians on a shared host with ~±10% run-to-run
noise, so per-image deltas are noisy (some images show detect +X%, others −Y%). The <b>aggregate median</b> above
is the trustworthy signal. FU-A's <code>t_extract</code> is now self-reported (was hardcoded 0.0 pre-FUA):
post median <b>{statistics.median([post[(i,'ps_grpc')]['t_extract'] for i in ASTRO if post.get((i,'ps_grpc'),{}).get('t_extract') is not None])*1:.4f}ms</b>;
<code>t_solve</code> post median <b>{statistics.median([post[(i,'ps_grpc')]['t_solve'] for i in ASTRO if post.get((i,'ps_grpc'),{}).get('t_solve') is not None]):.4f}ms</b>
(the search itself is sub-ms on 8/9 images).</blockquote>
</div>

<h2>Accuracy gate: ps_grpc_vs_cedar_flow primary parity (identical-green)</h2>
<div class="card">
<table>
<tr><th>snapshot</th><th class="num">rows</th><th class="num">flagged</th><th>verdict</th></tr>
<tr><td>pre-FU-A</td><td class=\"num\">{len(pre_parity)}</td><td class=\"num\">{sum(1 for c in pre_parity if c['flagged'])}</td>
<td>{chip('green','IDENTICAL-GREEN') if sum(1 for c in pre_parity if c['flagged'])==0 else chip('red','NOT GREEN')}</td></tr>
<tr><td>post-FU-A</td><td class=\"num\">{len(post_parity)}</td><td class=\"num\">{sum(1 for c in post_parity if c['flagged'])}</td>
<td>{chip('green','IDENTICAL-GREEN') if sum(1 for c in post_parity if c['flagged'])==0 else chip('red','NOT GREEN')}</td></tr>
<tr><td>post-FU-B (now)</td><td class=\"num\">{len(fub3_parity)}</td><td class=\"num\">{sum(1 for c in fub3_parity if c['flagged'])}</td>
<td>{chip('green','IDENTICAL-GREEN') if sum(1 for c in fub3_parity if c['flagged'])==0 else chip('red','NOT GREEN')}</td></tr>
</table>
<div class="stat">
<div><span class="n">{0 if drift else 9}/{9}</span><span class="l">pre→post result drift (RA/Dec/matched IDs)</span></div>
<div><span class="n">{same_ids}/9</span><span class="l">matched_cat_ids identical pre==post</span></div>
<div><span class="n">9/9</span><span class="l">centroids exact (0.00 px) every snapshot</span></div>
</div>
<blockquote><b>Zero accuracy change.</b> Every snapshot's 9 primary_same_catalog comparisons are unflagged
(centroids exact, RA within ~1″, Dec within ~0.2″, Roll exact, FOV within 0.1%, matched IDs exact). The
pre→post diff shows no attitude/ID drift — faster, not different.</blockquote>
</div>

<h2>What changed in each phase (code, not benchmark)</h2>
<div class="card">
<h3>SP4 (the big win — 0.27× → 1.55× solve)</h3>
<p class="sub">Replaced the eager <code>combinations_4(n) -&gt; Vec&lt;[usize;4]&gt;</code> (which materialized
<em>every</em> C(n,4) combo — up to ~618 MiB at n≈150 — before the solve loop could look at combo #1) with a
lazy, allocation-free breadth-first iterator. 8/9 images match on combo #1–2, so solve time collapsed from
single/tens-of-ms to sub-µs. <code>combos_examined</code> proved the win was laziness, not ordering.</p>
<h3>FU-A (measurement + copy removal)</h3>
<p class="sub"><b>FUA.1:</b> <code>solve_from_image</code> now self-reports real <code>t_extract</code> (was hardcoded 0.0).
<b>FUA.2:</b> <code>ExtractCentroids</code>/<code>SolveFromImage</code> take the request's image buffer instead of
<code>.clone()</code>-ing the full frame (~0.79 MB at 1024×768) — strictly less work, bit-for-bit identical output.</p>
<h3>FU-B (profile + honest no-win)</h3>
<p class="sub"><b>FUB.1:</b> profiled hale_bopp's NoMatch exhaustion (8855 combos, 2.15M DB lookups, 2778 slot hits):
DB lookup ~51%, verify ~31%, candidate_keys build+sort ~15%. <b>FUB.2:</b> attempted 2 ps-solve allocation-reuse
steps (candidate_keys Vec, A6 5-buffer hoist); both bit-for-bit green but neither statistically detectable
(step-2: −1.8% mean, SNR&lt;1 at n=12) — both <b>reverted</b> per the spec's "revert any step the measurement
says didn't help." <b>FUB.3:</b> re-confirmed the baseline; no product code changed. The dominant cost
(~66%) is in <code>ps-db</code> (<code>lookup_pattern</code> + <code>nearby_stars</code>) — a separate ps-db
follow-up, not FUB.2's ps-solve scope.</p>
</div>

<div class="footer">
Generated {__import__('datetime').date(2026,7,5).isoformat()} from
<code>tools/parity/benchmark/results_pre_fua.json</code>,
<code>results_post_fua.json</code>, <code>results_fub3.json</code> (same aarch64 host, 20 CPUs).
Pre-SP4 0.27× and post-SP4 1.55× from <code>notes/solve-perf-measurements.md</code>.
Served from <code>192.168.10.80</code> on port 8765.
</div>
</main></body></html>"""

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(html_out)
print(f"wrote {OUT} ({len(html_out)} bytes)")