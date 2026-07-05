## Performance Headline

ps_grpc is 0.9x faster than cedar_flow on detect (median over 9 astronomical images)
ps_grpc is 7.51x faster than tetra3_original on detect (median over 9 astronomical images)

ps_grpc is 1.68x faster than cedar_flow on solve (median over 9 astronomical images)
ps_grpc is 6.77x faster than tetra3_original on solve (median over 9 astronomical images)

## Methodology & Environment

This report was generated on a **Linux x86_64 system with 4 CPUs**. This is **NOT** the PRD's RPi-4B-class or mobile target hardware; these results do not represent the performance characteristics of that platform.

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
| ps_grpc | 20 | 0.0057 | 0.003 |
| cedar_flow | 20 | 0.005 | 0.0026 |
| tetra3_original | 20 | 0.0412 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.007 | 0.0001 | 3.12 |
| cedar_flow | 5 | 0.011 | 0.0046 | 2.71 |
| tetra3_original | 5 | 0.046 | 0.0045 | 41.13 |

### 2019-07-29T204726_Alt40_Azi135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0055 | 0.0032 |
| cedar_flow | 20 | 0.0048 | 0.0027 |
| tetra3_original | 20 | 0.0419 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0076 | 0.0002 | 3.25 |
| cedar_flow | 5 | 0.0131 | 0.0069 | 2.77 |
| tetra3_original | 5 | 0.1434 | 0.0996 | 45.95 |

### 2019-07-29T204726_Alt40_Azi-45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0047 | 0.0024 |
| cedar_flow | 20 | 0.0042 | 0.002 |
| tetra3_original | 20 | 0.0406 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0062 | 0.0001 | 2.41 |
| cedar_flow | 5 | 0.009 | 0.0039 | 1.99 |
| tetra3_original | 5 | 0.0528 | 0.0102 | 42.19 |

### 2019-07-29T204726_Alt40_Azi45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0055 | 0.0033 |
| cedar_flow | 20 | 0.0049 | 0.0028 |
| tetra3_original | 20 | 0.0411 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0075 | 0.0002 | 3.32 |
| cedar_flow | 5 | 0.0128 | 0.0063 | 2.91 |
| tetra3_original | 5 | 0.0587 | 0.0129 | 45.5 |

### 2019-07-29T204726_Alt60_Azi-135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0051 | 0.0028 |
| cedar_flow | 20 | 0.0046 | 0.0023 |
| tetra3_original | 20 | 0.0402 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0069 | 0.0001 | 2.91 |
| cedar_flow | 5 | 0.0094 | 0.004 | 2.3 |
| tetra3_original | 5 | 0.0444 | 0.0025 | 41.45 |

### 2019-07-29T204726_Alt60_Azi135_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0047 | 0.0024 |
| cedar_flow | 20 | 0.0042 | 0.0021 |
| tetra3_original | 20 | 0.042 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0067 | 0.0002 | 2.37 |
| cedar_flow | 5 | 0.0126 | 0.0073 | 2.13 |
| tetra3_original | 5 | 0.0455 | 0.0039 | 41.24 |

### 2019-07-29T204726_Alt60_Azi-45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0054 | 0.0032 |
| cedar_flow | 20 | 0.0049 | 0.0027 |
| tetra3_original | 20 | 0.0403 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0075 | 0.0001 | 3.19 |
| cedar_flow | 5 | 0.0119 | 0.0054 | 2.7 |
| tetra3_original | 5 | 0.136 | 0.091 | 44.96 |

### 2019-07-29T204726_Alt60_Azi45_Try1.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0067 | 0.0044 |
| cedar_flow | 20 | 0.006 | 0.0038 |
| tetra3_original | 20 | 0.0447 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0087 | 0.0002 | 4.43 |
| cedar_flow | 5 | 0.0146 | 0.0071 | 3.86 |
| tetra3_original | 5 | 0.049 | 0.0042 | 44.63 |

### hale_bopp.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 20 | 0.0055 | 0.0032 |
| cedar_flow | 20 | 0.0052 | 0.003 |
| tetra3_original | 20 | 0.0298 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 5 | 0.0104 | 0.0003 | 3.25 |
| cedar_flow | 5 | 0.0392 | 0.0308 | 3.1 |
| tetra3_original | 5 | 0.0392 | 0.0065 | 29.78 |

### tree.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 1 | 0.0032 | 0.0017 |
| cedar_flow | 1 | 0.0028 | 0.0015 |
| tetra3_original | 1 | 0.0232 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 1 | 0.0041 | 0.0007 | 1.53 |
| cedar_flow | 1 | 0.708 | 0.7039 | 2 |
| tetra3_original | 1 | 0.1215 | 0.0685 | 52.53 |

### test_5mp_g100_e50ms.jpg

**Detect (wall-clock & algorithm time in seconds)**

| System | Iterations | Wall-Clock (median) | Algorithm (median) |
|--------|------------|---------------------|-------------------|
| ps_grpc | 1 | 0.011 | 0.0033 |
| cedar_flow | 1 | 0.0071 | 0.0028 |
| tetra3_original | 1 | 0.0756 | — |

**Solve (wall-clock & solve time in seconds; extract time in milliseconds)**

| System | Iterations | Wall-Clock (median) | Solve (median) | Extract (self-reported, ms) |
|--------|------------|---------------------|----------------|-------------------------|
| ps_grpc | 1 | 0.011 | 0.0001 | 3.23 |
| cedar_flow | 1 | 0.0194 | 0.0103 | 2.77 |
| tetra3_original | 1 | 0.2429 | 0.0212 | 221.19 |

## Aggregate Speedup (Astronomical Images)

Median speedup ratios across all astronomical images (higher = faster for baseline):

| Comparison | Detect Speedup | Solve Speedup |
|------------|----------------|----|
| ps_grpc vs cedar_flow | 0.9x | 1.68x |
| ps_grpc vs tetra3_original | 7.51x | 6.77x |
| cedar_flow vs tetra3_original | 8.35x | 4.58x |

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
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=424.87px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.91″/10.00″) | ✓ (Δ=0.02″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.03%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=424.87px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=655.89px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.04″/10.00″) | ✓ (Δ=0.22″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.05%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi-45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=655.89px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=695.44px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.06″/10.00″) | ✓ (Δ=0.15″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.03%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=695.44px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=664.10px/0.10px) | ✗ (Δ=13.91″/10.00″) | ✓ (Δ=5.45″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.03%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.37″/10.00″) | ✓ (Δ=0.35″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.04%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt40_Azi45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=664.10px/0.10px) | ✗ (Δ=13.54″/10.00″) | ✓ (Δ=5.11″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.07%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=291.85px/0.10px) | ✓ (Δ=2.71″/10.00″) | ✓ (Δ=1.54″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.03%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.23″/10.00″) | ✓ (Δ=0.23″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.06%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=291.85px/0.10px) | ✓ (Δ=2.48″/10.00″) | ✓ (Δ=1.77″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.03%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=629.07px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.17″/10.00″) | ✓ (Δ=0.10″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.05%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi-45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=629.07px/0.10px) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (missing value) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✗ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=634.24px/0.10px) | ✓ (Δ=0.10″/10.00″) | ✓ (Δ=0.78″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.00%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.58″/10.00″) | ✓ (Δ=0.48″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.04%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi135_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=634.24px/0.10px) | ✓ (Δ=0.49″/10.00″) | ✓ (Δ=1.25″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.04%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=812.99px/0.10px) | ✓ (Δ=9.23″/10.00″) | ✓ (Δ=1.35″/10.00″) | ✓ (Δ=0.01″/0.01″) | ✓ (0.01%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.02″/10.00″) | ✓ (Δ=0.22″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.08%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| 2019-07-29T204726_Alt60_Azi45_Try1.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=812.99px/0.10px) | ✓ (Δ=9.24″/10.00″) | ✓ (Δ=1.12″/10.00″) | ✓ (Δ=0.01″/0.01″) | ✓ (0.09%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| hale_bopp.jpg | cedar_flow_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=772.74px/0.10px) | ✗ (Δ=13.04″/10.00″) | ✓ (Δ=4.77″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.01%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |
| hale_bopp.jpg | ps_grpc_vs_cedar_flow | primary_same_catalog | ✓ (max=0.00px/0.10px) | ✓ (Δ=0.04″/10.00″) | ✓ (Δ=0.01″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.01%/0.10%) | ✓ (Δ=0/2) | ✓ |  |
| hale_bopp.jpg | ps_grpc_vs_tetra3_original | cross_catalog_sanity_check | ✗ (max=772.74px/0.10px) | ✗ (Δ=13.07″/10.00″) | ✓ (Δ=4.75″/10.00″) | ✓ (Δ=0.00″/0.01″) | ✓ (0.01%/0.10%) | N/A (cross-catalog: catalog IDs use different catalogs, not directly comparable) | ✓ | **FLAGGED** |

### Stress Images (Status Check)

| Image | System | Status | Expected | OK | Flagged |
|-------|--------|--------|----------|-------|---------|
| test_5mp_g100_e50ms.jpg | cedar_flow | MATCH_FOUND | NO_MATCH, TOO_FEW | ✗ | **FLAGGED** |
| test_5mp_g100_e50ms.jpg | ps_grpc | MATCH_FOUND | NO_MATCH, TOO_FEW | ✗ | **FLAGGED** |
| test_5mp_g100_e50ms.jpg | tetra3_original | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |
| tree.jpg | cedar_flow | MATCH_FOUND | NO_MATCH, TOO_FEW | ✗ | **FLAGGED** |
| tree.jpg | ps_grpc | MATCH_FOUND | NO_MATCH, TOO_FEW | ✗ | **FLAGGED** |
| tree.jpg | tetra3_original | NO_MATCH | NO_MATCH, TOO_FEW | ✓ |  |

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

