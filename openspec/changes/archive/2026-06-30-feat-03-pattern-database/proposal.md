## Why

The solver is content-addressable: it turns an image pattern's edge-ratio key into a table
index and pulls candidate catalog patterns, then projects nearby catalog stars to verify. That
requires a loaded, queryable **pattern database** — the star table, the open-addressed hash
table, the per-pattern auxiliary arrays, and a spatial index over star vectors — plus the
lookup path with its cheap pre-filters. This change specifies the `ps-db` crate: the on-disk
format, the loader, the star KD-tree, and the key→candidates lookup. Grounded in
`reference-solutions/docs/05-database-generation.md` §6–7 and `docs/02` §6.2–6.4.

## What Changes

- Introduce the `ps-db` crate and the `pattern-database` capability: the runtime (read-side) of
  the database — on-disk layout, deserialization with legacy fallbacks, star KD-tree
  construction, and `key → hash → table-index → probe-chain → candidates` lookup with the
  16-bit key and largest-edge/FOV pre-filters.
- Define the database properties record (FOV range, bins, max error, hash-table type, pattern
  count, epochs) and the memory-mapping option for narrow-FOV / too-big-for-RAM databases.

## Capabilities

### New Capabilities

- `pattern-database`: load and query a precomputed sky pattern database — format, deserialize,
  star KD-tree, hash lookup, 16-bit + largest-edge/FOV pre-filters, nearby-star queries.

### Modified Capabilities

(none.)

## Impact

- New crate `ps-db` depending on `ps-core` (key hashing, table index, ordering), a KD-tree crate
  (e.g. `kiddo`), and `memmap2` for the mmap path.
- Consumed by `ps-solve` (lookup + nearby-star projection) and produced by `ps-dbgen`
  (`feat-04-database-generation`) — the two share this format contract.
