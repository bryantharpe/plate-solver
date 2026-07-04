#!/usr/bin/env python3
"""Report generation for the eval-harness (feat-09 eval-harness, tasks 4.1-4.2).

Reads a ``results.json`` produced by ``run_benchmark.py`` + ``parity.py`` and
deterministically renders ``docs/benchmarks/report.md`` and
``docs/benchmarks/report.html`` (self-contained, no CDN/external refs).

Run (stdlib only, no venv needed - after run_benchmark.py + parity.py have
produced results.json with parity section):

    python3 tools/parity/benchmark/report.py [--results tools/parity/benchmark/results.json] [--output-dir docs/benchmarks]
"""
from __future__ import annotations

import argparse
import html
import json
import statistics
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

BENCHMARK_DIR = Path(__file__).resolve().parent
ROOT = BENCHMARK_DIR.parents[2]

# Fixed system order for deterministic iteration
SYSTEMS = ["ps_grpc", "cedar_flow", "tetra3_original"]

# Placeholder for None/missing values
PLACEHOLDER = "—"


def _median_or_none(values: List[Optional[float]]) -> Optional[float]:
    """Compute median, skipping None entries. Return None if empty or all None."""
    valid = [v for v in values if v is not None]
    if not valid:
        return None
    return statistics.median(valid)


def _format_number(value: Optional[float], decimals: int = 3) -> str:
    """Format a float or None as a string. None becomes placeholder."""
    if value is None:
        return PLACEHOLDER
    if abs(value) < 1e-9:
        return "0"
    return f"{value:.{decimals}f}".rstrip("0").rstrip(".")


def _speedup_ratio(other_median: Optional[float], ps_grpc_median: Optional[float]) -> Optional[float]:
    """Compute speedup: other_median / ps_grpc_median. None if either is None/zero."""
    if ps_grpc_median is None or other_median is None or ps_grpc_median <= 0:
        return None
    return other_median / ps_grpc_median


def _parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--results", type=Path, default=BENCHMARK_DIR / "results.json")
    p.add_argument("--output-dir", type=Path, default=ROOT / "docs" / "benchmarks")
    return p.parse_args()


def _load_results(results_path: Path) -> Dict[str, Any]:
    """Load and validate results.json."""
    if not results_path.is_file():
        raise FileNotFoundError(f"{results_path} not found")
    data = json.loads(results_path.read_text())
    if "metadata" not in data or "results" not in data or "parity" not in data:
        raise ValueError("results.json missing required sections: metadata, results, parity")
    return data


def _index_results(results: List[Dict[str, Any]]) -> Dict[str, Dict[str, Any]]:
    """Index: image_name -> {"kind": str, "systems": {system_name -> {detect, solve}}}."""
    out: Dict[str, Dict[str, Any]] = {}
    for entry in results:
        image_name = entry["image_name"]
        system = entry["detect"]["system"]
        bucket = out.setdefault(image_name, {"kind": entry["kind"], "systems": {}})
        bucket["systems"][system] = {
            "detect": entry["detect"],
            "solve": entry["solve"],
        }
    return out


def _ordered_images(metadata: Dict[str, Any], indexed_results: Dict[str, Dict[str, Any]]) -> List[str]:
    """Return image names in fixed order: astronomical then stress, as per metadata."""
    ordered: List[str] = []
    for img in metadata["corpus"]["astronomical"]:
        if img in indexed_results:
            ordered.append(img)
    for img in metadata["corpus"]["stress"]:
        if img in indexed_results:
            ordered.append(img)
    # Append any images found in results but not in metadata.corpus
    for img in sorted(indexed_results.keys()):  # Sort for determinism
        if img not in ordered:
            ordered.append(img)
    return ordered


def _compute_pairwise_speedups(
    indexed_results: Dict[str, Dict[str, Any]],
    ordered_images: List[str],
    stage: str,
    baseline_system: str,
    other_system: str,
) -> List[float]:
    """Compute per-image speedup ratios for one pairwise comparison.

    Returns: [speedup_ratios_per_image, ...] (speedup = other_median / baseline_median)
    """
    ratios: List[float] = []

    for image in ordered_images:
        # Skip stress images for speedup calculation
        if indexed_results[image].get("kind") == "stress":
            continue

        systems = indexed_results[image].get("systems", {})
        baseline_data = systems.get(baseline_system, {}).get(stage)
        if baseline_data is None or "error" in baseline_data:
            continue
        baseline_samples = baseline_data.get("wall_clock_s")
        baseline_median = _median_or_none(baseline_samples)

        other_data = systems.get(other_system, {}).get(stage)
        if other_data is None or "error" in other_data:
            continue
        other_samples = other_data.get("wall_clock_s")
        other_median = _median_or_none(other_samples)

        speedup = _speedup_ratio(other_median, baseline_median)
        if speedup is not None:
            ratios.append(speedup)

    return ratios


def _headline_speedup_summary(
    data: Dict[str, Any],
    indexed_results: Dict[str, Dict[str, Any]],
) -> str:
    """Generate headline speedup numbers."""
    ordered = _ordered_images(data["metadata"], indexed_results)

    lines = []
    lines.append("## Performance Headline")
    lines.append("")

    for stage in ["detect", "solve"]:
        summary_lines = []
        for baseline, other in [("ps_grpc", "cedar_flow"), ("ps_grpc", "tetra3_original")]:
            ratios = _compute_pairwise_speedups(indexed_results, ordered, stage, baseline, other)
            if ratios:
                median_ratio = _median_or_none(ratios)
                if median_ratio is not None:
                    summary_lines.append(
                        f"{baseline} is {_format_number(median_ratio, decimals=2)}x faster than "
                        f"{other} on {stage} (median over {len(ratios)} astronomical images)"
                    )
                else:
                    summary_lines.append(f"{baseline} vs {other} on {stage}: N/A (insufficient data)")
            else:
                summary_lines.append(f"{baseline} vs {other} on {stage}: N/A (insufficient data)")

        if summary_lines:
            lines.extend(summary_lines)
            lines.append("")

    return "\n".join(lines)


def _methodology_section(data: Dict[str, Any]) -> str:
    """Generate methodology & environment disclosure section."""
    metadata = data["metadata"]
    host = metadata["host"]
    iterations = metadata["iteration_counts"]
    catalogs = metadata["catalogs"]
    limitations = metadata["known_limitations"]

    lines = []
    lines.append("## Methodology & Environment")
    lines.append("")
    lines.append(
        f"This report was generated on a **{host['system']} {host['machine']} "
        f"system with {host['cpu_count']} CPUs**. "
        f"This is **NOT** the PRD's RPi-4B-class or mobile target hardware; these results do not "
        f"represent the performance characteristics of that platform."
    )
    lines.append("")

    lines.append("### Iteration Counts")
    lines.append("")
    lines.append(f"- Detect stage: {iterations['detect']['n_iterations']} iterations (warmup: {iterations['detect']['warmup']})")
    lines.append(f"- Solve stage: {iterations['solve']['n_iterations']} iterations (warmup: {iterations['solve']['warmup']})")
    lines.append(f"- Stress images: {iterations['stress']['n_iterations']} iteration (warmup: {iterations['stress']['warmup']}, timeout: {iterations['stress']['timeout_s']}s)")
    lines.append("")

    lines.append("### Catalogs")
    lines.append("")
    lines.append(f"- `ps_grpc`: {catalogs['ps_grpc']}")
    lines.append(f"- `cedar_flow`: {catalogs['cedar_flow']}")
    lines.append(f"- `tetra3_original`: {catalogs['tetra3_original']} (different from cedar-solve's shared catalog — cross-catalog comparisons only)")
    lines.append("")

    lines.append("### Known Limitations")
    lines.append("")
    for limitation in limitations:
        # Escape any markdown special chars in limitations
        lines.append(f"- {limitation}")
    lines.append("")

    return "\n".join(lines)


def _per_image_tables(data: Dict[str, Any], indexed_results: Dict[str, Dict[str, Any]]) -> str:
    """Generate per-image timing tables."""
    ordered = _ordered_images(data["metadata"], indexed_results)

    lines = []
    lines.append("## Per-Image Timing")
    lines.append("")

    for image in ordered:
        image_entry = indexed_results[image]
        kind = image_entry.get("kind", "unknown")

        lines.append(f"### {image}")
        lines.append("")

        # Create table rows
        detect_rows = []
        solve_rows = []

        systems_dict = image_entry.get("systems", {})
        for system in SYSTEMS:
            sys_data = systems_dict.get(system, {})
            detect = sys_data.get("detect", {})
            solve = sys_data.get("solve", {})

            # Detect row
            if "error" in detect:
                detect_wall = "ERROR"
                detect_algo = "ERROR"
                n_iter_detect = "—"
            else:
                detect_wall_samples = detect.get("wall_clock_s", [])
                detect_algo_samples = detect.get("algorithm_s", [])
                detect_wall = _format_number(_median_or_none(detect_wall_samples), decimals=4)
                detect_algo = _format_number(_median_or_none(detect_algo_samples), decimals=4)
                n_iter_detect = str(detect.get("n_iterations", "—"))

            # Solve row
            if "error" in solve:
                solve_wall = "ERROR"
                solve_algo = "ERROR"
                extract_algo = "ERROR"
                n_iter_solve = "—"
            else:
                solve_wall_samples = solve.get("wall_clock_s", [])
                t_solve_ms = solve.get("t_solve_ms", [])
                solve_wall = _format_number(_median_or_none(solve_wall_samples), decimals=4)
                # t_solve_ms is in milliseconds, convert to seconds
                t_solve_s = [ms / 1000.0 if ms is not None else None for ms in t_solve_ms]
                solve_algo = _format_number(_median_or_none(t_solve_s), decimals=4)
                # t_extract_ms is in milliseconds
                t_extract_ms = solve.get("t_extract_ms", [])
                extract_algo = _format_number(_median_or_none(t_extract_ms), decimals=2)
                n_iter_solve = str(solve.get("n_iterations", "—"))

            detect_rows.append((system, n_iter_detect, detect_wall, detect_algo))
            solve_rows.append((system, n_iter_solve, solve_wall, solve_algo, extract_algo))

        # Detect table
        lines.append("**Detect (wall-clock & algorithm time in seconds)**")
        lines.append("")
        lines.append("| System | Iterations | Wall-Clock (median) | Algorithm (median) |")
        lines.append("|--------|------------|---------------------|-------------------|")
        for system, n_iter, wall, algo in detect_rows:
            lines.append(f"| {system} | {n_iter} | {wall} | {algo} |")
        lines.append("")

        # Solve table
        lines.append("**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**")
        lines.append("")
        lines.append("| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |")
        lines.append("|--------|------------|---------------------|----------------|-------------------------|")
        for system, n_iter, wall, algo, extract in solve_rows:
            lines.append(f"| {system} | {n_iter} | {wall} | {algo} | {extract} |")
        lines.append("")

    return "\n".join(lines)


def _aggregate_speedup_table(data: Dict[str, Any], indexed_results: Dict[str, Dict[str, Any]]) -> str:
    """Generate aggregate median-speedup table (astronomical images only)."""
    ordered = _ordered_images(data["metadata"], indexed_results)
    # Filter to only astronomical images
    astro_images = [img for img in ordered if indexed_results[img].get("kind") == "astronomical"]

    lines = []
    lines.append("## Aggregate Speedup (Astronomical Images)")
    lines.append("")
    lines.append("Median speedup ratios across all astronomical images (higher = faster for baseline):")
    lines.append("")

    lines.append("| Comparison | Detect Speedup | Solve Speedup |")
    lines.append("|------------|----------------|----|")

    # All three pairwise comparisons
    comparisons = [
        ("ps_grpc", "cedar_flow"),
        ("ps_grpc", "tetra3_original"),
        ("cedar_flow", "tetra3_original"),
    ]

    for baseline, other in comparisons:
        detect_ratios = _compute_pairwise_speedups(indexed_results, astro_images, "detect", baseline, other)
        solve_ratios = _compute_pairwise_speedups(indexed_results, astro_images, "solve", baseline, other)

        detect_median = _median_or_none(detect_ratios) if detect_ratios else None
        solve_median = _median_or_none(solve_ratios) if solve_ratios else None

        detect_str = f"{_format_number(detect_median, decimals=2)}x" if detect_median else "N/A"
        solve_str = f"{_format_number(solve_median, decimals=2)}x" if solve_median else "N/A"

        lines.append(f"| {baseline} vs {other} | {detect_str} | {solve_str} |")

    lines.append("")

    return "\n".join(lines)


def _format_check_result(check_dict: Dict[str, Any], check_type: str) -> str:
    """Format a parity check result with numeric details where available."""
    if check_dict is None:
        return PLACEHOLDER

    ok = check_dict.get("ok")

    # Handle cases with no status (missing data, errors)
    if ok is None:
        reason = check_dict.get("reason", "N/A")
        return f"N/A ({reason})"

    # Base status indicator
    status_char = "✓" if ok else "✗"

    # Append numeric details based on check type
    if check_type == "centroids":
        if "max_error_px" in check_dict and "tolerance_px" in check_dict:
            max_err = check_dict["max_error_px"]
            tol = check_dict["tolerance_px"]
            return f"{status_char} (max={max_err:.2f}px/{tol:.2f}px)"

    elif check_type in ("ra_deg", "dec_deg", "roll_deg"):
        if "error" in check_dict and "tolerance" in check_dict:
            err = check_dict["error"]
            tol = check_dict["tolerance"]
            return f"{status_char} (Δ={err:.2f}″/{tol:.2f}″)"

    elif check_type == "fov_deg":
        if "relative_error" in check_dict and "tolerance" in check_dict:
            rel_err = check_dict["relative_error"]
            tol = check_dict["tolerance"]
            return f"{status_char} ({rel_err*100:.2f}%/{tol*100:.2f}%)"

    elif check_type == "matched_cat_ids":
        applicable = check_dict.get("applicable", False)
        if not applicable:
            return f"N/A ({check_dict.get('reason', 'cross-catalog')})"
        if "symmetric_difference_count" in check_dict and "tolerance" in check_dict:
            sym_diff = check_dict["symmetric_difference_count"]
            tol = check_dict["tolerance"]
            return f"{status_char} (Δ={sym_diff}/{tol})"

    elif check_type == "status":
        # Status is always just ok/not ok, no numeric value
        return status_char

    # Fallback for unexpected format
    return status_char


def _parity_results_tables(data: Dict[str, Any]) -> str:
    """Generate parity results tables with flags."""
    parity = data["parity"]
    tolerances = parity["tolerances"]

    lines = []
    lines.append("## Parity Results")
    lines.append("")

    # Tolerances reference table
    lines.append("### Parity Tolerances")
    lines.append("")
    lines.append("| Metric | Tolerance | Source |")
    lines.append("|--------|-----------|--------|")
    lines.append(f"| RA/Dec | {tolerances['ra_dec_arcsec']['value']} arcsec | {tolerances['ra_dec_arcsec']['source']} |")
    lines.append(f"| Centroids | ±{tolerances['centroid_px']['value']} px | {tolerances['centroid_px']['source']} |")
    lines.append(f"| Matched Cat IDs | ≤{tolerances['matched_cat_id_symmetric_diff']['value']} symmetric diff | {tolerances['matched_cat_id_symmetric_diff']['source']} |")
    lines.append(f"| Roll | ±{tolerances['roll_deg']['value']}° | {tolerances['roll_deg']['source']} |")
    lines.append(f"| FOV | ±{tolerances['fov_relative']['value']*100:.1f}% relative | {tolerances['fov_relative']['source']} |")
    lines.append("")

    # Astronomical parity table
    lines.append("### Astronomical Images (Pairwise Comparisons)")
    lines.append("")
    lines.append(
        "| Image | Comparison | Label | Centroids | RA | Dec | Roll | FOV | "
        "Matched IDs | Status | Flagged |"
    )
    lines.append("|-------|-----------|-------|-----------|--------|--------|------|-----|-------------|--------|---------|")

    for entry in parity["astronomical"]:
        image = entry["image_name"]
        comparison = entry["comparison"]
        label = entry["label"]
        flagged = entry["flagged"]

        # Extract check results
        checks = entry.get("checks", {})

        centroids_str = _format_check_result(checks.get("centroids"), "centroids")
        ra_str = _format_check_result(checks.get("ra_deg"), "ra_deg")
        dec_str = _format_check_result(checks.get("dec_deg"), "dec_deg")
        roll_str = _format_check_result(checks.get("roll_deg"), "roll_deg")
        fov_str = _format_check_result(checks.get("fov_deg"), "fov_deg")
        matched_ids_str = _format_check_result(checks.get("matched_cat_ids"), "matched_cat_ids")
        status_str = _format_check_result(checks.get("status"), "status")

        flagged_str = "**FLAGGED**" if flagged else ""

        lines.append(
            f"| {image} | {comparison} | {label} | {centroids_str} | {ra_str} | {dec_str} | "
            f"{roll_str} | {fov_str} | {matched_ids_str} | {status_str} | {flagged_str} |"
        )

    lines.append("")

    # Stress parity table
    lines.append("### Stress Images (Status Check)")
    lines.append("")
    lines.append("| Image | System | Status | Expected | OK | Flagged |")
    lines.append("|-------|--------|--------|----------|-------|---------|")

    for entry in parity["stress"]:
        image = entry["image_name"]
        system = entry["system"]
        status = entry.get("status")
        expected = ", ".join(entry.get("expected_statuses", []))
        ok = entry.get("ok")
        flagged = entry.get("flagged")

        ok_str = "✓" if ok else "✗"
        status_str = status if status else PLACEHOLDER
        flagged_str = "**FLAGGED**" if flagged else ""

        lines.append(f"| {image} | {system} | {status_str} | {expected} | {ok_str} | {flagged_str} |")

    lines.append("")

    return "\n".join(lines)


def _reproduction_commands() -> str:
    """Generate reproduction commands appendix."""
    lines = []
    lines.append("## Reproduction")
    lines.append("")
    lines.append("To reproduce this report after changes to the plate-solving implementations:")
    lines.append("")
    lines.append("```bash")
    lines.append("# From the repo root, release-build the binaries:")
    lines.append("cargo build --release -p ps-grpc")
    lines.append("cargo build --release --manifest-path reference-solutions/cedar-detect/Cargo.toml --bin cedar-detect-server")
    lines.append("")
    lines.append("# Run the benchmark (generates results.json):")
    lines.append("tools/parity/.venv/bin/python tools/parity/benchmark/run_benchmark.py")
    lines.append("")
    lines.append("# Run the parity check (adds parity section to results.json):")
    lines.append("python3 tools/parity/benchmark/parity.py")
    lines.append("")
    lines.append("# Render this report:")
    lines.append("python3 tools/parity/benchmark/report.py")
    lines.append("```")
    lines.append("")

    return "\n".join(lines)


def generate_markdown(data: Dict[str, Any]) -> str:
    """Generate the markdown report."""
    indexed_results = _index_results(data["results"])

    sections = [
        _headline_speedup_summary(data, indexed_results),
        _methodology_section(data),
        _per_image_tables(data, indexed_results),
        _aggregate_speedup_table(data, indexed_results),
        _parity_results_tables(data),
        _reproduction_commands(),
    ]

    return "\n".join(sections)


def _escape_html(text: str) -> str:
    """Escape text for HTML."""
    return html.escape(text)


def generate_html(data: Dict[str, Any]) -> str:
    """Generate a self-contained HTML report from the same data."""
    indexed_results = _index_results(data["results"])

    # Convert markdown to HTML tables and sections
    md = generate_markdown(data)

    # Build HTML
    html_lines = [
        "<!DOCTYPE html>",
        "<html>",
        "<head>",
        "<meta charset='utf-8'>",
        "<meta name='viewport' content='width=device-width, initial-scale=1'>",
        "<title>Plate Solver Eval-Harness Report</title>",
        "<style>",
        "body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; "
        "line-height: 1.6; max-width: 1200px; margin: 40px auto; padding: 20px; color: #333; }",
        "h1, h2, h3 { color: #2c3e50; margin-top: 1.5em; margin-bottom: 0.5em; }",
        "h1 { border-bottom: 3px solid #3498db; padding-bottom: 0.3em; }",
        "h2 { border-bottom: 1px solid #ecf0f1; padding-bottom: 0.2em; }",
        "table { border-collapse: collapse; width: 100%; margin: 1em 0; }",
        "th, td { border: 1px solid #bdc3c7; padding: 0.75em; text-align: left; }",
        "th { background-color: #34495e; color: white; font-weight: bold; }",
        "tr:nth-child(even) { background-color: #f8f9fa; }",
        "tr:hover { background-color: #ecf0f1; }",
        "code { background-color: #f4f4f4; padding: 2px 4px; border-radius: 3px; "
        "font-family: 'Courier New', monospace; }",
        "pre { background-color: #f4f4f4; padding: 1em; border-radius: 5px; overflow-x: auto; "
        "border-left: 4px solid #3498db; }",
        "pre code { background-color: transparent; padding: 0; }",
        "ul, ol { margin: 0.5em 0; padding-left: 2em; }",
        "li { margin: 0.3em 0; }",
        ".flagged { background-color: #ffe6e6; font-weight: bold; }",
        ".error { color: #e74c3c; font-weight: bold; }",
        ".success { color: #27ae60; }",
        ".neutral { color: #95a5a6; }",
        "</style>",
        "</head>",
        "<body>",
        "<h1>Plate Solver Eval-Harness Report</h1>",
    ]

    # Parse markdown and convert to HTML
    # This is a simple conversion focusing on tables and headers
    lines = md.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i]

        # Headers
        if line.startswith("### "):
            html_lines.append(f"<h3>{_escape_html(line[4:])}</h3>")
        elif line.startswith("## "):
            html_lines.append(f"<h2>{_escape_html(line[3:])}</h2>")
        elif line.startswith("# "):
            html_lines.append(f"<h1>{_escape_html(line[2:])}</h1>")

        # Tables
        elif line.startswith("| "):
            # Collect table lines
            table_lines = []
            while i < len(lines) and lines[i].startswith("| "):
                table_lines.append(lines[i])
                i += 1
            i -= 1  # Back up one since we'll increment at end of loop

            # Parse table
            if len(table_lines) >= 2:
                html_lines.append("<table>")

                # Header row
                header_cells = [cell.strip() for cell in table_lines[0].split("|")[1:-1]]
                html_lines.append("<tr>")
                for cell in header_cells:
                    html_lines.append(f"<th>{_escape_html(cell)}</th>")
                html_lines.append("</tr>")

                # Body rows (skip separator row)
                for row_line in table_lines[2:]:
                    cells = [cell.strip() for cell in row_line.split("|")[1:-1]]
                    html_lines.append("<tr>")
                    for cell in cells:
                        # Check for flagged rows
                        is_flagged = "FLAGGED" in cell or "ERROR" in cell
                        css_class = " class='flagged'" if is_flagged else ""
                        html_lines.append(f"<td{css_class}>{_escape_html(cell)}</td>")
                    html_lines.append("</tr>")

                html_lines.append("</table>")

        # Code blocks
        elif line.startswith("```"):
            # Collect code block
            code_lines = []
            i += 1
            while i < len(lines) and not lines[i].startswith("```"):
                code_lines.append(lines[i])
                i += 1
            html_lines.append("<pre><code>")
            html_lines.append(_escape_html("\n".join(code_lines)))
            html_lines.append("</code></pre>")

        # Unordered lists
        elif line.startswith("- "):
            # Collect list items
            list_items = []
            while i < len(lines) and lines[i].startswith("- "):
                list_items.append(lines[i][2:].strip())
                i += 1
            i -= 1

            html_lines.append("<ul>")
            for item in list_items:
                html_lines.append(f"<li>{_escape_html(item)}</li>")
            html_lines.append("</ul>")

        # Paragraphs and other text
        elif line.strip() and not line.startswith("|"):
            html_lines.append(f"<p>{_escape_html(line)}</p>")

        i += 1

    html_lines.extend([
        "</body>",
        "</html>",
    ])

    return "\n".join(html_lines)


def main() -> int:
    args = _parse_args()

    try:
        data = _load_results(args.results)
    except (FileNotFoundError, ValueError, json.JSONDecodeError) as e:
        print(f"FAIL: {e}", file=sys.stderr)
        return 1

    # Generate reports
    markdown = generate_markdown(data)
    html_report = generate_html(data)

    # Write outputs
    args.output_dir.mkdir(parents=True, exist_ok=True)

    md_path = args.output_dir / "report.md"
    html_path = args.output_dir / "report.html"

    md_path.write_text(markdown + "\n")
    html_path.write_text(html_report + "\n")

    print(f"Wrote {md_path}")
    print(f"Wrote {html_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
