# 05 — Pattern Database Generation

The database is built **offline** once per (FOV range, magnitude limit, catalog). It is the
content-addressable store of sky patterns the solver looks things up in. This document
covers `generate_database` for **both** the original tetra3 and cedar-solve (they differ
substantially in how patterns are *chosen*, but share the catalog parsing and the
edge-ratio hashing). Math primitives are in doc 02 §6–7.

```
star catalog (BSC5/HIP/TYC) ─► parse + proper motion ─► magnitude/area trim ─► unit vectors
   ─► choose PATTERN stars (thinned by density)        ─► choose VERIFICATION stars (denser)
   ─► enumerate 4-star patterns sized for FOV          ─► dedup
   ─► for each pattern: edge-ratio key → hash → open-address insert into pattern_catalog
   ─► save .npz (star_table, pattern_catalog, [largest_edge], [key_hashes], catalog_IDs, props)
```

---

## 1. Star catalogs

Three supported source catalogs (placed in the `tetra3/` package dir):

| name | file | size | stars | completeness | good for |
|---|---|---|---|---|---|
| `bsc5` | Yale Bright Star Catalog (binary) | 285 KB | 9,110 | ~mag 7 | wide FOV (>10–20°) |
| `hip_main` | Hipparcos (`.dat`, pipe-delimited) | 51 MB | 118,218 | ~mag 9, ~3/deg² | medium FOV (>3–10°) |
| `tyc_main` | Tycho (`.dat`, pipe-delimited) | 355 MB | 1,058,332 | mag 10, ~25/deg² | narrow FOV (>3°) |

`hip_main` is the default and recommended.

### 1.1 Parsing (`_load_catalog` in cedar; inline in tetra3)

Returns `(star_table, star_catID, epoch_equinox)`; `star_table` rows are
`[RA, Dec, 0, 0, 0, mag]` (RA/Dec radians) — the vector columns are filled later.

**BSC5 (binary):**
- Read a 7×int32 header: entry count is `STARN` (`entry[2]`); if **negative**, equinox is
  **J2000** (else **B1950**); `pm_origin` assumed equal to equinox year. Header sanity
  checks on `STNUM`/`MPROP`/`NMAG`/`NBENT` (warn on unexpected values; `NBENT` should be 32).
- Each record: `mag = raw_mag / 100`; RA/Dec already in **radians**; proper-motion fields
  `RA_pm`, `Dec_PM` in radians/year. `star_catID` = the BSC number (`uint16`).

**HIP / TYC (CSV, `|`-delimited):**
- Equinox **J2000 (ICRS)**, `pm_origin = 1991.25`.
- Skip rows with empty mag (field 5), RA (field 8) or Dec (field 9). If propagating proper
  motion, also skip rows with empty pmRA (12) / pmDec (13).
- RA/Dec in **degrees** (→ converted to radians). Proper motions in **mas/yr** → deg/yr via
  `/1000/60/60`.
- ID: HIP number (`uint32`, field 1) or the three TYC numbers `(TYC1,TYC2,TYC3)`
  (`uint16×3`, split from field 1).

### 1.2 Proper motion propagation

Star catalogs give positions at `pm_origin` plus an annual rate. The database propagates to
`epoch_proper_motion`:

```
RA  = α + μ_α · (epoch_proper_motion − pm_origin)
Dec = δ + μ_δ · (epoch_proper_motion − pm_origin)
```

- `μ_α = μ_α·cosδ / cosδ` — the catalog stores `pmRA = μ_α·cosδ`, so divide by `cosδ` to get
  the RA rate. **Near the poles** `cosδ → 0` makes this blow up, so propagation is skipped
  when `cosδ ≤ 0.1` (tetra3) / `≤ 0.05` (cedar, i.e. |Dec|≳87°).
- `epoch_proper_motion`: `'now'` (default) → current UTC year; a number → that year;
  `None`/`'none'` → no propagation (and then stars lacking proper motions are **kept**).
- Stored in `props.epoch_proper_motion`; the solver returns it so callers can match image
  vintage. Matters mostly for very small FOV or historical images.

### 1.3 Post-parse cleanup (both)

- Drop entries with RA==0 **and** Dec==0 (placeholder/missing).
- **Sort the whole table by magnitude ascending** (brightest first). This brightest-first
  ordering of `star_table` is relied on everywhere downstream.
- Reorder `star_catID` to match.

---

## 2. Magnitude & sky-region trimming

### tetra3
- Keep stars with `mag ≤ star_max_magnitude` (default **7**) — filtered during parse.
- Optional `range_ra`/`range_dec` (degrees) clip to a sky sub-region (handles the 360°/±90°
  wrap). Used to build partial-sky databases.

### cedar
- `range_ra`/`range_dec` **removed** — cedar databases always cover the whole sky.
- `star_max_magnitude` defaults to **auto** (`None`): compute the limiting magnitude from
  required star density:
  ```
  num_fovs          = num_fields_for_sky(min_fov)          # ⌈4π / min_fov²⌉
  total_stars_needed = num_fovs · verification_stars_per_fov · 0.7   # 0.7 = empirical fudge
  ```
  Histogram the catalog magnitudes (100 bins); the magnitude at which the cumulative count
  first exceeds `total_stars_needed` becomes `star_max_magnitude`. (If the catalog's own
  count peak — its completeness limit — is brighter than this, warn that the catalog can't
  supply enough stars.) The `0.7` makes the limiting magnitude ~0.5 mag fainter than the
  dimmest pattern star.

After trimming, compute each star's unit vector `[cosα cosδ, sinα cosδ, sinδ]` into
`star_table[:, 2:5]`, and build a **KD-tree** over those vectors for fast neighbor queries.

---

## 3. Multiscale FOV ladder (both, identical)

Patterns must be sized to the camera FOV. For a FOV **range**, build patterns at several
scales:

```
fov_ratio     = max_fov / min_fov
fov_divisions = ⌈log_step(fov_ratio)⌉ + 1          # step = multiscale_step (default 1.5)
if fov_ratio < √multiscale_step:  pattern_fovs = [max_fov]          # single scale
else:                             pattern_fovs = 2^linspace(log2(min_fov), log2(max_fov), fov_divisions)
```

So a small range → one scale; a wide range → geometrically-spaced scales. Patterns are
generated at **each** scale and pooled (deduped).

---

## 4. Choosing pattern stars vs verification stars

Two densities of stars are needed:

- **Pattern stars** — sparse; the ones combined into 4-star patterns. Density driven so each
  FOV has a manageable number.
- **Verification stars** — dense; used at solve time to count matches in Stage B. Superset
  of pattern stars.

Both are selected by **greedy density thinning**: walk stars brightest-first, keep a star
only if no already-kept star lies within a separation radius (KD-tree
`query_ball_point`). The separation for a target density:

```
separation_for_density(fov, stars_per_fov) = 0.6 · fov / √stars_per_fov     # (cedar helper; tetra3 inlines the same .6·fov/√n)
```

(`0.6` is an empirical packing factor.) This is the **cluster-buster**: it prevents dense
clusters (Pleiades, etc.) from dominating, and bounds pattern count.

### tetra3 specifics
- Pattern density target: `pattern_stars_per_fov` (default **10**); separation uses
  `min_fov` (single-scale) or the current `pattern_fov` (multiscale).
- Verification density target: `verification_stars_per_fov` (default **30**); separation
  `0.6·min_fov/√30`. Built by adding stars to the pattern-star set.

### cedar specifics
- There is **no separate "pattern_stars_per_fov"**. Pattern stars are thinned at separation
  `separation_for_density(fov, verification_stars_per_fov)` with
  `verification_stars_per_fov` default **150**. (The verification set is effectively the
  kept catalog itself; cedar keeps the full trimmed `star_table` and limits density at solve
  time.)

---

## 5. Enumerating patterns

This is the **major algorithmic difference** between the two.

### 5.1 tetra3 — neighbor-combinations per pattern star

For each FOV scale, with the thinned pattern-star set in a KD-tree:

```
for each pattern star s (brightest first):
    remove s from "available"
    neighbours = stars within  pattern_fov  of s   (or pattern_fov/2 if simplify_pattern)
    keep only still-available neighbours
    for each combination of (pattern_size−1)=3 neighbours:
        pattern = (s, + the 3)
        if simplify_pattern:
            add pattern                                  # already bounded by fov/2
        else:
            vectors = the 4 star vectors
            if min pairwise dot product > cos(pattern_fov):   # all mutual angles ≤ FOV
                add pattern
```

- `simplify_pattern=False` (default): patterns may span up to the full FOV; the
  `dots.min() > cos(fov)` test enforces the max mutual angle ≤ FOV. Slower but better.
- `simplify_pattern=True`: neighbors limited to `fov/2` from the central star; faster,
  smaller patterns.
- Patterns accumulate in a **set** (dedup) across all FOV scales.

### 5.2 cedar — lattice fields + breadth-first combinations

cedar abandons "per-anchor-star neighborhoods" for **uniformly distributed lattice fields**,
to guarantee even pattern density over the whole sky regardless of local star density.

For each FOV scale `pattern_fov`:

1. Thin pattern stars (density `verification_stars_per_fov`); put them in a KD-tree;
   record `pattern_index` mapping back to `star_table`.
2. Lay down lattice field centers over the sphere:
   ```
   n = num_fields_for_sky(pattern_fov) · lattice_field_oversampling   # oversampling default 100
   for center in fibonacci_sphere_lattice(n):     # 2n+1 evenly-spread unit vectors (golden-ratio spiral)
   ```
   `lattice_field_oversampling` overlaps the fields so a real (misaligned) FOV still lands
   inside enough fields. Diminishing returns: 100 → 61%, 1000 → 86%, 10000 → 96% of the
   patterns found at oversampling=100000.
3. For each lattice field: gather pattern stars within radius `pattern_fov/2` of the center
   (so patterns can't exceed the FOV). Convert to `star_table` indices; **sort
   (brightness order)**.
4. Generate up to `patterns_per_lattice_field` (default **50**) patterns from that field's
   stars using `breadth_first_combinations(field_stars, 4)` (doc: this enumerates 4-subsets
   so the *last-added* element advances slowest — i.e. it exhausts combinations of the
   brightest stars first). Each new pattern (deduped against the global set) counts toward
   the field's budget. Stop the field once `patterns_per_lattice_field` combinations have
   been *considered*.

The "cluster-buster" thinning in step 1 is what stops a tight bright cluster from burning
the whole `patterns_per_lattice_field` budget on tiny patterns (the Pleiades example in the
source: thinning leaves ~6 separated bright members → C(6,4)=15 < 50, so other stars get
used too).

Patterns from all fields and scales pool into one deduped set.

---

## 6. Building the hash table (both, with cedar refinements)

`pattern_size = 4` always. `pattern_bins = round(1/(4·pattern_max_error))`
(tetra3 default 50; cedar default 250).

### 6.1 Table sizing

- **tetra3**: `catalog_length = 2 · num_patterns` (quadratic probing).
- **cedar**: `catalog_length = next_prime(2·num_patterns)` (quadratic) or
  `next_prime(3·num_patterns)` (linear probing). Prime size disperses keys for the
  `mod`-only linear index function.

Row dtype is the smallest unsigned int that holds the max star index (`uint8` / `uint16` /
`uint32`), `catalog_length × 4`.

### 6.2 Per-pattern insertion

For each pattern (4 star indices into `star_table`):

```
vectors      = the 4 unit vectors
edge_angles  = [2·asin(½·dist(vi,vj)) for the 6 pairs]
sorted       = sort(edge_angles);  L = sorted[-1]            # largest edge
edge_ratios  = sorted[:-1] / L                                # 5 values
key          = [int(ratio · pattern_bins) for ratio in edge_ratios]   # 5-tuple
key_hash     = Σ key[m]·pattern_bins^m                        # 64-bit positional encoding
index        = key_hash·_MAGIC_RAND mod catalog_length   (quadratic)
                or key_hash mod catalog_length            (linear)
# PRESORT the 4 indices by distance from the pattern centroid (doc 02 §7) so solver can pair them
pattern      = reorder(pattern, by ascending centroid distance)
slot         = open-address insert(pattern, index)            # quadratic c² or linear c probing
# auxiliary arrays at the chosen slot:
pattern_largest_edge[slot] = L · 1000                         # milliradians, float16
pattern_key_hashes[slot]   = key_hash & 0xFFFF                # cedar only, uint16
```

- **Presorting** (centroid-distance order) is done at build time so the solver doesn't
  redo it. tetra3 gates this behind `presort_patterns` (default True); cedar always does it.
- **`pattern_largest_edge`** (milliradians as `float16` to use the dtype's range): optional
  in tetra3 (`save_largest_edge`, default False), **always** in cedar. Lets the solver do an
  instant FOV check before vector math.
- **`pattern_key_hashes`** (16-bit): **cedar only** — the hash-collision pre-filter.

### 6.3 Collisions (two kinds)

1. **Pattern-key collisions**: distinct sky patterns producing the same 5-tuple key. More
   frequent with fewer bins. Unavoidable; resolved by Stage-B verification at solve time.
2. **Hash-table collisions**: distinct keys mapping to the same table slot (the
   `mod table_size` step) plus probe chains. Kept low by oversizing the table. cedar can
   measure both (`EVALUATE_COLLISIONS` flag) but it's off by default.

---

## 7. On-disk format (`.npz`)

`save_database` writes a compressed NumPy archive. Arrays:

- `star_table` — `(N, 6)` float32: `[RA, Dec, x, y, z, mag]`.
- `pattern_catalog` — `(catalog_length, 4)` uint8/16/32: star indices, hashed slot order.
- `pattern_largest_edge` — `(catalog_length,)` float16 (milliradians). *(optional/tetra3,
  always/cedar)*
- `pattern_key_hashes` — `(catalog_length,)` uint16. *(cedar only)*
- `star_catalog_IDs` — `(N,)` or `(N,3)` source-catalog IDs.
- `props_packed` — a single NumPy structured-array record of all properties.

### Properties (`props_packed` fields)

Common: `pattern_mode` (always `'edge_ratio'`), `pattern_size`, `pattern_bins`,
`pattern_max_error`, `max_fov`, `min_fov` (degrees), `star_catalog`, `epoch_equinox`,
`epoch_proper_motion`, `verification_stars_per_fov`, `star_max_magnitude`, `range_ra`,
`range_dec`, `presort_patterns`.

- **tetra3 also**: `pattern_stars_per_fov`, `simplify_pattern`.
- **cedar also**: `hash_table_type` (`'quadratic_probe'`/`'linear_probe'`),
  `lattice_field_oversampling`, `patterns_per_lattice_field`, `num_patterns`, and
  **legacy-compat** duplicates so old loaders work (`anchor_stars_per_fov`,
  `pattern_stars_per_fov`, `patterns_per_anchor_star`, `simplify_pattern=True`,
  `range_ra=range_dec=None`).

`load_database` reads these back, rebuilds the star KD-tree (cedar caches it as
`star_kd_tree`), and has fallbacks for missing keys (older databases): e.g.
`verification_stars_per_fov` ← `catalog_stars_per_fov`, `star_max_magnitude` ←
`star_min_magnitude`, `num_patterns` ← `pattern_catalog.shape[0] // 2`, missing `min_fov` ←
`max_fov`.

---

## 8. Defaults & API summary

### tetra3 `generate_database`

```
generate_database(max_fov, min_fov=None, save_as=None, star_catalog='hip_main',
                  pattern_stars_per_fov=10, verification_stars_per_fov=30,
                  star_max_magnitude=7, pattern_max_error=.005, simplify_pattern=False,
                  range_ra=None, range_dec=None, presort_patterns=True,
                  save_largest_edge=False, multiscale_step=1.5, epoch_proper_motion='now')
```

### cedar `generate_database`

```
generate_database(max_fov, min_fov=None, save_as=None, star_catalog='hip_main',
                  lattice_field_oversampling=100, patterns_per_lattice_field=50,
                  verification_stars_per_fov=150, star_max_magnitude=None,   # auto
                  pattern_max_error=.001, multiscale_step=1.5,
                  epoch_proper_motion='now', pattern_stars_per_fov=None, linear_probe=False)
```
(`pattern_stars_per_fov` kept only as a deprecated alias for `lattice_field_oversampling`.)

The bundled default databases: tetra3 `default_database` = 10–30° / mag 7; cedar
`default_database` = 10–30° / mag 8.

CLI (cedar): `tetra3-gen-db STAR_CATALOG SAVE_AS --max-fov 30 [--min-fov ...] [--linear-probe]`
(`cedar-solve/tetra3/cli/generate_database.py`).

---

## 9. Rebuild checklist (database)

1. Catalog reader for BSC5/HIP/TYC → `[RA, Dec, mag]` + IDs; proper-motion propagation with
   pole guard; drop empties; **sort by magnitude**.
2. Magnitude limit (fixed, or cedar's density-derived auto). Compute `[x,y,z]` vectors;
   build a vector KD-tree.
3. Multiscale FOV ladder (`multiscale_step`).
4. Density thinning (`0.6·fov/√n`) for pattern stars (+ verification set in tetra3).
5. Pattern enumeration: tetra3 per-anchor neighbor combinations (with `dots.min()>cos(fov)`)
   **or** cedar Fibonacci lattice fields × breadth-first combinations (budget
   `patterns_per_lattice_field`). Dedup in a set.
6. Edge-ratio key (doc 02 §6), 64-bit key hash, table index (quadratic `·MAGIC mod` or
   linear `mod` prime), open-address insert; presort each pattern by centroid distance.
7. Auxiliary arrays: `pattern_largest_edge` (mrad, f16), cedar `pattern_key_hashes` (16-bit).
8. Serialize all arrays + packed properties to `.npz`.
