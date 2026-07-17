# pattern-database Specification

## Purpose

The read side of the precomputed sky index: the on-disk database format and everything needed to
query it at solve time. It defines the file layout and properties record, deserialization
(including legacy fallbacks), the catalog-star KD-tree for nearby-star queries, and the lookup
path from a pattern key to a set of candidate patterns — hash to table index, open-addressing
probe, then the cheap rejection filters (16-bit key pre-filter, largest-edge/FOV bound,
edge-ratio band test) that keep the candidate set small.

It must support **memory-mapped** loading: a narrow-FOV database is far too large to hold in
phone RAM, and the product target is a phone. This capability is read-only — the database is
built offline by `database-generation`.
## Requirements
### Requirement: On-disk database layout

The system SHALL define a database containing: `star_table` (`(N,6)` f32 rows
`[RA, Dec, x, y, z, mag]`, RA/Dec radians, brightest-first); `pattern_catalog`
(`(catalog_length, 4)` unsigned star indices in hashed-slot order, dtype the smallest unsigned
int holding the max index — u8/u16/u32); `pattern_largest_edge` (`(catalog_length,)` f16,
largest edge in **milliradians**); `pattern_key_hashes` (`(catalog_length,)` u16, low 16 bits of
each key hash); `star_catalog_IDs` (`(N,)` or `(N,3)` source IDs); and a packed **properties**
record. (Ref: doc 05 §7.)

#### Scenario: Star table is brightest-first
- **WHEN** the star table is loaded
- **THEN** its rows are ordered by ascending magnitude (brightest first)

#### Scenario: Pattern rows index the star table
- **WHEN** a `pattern_catalog` row is read
- **THEN** its 4 values are indices into `star_table`

### Requirement: Database properties record

The system SHALL carry properties including `pattern_mode` (`'edge_ratio'`), `pattern_size`
(`4`), `pattern_bins`, `pattern_max_error`, `max_fov`, `min_fov` (degrees), `star_catalog`,
`epoch_equinox`, `epoch_proper_motion`, `verification_stars_per_fov`, `star_max_magnitude`,
`hash_table_type` (`'quadratic_probe'`/`'linear_probe'`), and `num_patterns`. The loader SHALL
expose these to the solver. (Ref: doc 05 §7.)

#### Scenario: Bins drive quantization at solve time
- **WHEN** the solver computes a pattern key
- **THEN** it uses `pattern_bins` from the loaded properties

#### Scenario: Hash-table type selects the index function
- **WHEN** `hash_table_type` is `linear_probe`
- **THEN** the table index uses `key_hash mod table_size` (no magic multiply) and linear probing

### Requirement: Deserialization with legacy fallbacks

The system SHALL load all arrays and properties from disk and SHALL apply documented fallbacks
for older databases: `verification_stars_per_fov ← catalog_stars_per_fov`,
`star_max_magnitude ← star_min_magnitude`, `num_patterns ← pattern_catalog.shape[0] // 2`, and
missing `min_fov ← max_fov`. (Ref: doc 05 §7.)

#### Scenario: Missing num_patterns derived
- **WHEN** a database omits `num_patterns`
- **THEN** it is derived as `pattern_catalog.shape[0] // 2`

#### Scenario: Missing min_fov defaults to max_fov
- **WHEN** a database omits `min_fov`
- **THEN** `min_fov` is set equal to `max_fov`

### Requirement: Star KD-tree

The system SHALL build a KD-tree over the `star_table` unit vectors (`x,y,z`) at load time for
fast nearest / radius queries, and SHALL cache it with the loaded database. (Ref: doc 05 §2,
§7.)

#### Scenario: Tree built on load
- **WHEN** a database is loaded
- **THEN** a KD-tree over the star unit vectors is available for queries

### Requirement: Key-to-candidates lookup

The system SHALL, given a pattern key, compute `key_hash` and the table index (per
`hash_table_type`), gather the open-addressing probe chain up to the first empty slot, and
return the occupied slots as candidate patterns. (Ref: doc 02 §6.2–6.3; doc 06 §4.4.)

#### Scenario: Probe chain gathered
- **WHEN** a key hashes to an occupied starting slot
- **THEN** all occupied slots in the probe sequence up to the first empty slot are returned as
  candidates

#### Scenario: Empty starting slot yields nothing
- **WHEN** a key hashes to an empty slot
- **THEN** no candidates are returned for that key

### Requirement: 16-bit key pre-filter

The system SHALL discard probe-chain candidates whose stored `pattern_key_hashes` value differs
from the query's `key_hash & 0xFFFF` before any vector math, removing hash-table collisions
cheaply. (Ref: doc 02 §6.4; doc 06 §4.4.)

#### Scenario: Collision discarded by 16-bit hash
- **WHEN** a probed slot shares the table index but has a different pattern key
- **THEN** its differing 16-bit hash causes it to be discarded before edge comparison

### Requirement: Largest-edge and FOV pre-filter

The system SHALL apply a largest-edge/FOV pre-filter when `pattern_largest_edge` is present and
both `fov_estimate` and `fov_max_error` are provided: it computes each candidate's implied FOV as
the candidate largest edge (milliradians, divided by 1000) divided by the image pattern's largest
edge, times `fov_estimate`, and keeps only candidates whose implied FOV is within `fov_max_error`
of `fov_estimate`. (Ref: doc 06 §4.4.)

#### Scenario: Out-of-FOV candidate dropped
- **WHEN** a candidate's implied FOV deviates from `fov_estimate` by more than `fov_max_error`
- **THEN** it is removed before verification

### Requirement: Edge-ratio band test

The system SHALL compute each surviving candidate's 6 sorted catalog edge angles, form its 5
edge ratios (`edges[:-1]/edges[-1]`), and keep only candidates whose every ratio lies strictly
inside the image pattern's tolerance band `(ratio_min, ratio_max)`. (Ref: doc 06 §4.4.)

#### Scenario: Ratio outside band rejected
- **WHEN** any of a candidate's 5 edge ratios falls outside `(ratio_min, ratio_max)`
- **THEN** the candidate is not passed to verification

### Requirement: Nearby catalog-star query

The system SHALL return catalog stars within an angular radius of a boresight vector via the
KD-tree (`query_ball_point` with chord radius `2·sin(radius/2)`), ordered brightest-first
(since `star_table` is brightness-sorted), for Stage-B projection. (Ref: doc 06 §8.)

#### Scenario: Radius query returns brightest-first
- **WHEN** nearby stars within the diagonal-FOV radius of the boresight are requested
- **THEN** the returned stars are ordered brightest-first

### Requirement: Memory-mapped loading

The system SHALL support memory-mapping the database for narrow-FOV / too-big-for-RAM cases, in
which the `pattern_catalog` uses a **linear-probe** layout so probe chains are contiguous in
memory; quadratic-probe tables assume the table fits in RAM. (Ref: doc 02 §6.3; doc 05 §6.1.)

#### Scenario: Linear-probe table is mmap-friendly
- **WHEN** a linear-probe database is memory-mapped
- **THEN** lookups follow a contiguous probe chain without loading the whole table into RAM

