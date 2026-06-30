## Why

A solver is useless without a database, and a phone needs one sized to its FOV and bundled
offline. This change specifies `ps-dbgen`: the offline tool that turns a star catalog into the
pattern database `ps-db` loads — parsing catalogs, propagating proper motion, thinning by
density, enumerating 4-star patterns over Fibonacci lattice fields, hashing them into the
open-addressed table, and serializing. Grounded in
`reference-solutions/docs/05-database-generation.md` (cedar lattice path).

## What Changes

- Introduce the `ps-dbgen` crate (a CLI) and the `database-generation` capability: catalog
  parsing (BSC5/HIP/TYC), proper-motion propagation with a pole guard, magnitude/density
  trimming with cedar's auto limiting-magnitude, the multiscale FOV ladder, density thinning,
  **Fibonacci lattice-field** pattern enumeration via breadth-first combinations, edge-ratio
  key → hash → open-address insert (presorted), auxiliary arrays, and serialization to the
  `feat-03-pattern-database` format.
- The original tetra3 per-anchor-star enumeration (doc 05 §5.1) is **reference-only / a
  non-goal**.

## Capabilities

### New Capabilities

- `database-generation`: offline build of the pattern database — catalog parse, proper motion,
  density thinning, lattice-field pattern enumeration, hashing/insert, serialization.

### Modified Capabilities

(none.)

## Impact

- New crate `ps-dbgen` (CLI binary) depending on `ps-core` (vectors, key/hash, ordering),
  `ps-db` (the output format/writer), and a KD-tree crate (thinning + lattice gathering).
- Produces the database consumed by `ps-db`/`ps-solve`; runs offline, not on device.
- Determinism is required so a given catalog + parameters reproduces the same database.
