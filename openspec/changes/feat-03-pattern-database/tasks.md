## 1. Crate & types

- [ ] 1.1 Create the `ps-db` crate depending on `ps-core`, a KD-tree crate, and `memmap2`
- [ ] 1.2 Define the `Database` struct and the properties record type

## 2. Format & loading

- [ ] 2.1 Specify the native little-endian on-disk layout (arrays + versioned properties header)
- [ ] 2.2 Implement the loader for all arrays with dtype handling (u8/u16/u32 catalog, f16 edges, u16 hashes)
- [ ] 2.3 Implement legacy-fallback property resolution
- [ ] 2.4 Implement the `.npz → native` importer (offline bridge for reference parity)

## 3. Spatial index

- [ ] 3.1 Build and cache the star KD-tree over unit vectors at load
- [ ] 3.2 Implement `nearby_stars(vector, radius)` (chord-radius query, brightest-first)

## 4. Lookup path

- [ ] 4.1 Implement `key → key_hash → table index` (quadratic/linear) via `ps-core`
- [ ] 4.2 Implement probe-chain gathering to the first empty slot
- [ ] 4.3 Implement the 16-bit key pre-filter
- [ ] 4.4 Implement the largest-edge/FOV pre-filter and the edge-ratio band test

## 5. mmap & validation

- [ ] 5.1 Implement the memory-mapped (linear-probe) loading path
- [ ] 5.2 Validate against an imported reference database (lookup returns identical candidate slots)
