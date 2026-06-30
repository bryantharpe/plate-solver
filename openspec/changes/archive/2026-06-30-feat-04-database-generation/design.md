## Context

`ps-dbgen` is an offline CLI that builds the database `ps-db` loads. It re-implements
cedar-solve's `generate_database` lattice-field path (`docs/05`). It is not on the device hot
path, so clarity and determinism beat raw speed — but it must produce a database whose lookups
match the reference. Catalog parsing and proper motion are shared with tetra3; the pattern
enumeration is cedar's lattice-field redesign.

## Goals / Non-Goals

**Goals:**
- Parse BSC5/HIP/TYC, propagate proper motion, thin by density, enumerate patterns over
  Fibonacci lattice fields, hash/insert, and serialize to the `ps-db` format.
- Cedar's auto limiting-magnitude from required density; the multiscale FOV ladder.
- Deterministic output and a CLI mirroring `tetra3-gen-db`.

**Non-Goals:**
- The tetra3 per-anchor-star neighbor-combination enumeration (doc 05 §5.1) — reference-only.
- Partial-sky databases (`range_ra`/`range_dec`) — removed in cedar; whole-sky only.
- On-device generation; this is a build-time tool.

## Decisions

- **Cedar lattice path only.** Fibonacci-sphere field centers with `lattice_field_oversampling`
  guarantee uniform pattern density over the whole sky regardless of local crowding, and bound
  patterns/field — the reason cedar replaced per-anchor enumeration. We implement only this.
- **`breadth_first_combinations(field_stars, 4)`** so the brightest stars' combinations are
  exhausted first; the per-field budget (`patterns_per_lattice_field`) caps work and the
  global dedup set prevents repeats across overlapping fields and scales.
- **Auto limiting-magnitude by default** (`star_max_magnitude = None`): derive from
  `num_fields_for_sky(min_fov)·verification_stars_per_fov·0.7`; warn if the catalog's
  completeness limit is brighter than required.
- **Determinism**: fixed iteration order (catalog sort, lattice index order, breadth-first
  order), no RNG; the Fibonacci spiral is a closed-form deterministic sequence.
- **Writer lives here, format owned by `ps-db`.** `ps-dbgen` calls the `ps-db` serializer so the
  read/write contract can't drift; it also hosts the `.npz → native` importer for reference
  parity.

## Risks / Trade-offs

- [Catalog file size/licensing] (HIP 51 MB, TYC 355 MB) → build device databases offline and
  ship only the compiled DB; document catalog provenance.
- [Lattice oversampling vs build time] → oversampling 100 finds ~61% of the patterns of
  oversampling 100000; expose the knob, default 100 (cedar default) with documented trade-off.
- [Auto-magnitude vs catalog completeness] → warn when the catalog cannot supply the needed
  density; allow an explicit `star_max_magnitude` override.
- [Determinism across platforms] → avoid floating-point-order-dependent set iteration; key the
  global dedup set on the canonical sorted pattern index tuple.

## Migration Plan

Greenfield offline tool. Validate by importing a reference `.npz`, regenerating with matching
parameters, and asserting equal pattern counts and that sample lookups return corresponding
patterns (within enumeration-order tolerance).

## Open Questions

- Whether to ship a small bundled default database (e.g. 10–30°, mag 8, like cedar's
  `default_database`) as a test fixture in-repo or generate on demand. Leaning: generate on
  demand in CI; bundle only a tiny FOV-matched DB for mobile tests (decided in
  `feat-07-mobile-runtime`).
