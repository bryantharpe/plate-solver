# Data bundled into the finder console

These files are compiled into the `finder` binary. Each keeps its own license, as
`reference-solutions/` does; none of them is covered by this project's code license.

## `messier.csv`: the 110 Messier objects

Derived from [OpenNGC](https://github.com/mattiaverga/OpenNGC) by Mattia Verga,
`database_files/NGC.csv` and `database_files/addendum.csv`, fetched on 2026-10-19.
OpenNGC is licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/),
and so is this extract. OpenNGC itself credits NED, HyperLEDA, SIMBAD and the other sources
listed in its README.

Changes from the source:

- Kept only the rows carrying a Messier number (`M` column), dropping `Dup` and `NonEx`
  rows, and only these columns: Messier number, OpenNGC name, type, position, constellation,
  axes, position angle, magnitude (V, falling back to B) and the first common name.
- Converted RA and Dec from sexagesimal to decimal degrees (J2000).
- M102 is unlabelled in OpenNGC because its identity is disputed; this file follows the
  common identification with NGC 5866, the Spindle Galaxy.
- M76 is NGC 650 and NGC 651; the file keeps NGC 650.
- Seven objects have no magnitude in OpenNGC and none here (M22, M35, M62, M69, M70, M71,
  M107).

## `openngc.csv`: every NGC, IC and addendum object, for choosing a target

Derived from [OpenNGC](https://github.com/mattiaverga/OpenNGC) by Mattia Verga,
`database_files/NGC.csv` and `database_files/addendum.csv` at commit
`be3cd6c5498806a2fbb2f7e0452ef802dbb02be0` (2026-10-18, "Revise data from Simbad"). OpenNGC
is licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), and so is
this extract. OpenNGC itself credits NED, HyperLEDA, SIMBAD and the other sources listed in
its README.

**Object count: 13372 objects and 651 aliases.** That is the 14034 source rows less 10
`NonEx` rows and 652 `Dup` rows, with 651 of the `Dup` rows kept as aliases.

To regenerate it, check out OpenNGC at the commit above and run:

```sh
python3 scripts/data/openngc-extract.py /path/to/OpenNGC > crates/finder/data/openngc.csv
```

The script uses the standard library only. Its output for that commit is byte-identical to
this file.

Changes from the source:

- **Rows:**
  - Every row of both files is kept except those typed `NonEx`.
  - A `Dup` row is kept only as an alias. Its type is `Dup`, every other field is empty,
    and `dup_of` names the object it duplicates. That object is the first of its NGC and
    IC references that is not itself a `Dup`; this matters for IC0843, IC4088 and NGC5620,
    which name both.
- **Columns:** only name (OpenNGC's spelling, e.g. `NGC0224`), type, position,
  constellation, axes (arcminutes), position angle, magnitude (V, falling back to B),
  Messier number (`M31`), every common name (comma-separated, as OpenNGC gives them) and
  `dup_of`.
- **Positions:** RA and Dec are converted from sexagesimal to decimal degrees (J2000), to
  five places, exactly as for `messier.csv`. For every Messier object, the row here matches
  `messier.csv` field for field.
- **M102:** OpenNGC lists it as a duplicate of M101 because its identity is disputed. That
  alias is dropped, and NGC 5866, the Spindle Galaxy, carries M102, following the common
  identification as `messier.csv` does.

## `constellation_boundaries.dat` and `constellation_names.dat`

The IAU constellation boundaries (Delporte 1930) as tabulated by N. G. Roman, "Identification
of a Constellation From a Position", PASP 99, 695 (1987), CDS catalogue
[VI/42](https://cdsarc.cds.unistra.fr/viz-bin/Cat?VI/42): RA band in hours, lower declination
in degrees, constellation, for the B1875.0 equinox. The names are the IAU's. Both files are
copied from [Astropy](https://github.com/astropy/astropy) (`astropy/coordinates/data/`),
Copyright (c) 2011-2026, Astropy Developers, under the
[BSD 3-Clause license](https://github.com/astropy/astropy/blob/main/LICENSE.rst), whose full
text is in [`../THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md). The boundary table is
unchanged; the names file had three misspellings, corrected here to the IAU's
Chamaeleon, Ophiuchus and Piscis Austrinus.
