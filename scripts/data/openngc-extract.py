#!/usr/bin/env python3
"""Regenerate crates/finder/data/openngc.csv from an OpenNGC checkout.

Usage:
    git clone https://github.com/mattiaverga/OpenNGC.git /tmp/openngc
    git -C /tmp/openngc checkout <the commit crates/finder/data/README.md pins>
    python3 scripts/data/openngc-extract.py /tmp/openngc > crates/finder/data/openngc.csv

The recipe, and every way the extract differs from the source, is documented in
crates/finder/data/README.md. Standard library only; the output is deterministic for a
given OpenNGC commit.
"""

from __future__ import annotations

import csv
import sys
from pathlib import Path

SOURCES = ("NGC.csv", "addendum.csv")
COLUMNS = (
    "name",
    "type",
    "ra_deg",
    "dec_deg",
    "const",
    "major_arcmin",
    "minor_arcmin",
    "pa_deg",
    "mag",
    "messier",
    "common_names",
    "dup_of",
)


def ra_deg(text: str) -> str:
    """Sexagesimal hours (HH:MM:SS.ss) to decimal degrees, five places."""
    h, m, s = (float(part) for part in text.split(":"))
    return f"{(h + m / 60 + s / 3600) * 15:.5f}"


def dec_deg(text: str) -> str:
    """Sexagesimal degrees (+DD:MM:SS.s) to decimal degrees, five places."""
    sign = -1.0 if text.startswith("-") else 1.0
    d, m, s = (float(part) for part in text.lstrip("+-").split(":"))
    return f"{sign * (d + m / 60 + s / 3600):.5f}"


def messier(code: str) -> str:
    """OpenNGC's zero-padded Messier column (`031`) as `M31`."""
    return f"M{int(code)}" if code else ""


def main(checkout: Path) -> int:
    rows: list[dict[str, str]] = []
    for name in SOURCES:
        with open(checkout / "database_files" / name, newline="", encoding="utf-8") as fh:
            rows.extend(csv.DictReader(fh, delimiter=";"))
    by_name = {row["Name"]: row for row in rows}

    def original(row: dict[str, str]) -> str:
        """The object a `Dup` row duplicates: the first referenced row that is not a Dup."""
        for prefix, column in (("NGC", "NGC"), ("IC", "IC")):
            if row[column]:
                target = prefix + row[column]
                if target in by_name and by_name[target]["Type"] != "Dup":
                    return target
        raise SystemExit(f"{row['Name']}: Dup row names no existing original")

    out = csv.writer(sys.stdout, delimiter=";", lineterminator="\n")
    out.writerow(COLUMNS)
    objects = aliases = 0
    for row in rows:
        kind = row["Type"]
        if kind == "NonEx":
            continue
        if row["Name"] == "M102":
            # OpenNGC aliases M102 to M101; the console follows the common
            # identification with NGC 5866 instead (see data/README.md).
            continue
        if kind == "Dup":
            out.writerow([row["Name"], "Dup", "", "", "", "", "", "", "", "", "", original(row)])
            aliases += 1
            continue
        code = messier(row["M"])
        if row["Name"] == "NGC5866":
            code = "M102"
        out.writerow(
            [
                row["Name"],
                kind,
                ra_deg(row["RA"]),
                dec_deg(row["Dec"]),
                row["Const"],
                row["MajAx"],
                row["MinAx"],
                row["PosAng"],
                row["V-Mag"] or row["B-Mag"],
                code,
                row["Common names"],
                "",
            ]
        )
        objects += 1
    print(f"openngc-extract: {objects} objects, {aliases} aliases", file=sys.stderr)
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    raise SystemExit(main(Path(sys.argv[1])))
