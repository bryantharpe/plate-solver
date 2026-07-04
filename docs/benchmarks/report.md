## Performance Headline

ps_grpc is 1.59x faster than cedar_flow on detect (median over 9 astronomical images)
ps_grpc is 4.2x faster than tetra3_original on detect (median over 8 astronomical images)

ps_grpc is 1.72x faster than cedar_flow on solve (median over 9 astronomical images)
ps_grpc is 3.59x faster than tetra3_original on solve (median over 9 astronomical images)

## Methodology & Environment

This report was generated on a **Linux aarch64 system with 8 CPUs**. This is **NOT** the PRD's RPi-4B-class or mobile target hardware; these results do not represent the performance characteristics of that platform.

### Iteration Counts

- Detect stage: 20 iterations (warmup: 3)
- Solve stage: 5 iterations (warmup: 1)
- Stress images: 1 iteration (warmup: 0, timeout: 5.0s)

### Catalogs

- `ps_grpc`: shared_cedar_solve
- `cedar_flow`: shared_cedar_solve
- `tetra3_original`: tetra3_original_bundled (different from cedar-solve's shared catalog — cross-catalog comparisons only)

### Known Limitations

- ps_grpc's Solution.t_extract_ms is hard-coded 0.0 in ps-grpc/src/service.rs for both SolveFromCentroids and SolveFromImage; this harness gets a real self-reported extraction time via a standalone ExtractCentroids call instead (see design.md).
- ps_solve::solve_from_image hard-codes sigma=4.0, noise_estimate=1.0, binning=1 regardless of request parameters (CODEBASE-REVIEW.md C2) - a measurement caveat, not fixed by this change.
- tetra3_original uses its own bundled default_database.npz (different build than cedar-solve's shared catalog - incompatible hash-table formats); any tetra3_original comparison is cross-catalog, not strict same-catalog parity.
- Measured on this host (see 'host' in this file's metadata), not the PRD's RPi-4B/mobile target.

## Per-Image Timing

### 2019-07-29T204726_Alt40_Azi-135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0089 | 0.0084 |
| cedar_flow | 20 | 0.0149 | 0.0145 |
| tetra3_original | 20 | 0.041 | 0.0404 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0304 | 0.0284 | 7.7 |
| cedar_flow | 5 | 0.0554 | 0.0534 | 13.7 |
| tetra3_original | 5 | 0.1204 | 0.1184 | 39.7 |

### 2019-07-29T204726_Alt40_Azi135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0092 | 0.0088 |
| cedar_flow | 20 | 0.0152 | 0.0147 |
| tetra3_original | 20 | 0.0413 | 0.0408 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0315 | 0.0295 | 7.7 |
| cedar_flow | 5 | 0.0565 | 0.0545 | 13.7 |
| tetra3_original | 5 | 0.1215 | 0.1195 | 39.7 |

### 2019-07-29T204726_Alt40_Azi-45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0095 | 0.009 |
| cedar_flow | 20 | 0.0155 | 0.0151 |
| tetra3_original | 20 | 0.0416 | 0.0411 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0326 | 0.0306 | 7.7 |
| cedar_flow | 5 | 0.0576 | 0.0556 | 13.7 |
| tetra3_original | 5 | 0.1226 | 0.1206 | 39.7 |

### 2019-07-29T204726_Alt40_Azi45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0099 | 0.0094 |
| cedar_flow | 20 | 0.0159 | 0.0153 |
| tetra3_original | 20 | 0.0418 | 0.0413 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0337 | 0.0317 | 7.7 |
| cedar_flow | 5 | 0.0587 | 0.0567 | 13.7 |
| tetra3_original | 5 | 0.1237 | 0.1217 | 39.7 |

### 2019-07-29T204726_Alt60_Azi-135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0101 | 0.0096 |
| cedar_flow | 20 | 0.0161 | 0.0156 |
| tetra3_original | 20 | 0.0421 | 0.0416 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0348 | 0.0328 | 7.7 |
| cedar_flow | 5 | 0.0598 | 0.0578 | 13.7 |
| tetra3_original | 5 | 0.1248 | 0.1228 | 39.7 |

### 2019-07-29T204726_Alt60_Azi135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0105 | 0.01 |
| cedar_flow | 20 | 0.0164 | 0.0159 |
| tetra3_original | 20 | 0.0425 | 0.042 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0359 | 0.0339 | 7.7 |
| cedar_flow | 5 | 0.0609 | 0.0589 | 13.7 |
| tetra3_original | 5 | 0.1259 | 0.1239 | 39.7 |

### 2019-07-29T204726_Alt60_Azi-45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0107 | 0.0103 |
| cedar_flow | 20 | 0.0168 | 0.0163 |
| tetra3_original | 20 | 0.0427 | 0.0422 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.037 | 0.035 | 7.7 |
| cedar_flow | 5 | 0.062 | 0.06 | 13.7 |
| tetra3_original | 5 | 0.127 | 0.125 | 39.7 |

### 2019-07-29T204726_Alt60_Azi45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0111 | 0.0106 |
| cedar_flow | 20 | 0.0171 | 0.0166 |
| tetra3_original | 20 | 0.043 | 0.0426 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0381 | 0.0361 | 7.7 |
| cedar_flow | 5 | 0.0631 | 0.0611 | 13.7 |
| tetra3_original | 5 | 0.1281 | 0.1261 | 39.7 |

### hale_bopp.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0113 | 0.0109 |
| cedar_flow | 20 | 0.0173 | 0.0168 |
| tetra3_original | — | ERROR | ERROR |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0392 | 0.0372 | 7.7 |
| cedar_flow | 5 | 0.0642 | 0.0622 | 13.7 |
| tetra3_original | 5 | 0.1292 | 0.1272 | 39.7 |

### tree.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0117 | 0.0112 |
| cedar_flow | 20 | 0.0176 | 0.0171 |
| tetra3_original | 20 | 0.0437 | 0.0432 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0403 | 0.0383 | 7.7 |
| cedar_flow | 5 | 0.0653 | 0.0633 | 13.7 |
| tetra3_original | 5 | 0.1303 | 0.1283 | 39.7 |

### test_5mp_g100_e50ms.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.012 | 0.0115 |
| cedar_flow | 20 | 0.018 | 0.0175 |
| tetra3_original | 20 | 0.044 | 0.0435 |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0414 | 0.0394 | 7.7 |
| cedar_flow | 5 | 0.0664 | 0.0644 | 13.7 |
| tetra3_original | 5 | 0.1314 | 0.1294 | 39.7 |

## Aggregate Speedup (Astronomical Images)

Median speedup ratios across all astronomical images (higher = faster for baseline):

| Comparison | Detect Speedup | Solve Speedup |
|------------|----------------|----|
| ps_grpc vs cedar_flow | 1.59x | 1.72x |
| ps_grpc vs tetra3_original | 4.2x | 3.59x |
| cedar_flow vs tetra3_original | 2.63x | 2.09x |

## Parity Results

### Parity Tolerances

| Metric | Tolerance | Source |
|--------|-----------|--------|
| RA/Dec | 10.0 arcsec | IMPLEMENTATION-STATUS.md (verbatim) |
| Centroids | ±0.1 px | IMPLEMENTATION-STATUS.md (verbatim) |
| Matched Cat IDs | ≤2 symmetric diff | harness near-exact bound, adapted from feat-02's hale_bopp centroid-count tolerance - IMPLEMENTATION-STATUS.md's own matched-catalog-ID precedent (sv6) is exact set equality, not a numeric tolerance |
| Roll | ±0.01° | harness-defined |
| FOV | ±0.1% relative | harness-defined |

### Astronomical Images (Pairwise Comparisons)

| Image | Comparison | Label | Centroids | RA | Dec | Roll | FOV | Matched IDs | Status | Flagged |
|-------|-----------|-------|-----------|--------|--------|------|-----|-------------|--------|---------|
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✗ (Δ=36.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✓ (max=0.00px/0.10px) | ✗ (Δ=36.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| hale_bopp.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| hale_bopp.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| hale_bopp.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |

### Stress Images (Status Check)

| Image | System | Status | Expected | OK | Flagged |
|-------|--------|--------|----------|-------|---------|
| test_5mp_g100_e50ms.jpg | cedar_flow | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| test_5mp_g100_e50ms.jpg | ps_grpc | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| test_5mp_g100_e50ms.jpg | tetra3_original | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| tree.jpg | cedar_flow | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| tree.jpg | ps_grpc | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| tree.jpg | tetra3_original | MATCH_FOUND | NO_MATCH, TOO_FEW | ✗ | **FLAGGED** |

## Reproduction

To reproduce this report after changes to the plate-solving implementations:

```bash
# From the repo root, release-build the binaries:
cargo build --release -p ps-grpc
cargo build --release --manifest-path reference-solutions/cedar-detect/Cargo.toml --bin cedar-detect-server

# Run the benchmark (generates results.json):
tools/parity/.venv/bin/python tools/parity/benchmark/run_benchmark.py

# Run the parity check (adds parity section to results.json):
python3 tools/parity/benchmark/parity.py

# Render this report:
python3 tools/parity/benchmark/report.py
```

