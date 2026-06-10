## 1. Crate & CLI

- [ ] 1.1 Create the `ps-dbgen` crate (CLI binary) depending on `ps-core`, `ps-db`, and a KD-tree crate
- [ ] 1.2 Define the CLI (`STAR_CATALOG`, `SAVE_AS`, `--max-fov`, `--min-fov`, `--linear-probe`, params)

## 2. Catalog ingest

- [ ] 2.1 Implement the BSC5 binary parser (header sanity, equinox sign rule, mag/RA/Dec/PM fields)
- [ ] 2.2 Implement the HIP/TYC pipe-delimited parser (units, ID handling, empty-field skipping)
- [ ] 2.3 Implement proper-motion propagation with the `cosδ ≤ 0.05` pole guard
- [ ] 2.4 Implement post-parse cleanup (drop 0/0, sort by magnitude) and auto limiting-magnitude

## 3. Star sets & lattice

- [ ] 3.1 Compute unit vectors and build the KD-tree
- [ ] 3.2 Implement the multiscale FOV ladder
- [ ] 3.3 Implement density thinning `0.6·fov/√n` (cluster-buster)
- [ ] 3.4 Implement the Fibonacci sphere lattice and `num_fields_for_sky`

## 4. Pattern enumeration & hashing

- [ ] 4.1 Implement `breadth_first_combinations` and per-field pattern generation with budget + global dedup
- [ ] 4.2 Compute edge-ratio keys, 64-bit hashes, table index; size the (prime) table
- [ ] 4.3 Open-address insert (presorted by centroid distance) and fill `largest_edge` (mrad f16) + `key_hashes` (u16)

## 5. Serialize & validate

- [ ] 5.1 Serialize via the `ps-db` writer (arrays + properties, smallest unsigned catalog dtype)
- [ ] 5.2 Assert determinism (byte-identical re-build) and round-trip through the `ps-db` loader
- [ ] 5.3 Validate pattern counts / sample lookups against an imported reference database
