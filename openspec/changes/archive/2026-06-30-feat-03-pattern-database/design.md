## Context

`ps-db` is the read-side of the database: the format contract shared with `ps-dbgen`, the
loader, the star KD-tree, and the lookup path the solver hammers. The reference stores databases
as NumPy `.npz` (compressed arrays + a packed structured-record of properties). We must decide
whether to mirror `.npz` or define a native Rust binary; either way the **logical** layout
(arrays, dtypes, properties) is fixed by `docs/05` §6–7 and the index math by `docs/02` §6.2–6.4.

## Goals / Non-Goals

**Goals:**
- A `Database` type exposing `star_table`, `pattern_catalog`, `pattern_largest_edge`,
  `pattern_key_hashes`, `star_catalog_IDs`, properties, and a cached star KD-tree.
- `lookup(key, band, fov_estimate, fov_max_error) → candidate patterns` applying the 16-bit and
  largest-edge/FOV pre-filters and the edge-ratio band test.
- `nearby_stars(vector, radius) → brightest-first stars` via the KD-tree.
- An mmap path with the linear-probe layout for narrow-FOV databases.

**Non-Goals:**
- Building/generating the database (that is `feat-04-database-generation`).
- The solve loop, attitude, or projection (that is `feat-05-plate-solver`); `ps-db` only returns
  candidates and nearby stars.

## Decisions

- **Native little-endian binary format, with a documented `.npz` import path.** A native format
  gives zero-copy mmap of typed arrays (f32 star table, u8/u16/u32 catalog, f16 largest-edge,
  u16 key-hashes) and a fixed properties header. We keep an offline `.npz → native` importer so
  reference databases can be reused for parity. Alternative considered: read `.npz` directly via
  an `npy` crate — rejected for the runtime path because zip+mmap don't compose well; kept only
  as the import tool in `ps-dbgen`.
- **KD-tree via `kiddo`** over the f32 unit vectors; chord-radius queries (`2·sin(r/2)`) mirror
  the reference `query_ball_point`. Built once at load and cached.
- **Smallest-unsigned catalog dtype** (u8/u16/u32) chosen by `N` (max star index) — parity with
  the reference and minimal footprint.
- **Pre-filter ordering**: 16-bit key hash first (cheapest), then largest-edge/FOV, then the
  exact edge-ratio band test — discard the most candidates with the least work, matching cedar's
  `_get_all_patterns_for_index`.
- **Two index functions** sourced from `ps-core`: `·_MAGIC_RAND mod` (quadratic) and `mod`
  (linear); `ps-db` owns table sizing and the `hash_table_type` switch.

## Risks / Trade-offs

- [Format divergence from reference] → keep the logical array layout and dtypes identical;
  validate by importing a reference `.npz` and round-tripping a known lookup.
- [f16 milliradian range for largest edge] → f16 covers the needed angular range; document the
  unit (mrad) so the FOV pre-filter divides by 1000 correctly.
- [mmap + endianness on mobile] → fix little-endian on disk; convert if a big-endian target ever
  appears (none in scope).
- [KD-tree memory on device] → for very large narrow-FOV DBs, prefer the linear-probe mmap table
  and bound the star set per FOV (sizing happens in `feat-04`).

## Migration Plan

Greenfield. The `.npz` importer doubles as the parity bridge: load a reference database, import
to native, and assert lookups return the same candidate slots.

## Open Questions

- Exact properties-header encoding (fixed struct vs length-prefixed key/values). Leaning: a
  versioned fixed header + a small typed key/value tail for forward-compat. Finalized alongside
  `feat-04-database-generation` (the writer).
