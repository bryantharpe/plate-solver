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

## `constellation_boundaries.dat` and `constellation_names.dat`

The IAU constellation boundaries (Delporte 1930) as tabulated by N. G. Roman, "Identification
of a Constellation From a Position", PASP 99, 695 (1987), CDS catalogue
[VI/42](https://cdsarc.cds.unistra.fr/viz-bin/Cat?VI/42): RA band in hours, lower declination
in degrees, constellation, for the B1875.0 equinox. The names are the IAU's. Both files are
copied from [Astropy](https://github.com/astropy/astropy) (`astropy/coordinates/data/`),
Copyright (c) 2011-2026, Astropy Developers, under the
[BSD 3-Clause license](https://github.com/astropy/astropy/blob/main/LICENSE.rst). The boundary
table is unchanged; the names file had three misspellings, corrected here to the IAU's
Chamaeleon, Ophiuchus and Piscis Austrinus.
