# database-generation Specification

## Purpose
TBD - created by archiving change feat-04-database-generation. Update Purpose after archive.
## Requirements
### Requirement: Star catalog parsing

The system SHALL parse three source catalogs into rows `[RA, Dec, mag]` (RA/Dec radians) plus
source IDs: BSC5 (binary; entry count `STARN`, negative ⇒ J2000 else B1950; `mag = raw/100`;
RA/Dec already radians; ID = BSC number), and HIP / TYC (pipe-delimited ICRS J2000,
`pm_origin = 1991.25`; RA/Dec degrees → radians; rows with empty mag/RA/Dec skipped; ID = HIP
number or the three TYC numbers). (Ref: doc 05 §1.1.)

#### Scenario: BSC5 equinox from sign
- **WHEN** the BSC5 header entry count is negative
- **THEN** the equinox is taken as J2000 (otherwise B1950)

#### Scenario: HIP degrees converted to radians
- **WHEN** a HIP row with RA/Dec in degrees is parsed
- **THEN** the stored RA/Dec are in radians

### Requirement: Proper-motion propagation

The system SHALL propagate positions from `pm_origin` to `epoch_proper_motion` using
`RA += μ_α·Δt`, `Dec += μ_δ·Δt` (recovering the RA rate by dividing the catalog `pmRA = μ_α·cosδ`
by `cosδ`), and SHALL skip propagation near the poles where `cosδ ≤ 0.05` to avoid blow-up.
`epoch_proper_motion` defaults to the current year (`'now'`); `none` disables propagation (and
keeps stars lacking proper motions). (Ref: doc 05 §1.2.)

#### Scenario: Pole guard
- **WHEN** a star has `cosδ ≤ 0.05` (|Dec| ≳ 87°)
- **THEN** proper-motion propagation is skipped for that star

#### Scenario: Epoch recorded
- **WHEN** a database is generated
- **THEN** `epoch_proper_motion` is stored in its properties for the solver to report

### Requirement: Post-parse cleanup and magnitude limiting

The system SHALL drop entries with `RA == 0 and Dec == 0`, sort the table by magnitude ascending
(brightest first), and apply a magnitude limit. With cedar's **auto** limit
(`star_max_magnitude = None`), the limit SHALL be derived from required density:
`total_stars_needed = num_fields_for_sky(min_fov) · verification_stars_per_fov · 0.7`, choosing
the magnitude at which the cumulative catalog count first exceeds that total. (Ref: doc 05
§1.3, §2.)

#### Scenario: Brightest-first table
- **WHEN** the parsed catalog is cleaned
- **THEN** rows are sorted by ascending magnitude

#### Scenario: Auto limiting magnitude
- **WHEN** `star_max_magnitude` is unset
- **THEN** the limit is the magnitude where the cumulative count first exceeds
  `num_fields_for_sky(min_fov)·verification_stars_per_fov·0.7`

### Requirement: Multiscale FOV ladder

The system SHALL generate patterns at one or more FOV scales: with `fov_ratio = max_fov/min_fov`
and step `multiscale_step` (default 1.5), use a single scale `[max_fov]` when
`fov_ratio < √multiscale_step`, else geometrically-spaced scales
`2^linspace(log2(min_fov), log2(max_fov), fov_divisions)`. Patterns from all scales pool and
dedup. (Ref: doc 05 §3.)

#### Scenario: Narrow range uses one scale
- **WHEN** `fov_ratio < √multiscale_step`
- **THEN** patterns are generated only at `max_fov`

### Requirement: Density thinning (cluster-buster)

The system SHALL thin stars greedily brightest-first, keeping a star only if no already-kept
star lies within `separation = 0.6·fov/√stars_per_fov` (KD-tree radius query), producing the
pattern-star set at density `verification_stars_per_fov` (default 150 in cedar). This prevents
dense clusters from dominating and bounds the pattern count. (Ref: doc 05 §4.)

#### Scenario: Cluster thinned
- **WHEN** many stars fall within the separation radius of a kept star
- **THEN** only the brightest is kept and the rest are skipped at that scale

### Requirement: Lattice-field pattern enumeration

The system SHALL enumerate patterns over uniformly distributed lattice fields: place
`num_fields_for_sky(pattern_fov)·lattice_field_oversampling` (oversampling default 100) Fibonacci
sphere centers, gather pattern stars within `pattern_fov/2` of each center, and generate up to
`patterns_per_lattice_field` (default 50) patterns per field via `breadth_first_combinations`
over the field's stars (brightest combinations first), deduping against the global set. (Ref:
doc 05 §5.2.)

#### Scenario: Field radius bounds pattern size
- **WHEN** stars are gathered for a lattice field
- **THEN** only stars within `pattern_fov/2` of the field center are used, so no pattern exceeds
  the FOV

#### Scenario: Per-field budget
- **WHEN** a field has generated `patterns_per_lattice_field` patterns
- **THEN** enumeration for that field stops

### Requirement: Edge-ratio hashing and insertion

The system SHALL, per pattern, compute the 6 sorted edge angles, the largest edge `L`, the 5
edge ratios, the quantized key (`pattern_bins = round(1/(4·pattern_max_error))`, cedar default
250), the 64-bit `key_hash`, and the table index, then open-address insert the pattern
(presorted by centroid distance) and record `pattern_largest_edge = L·1000` (mrad, f16) and
`pattern_key_hashes = key_hash & 0xFFFF` (u16) at the chosen slot. Table size SHALL be
`next_prime(2·N)` (quadratic) or `next_prime(3·N)` (linear). (Ref: doc 05 §6.)

#### Scenario: Patterns presorted at build time
- **WHEN** a pattern is inserted
- **THEN** its 4 star indices are ordered by ascending centroid distance so the solver need not
  reorder them

#### Scenario: Prime table size
- **WHEN** the hash table is allocated for `N` patterns with quadratic probing
- **THEN** its size is `next_prime(2·N)`

### Requirement: Serialization to the database format

The system SHALL serialize `star_table`, `pattern_catalog`, `pattern_largest_edge`,
`pattern_key_hashes`, `star_catalog_IDs`, and the packed properties into the
`feat-03-pattern-database` on-disk format, choosing the smallest unsigned catalog dtype that
holds the max star index. (Ref: doc 05 §6–7.)

#### Scenario: Round-trips through the loader
- **WHEN** a generated database is written and then loaded by `ps-db`
- **THEN** all arrays and properties read back identically

### Requirement: Deterministic, offline generation

The system SHALL be deterministic: the same catalog, parameters, and epoch SHALL reproduce the
same database byte-for-byte, and the tool SHALL run offline (no network, not on device). (Ref:
doc 05; PRD non-functional requirements.)

#### Scenario: Reproducible build
- **WHEN** generation runs twice with identical inputs and parameters
- **THEN** the two output databases are byte-for-byte identical

### Requirement: Generation CLI

The system SHALL provide a CLI accepting at least `STAR_CATALOG`, `SAVE_AS`, `--max-fov`,
optional `--min-fov`, and `--linear-probe`, mirroring the reference `tetra3-gen-db` entry point.
(Ref: doc 05 §8.)

#### Scenario: Linear-probe flag selects table type
- **WHEN** `--linear-probe` is passed
- **THEN** the generated database uses a linear-probe hash table sized `next_prime(3·N)`

