#!/usr/bin/env python3
"""Capture hash-lookup parity fixtures from the reference database.

For several known non-empty slots in the pattern catalog, re-compute their
pattern key from the star vectors, then walk the probe chain applying the
same pre-filters as tetra3's _get_all_patterns_for_index, and record the
expected candidate slot indices.

Output: ps-db/tests/fixtures/hash_lookup.json
"""

import numpy as np
import json
import math
import itertools
from pathlib import Path

# ── Paths ────────────────────────────────────────────────────────────────
NPZ_PATH = Path(__file__).resolve().parent.parent.parent / \
    "reference-solutions/cedar-solve/tetra3/data/default_database.npz"
OUT_PATH = Path(__file__).resolve().parent.parent.parent / \
    "ps-db/tests/fixtures/hash_lookup.json"

# ── Reference constants ──────────────────────────────────────────────────
_MAGIC_RAND = np.uint64(2654435761)


def _compute_pattern_key_hash(pattern_key, bin_factor):
    """Reference hash computation."""
    pattern_key = np.uint64(pattern_key)
    bin_factor = np.uint64(bin_factor)
    if pattern_key.ndim == 1:
        return np.sum(
            pattern_key * bin_factor ** np.arange(len(pattern_key), dtype=np.uint64),
            dtype=np.uint64,
        )
    else:
        return np.sum(
            pattern_key * bin_factor ** np.arange(pattern_key.shape[1], dtype=np.uint64)[None, :],
            axis=1,
            dtype=np.uint64,
        )


def _pattern_key_hash_to_index(pattern_key_hash, max_index, linear_probe):
    """Reference hash-to-index mapping."""
    max_index = np.uint64(max_index)
    if linear_probe:
        return pattern_key_hash % max_index
    else:
        with np.errstate(over='ignore'):
            return (pattern_key_hash * _MAGIC_RAND) % max_index


def _get_table_indices_from_hash(hash_index, table, linear_probe):
    """Reference probe chain — returns ALL non-empty indices before first empty."""
    max_ind = np.uint64(table.shape[0])
    hash_index = np.uint64(hash_index)
    found = []
    for c in itertools.count():
        c = np.uint64(c)
        if linear_probe:
            i = (hash_index + c) % max_ind
        else:
            i = (hash_index + c * c) % max_ind
        if all(table[i, :] == 0):
            return np.array(found)
        else:
            found.append(i)


def _angle_from_distance(dist):
    """2 * asin(0.5 * dist)."""
    return 2.0 * np.arcsin(0.5 * dist)


def main():
    data = np.load(str(NPZ_PATH), allow_pickle=True)

    pattern_catalog = data['pattern_catalog']  # (N, 4), uint16 for hip_main default
    star_table = data['star_table']            # (S, 6), float32
    largest_edge = data['pattern_largest_edge']  # (N,), float16
    key_hashes = data['pattern_key_hashes']    # (N,), uint16

    props_packed = data['props_packed']
    linear_probe = props_packed['hash_table_type'][()] == 'linear_probe'
    pattern_bins = int(props_packed['pattern_bins'][()])
    pattern_max_error = float(props_packed['pattern_max_error'][()])

    print(f"DB: {star_table.shape[0]} stars, {pattern_catalog.shape[0]} slots")
    print(f"  linear_probe={linear_probe}, bins={pattern_bins}, max_error={pattern_max_error}")

    # Pick test slots: find non-empty ones at well-spaced positions.
    # The reference considers a slot empty when ALL pattern_catalog values are 0.
    non_empty_mask = ~np.all(pattern_catalog == 0, axis=1)
    non_empty_indices = np.where(non_empty_mask)[0]
    n = len(non_empty_indices)
    test_slots = [
        int(non_empty_indices[0]),
        int(non_empty_indices[n // 4]),
        int(non_empty_indices[n // 2]),
        int(non_empty_indices[3 * n // 4]),
        int(non_empty_indices[-1]),
    ]
   # Also find a collision case (multiple patterns at same hash index)
    # We'll pick the first slot from a collision group and use its key to look up.
    # This tests that our probe chain finds all colliding entries.
    collision_slot = None
    collision_hash_index = None

    _MAGIC_RAND_VAL = np.uint64(2654435761)

    def _ckh(pk, bf):
        pk = np.uint64(pk)
        bf = np.uint64(bf)
        return int(np.sum(pk * bf ** np.arange(len(pk), dtype=np.uint64), dtype=np.uint64))

    def _pkhi(h, mi, lp):
        mi = np.uint64(mi)
        if lp:
            return int(h % mi)
        else:
            with np.errstate(over='ignore'):
                return int((np.uint64(h) * _MAGIC_RAND_VAL) % mi)

    # Scan for collisions among first N non-empty slots
    hi_map = {}
    scan_count = 0
    for slot in non_empty_indices[:30000]:
        cat_row = pattern_catalog[slot]
        star_indices = [int(x) for x in cat_row]
        vectors = star_table[star_indices, 2:5].tolist()
        import math as _m
        edge_angles = []
        for i, j in itertools.combinations(range(4), 2):
            d = _m.dist(vectors[i], vectors[j])
            edge_angles.append(2.0 * _m.asin(0.5 * d))
        edge_angles_sorted = sorted(edge_angles)
        largest_angle = edge_angles_sorted[-1]
        edge_ratios = [a / largest_angle for a in edge_angles_sorted[:-1]]
        pk = tuple(int(r * pattern_bins) for r in edge_ratios)
        fh = _ckh(np.array(pk, dtype=np.uint64), pattern_bins)
        hi = _pkhi(fh, pattern_catalog.shape[0], linear_probe)
        if hi not in hi_map:
            hi_map[hi] = []
        hi_map[hi].append(slot)
        scan_count += 1
        if len(hi_map[hi]) >= 3:
            collision_slot = slot
            collision_hash_index = hi
            break

    print(f"Scanned {scan_count} slots, found collision at hash_index={collision_hash_index}")
    print(f"Total non-empty slots: {n}")
    print(f"Test slots: {test_slots}")

    results = []
    for slot in test_slots:
        cat_row = pattern_catalog[slot]
        star_indices = [int(x) for x in cat_row]

        # Get the 4 star vectors (columns 2,3,4 are x,y,z)
        vectors = star_table[star_indices, 2:5].tolist()  # Nx3

        # Compute edge angles (6 pairwise distances -> angles)
        edge_angles = []
        for i, j in itertools.combinations(range(4), 2):
            d = math.dist(vectors[i], vectors[j])
            edge_angles.append(_angle_from_distance(d))
        edge_angles_sorted = sorted(edge_angles)
        largest_angle = edge_angles_sorted[-1]

        # Edge ratios (5 smaller edges / largest)
        edge_ratios = [a / largest_angle for a in edge_angles_sorted[:-1]]

        # Pattern key (quantised)
        pattern_key = tuple(int(r * pattern_bins) for r in edge_ratios)

        # Compute hash
        pk_array = np.array(pattern_key, dtype=np.uint64)
        full_hash = int(_compute_pattern_key_hash(pk_array, pattern_bins))
        low16 = full_hash & 0xFFFF

        # Hash index
        table_size = pattern_catalog.shape[0]
        hash_index = int(_pattern_key_hash_to_index(
            np.uint64(full_hash), table_size, linear_probe))

        # Walk probe chain with pre-filters (matching _get_all_patterns_for_index)
        # We do NOT apply the FOV filter here — we want the raw lookup result
        # that would be returned when coarse_fov_rad is None.
        all_indices = _get_table_indices_from_hash(
            np.uint64(hash_index), pattern_catalog, linear_probe)

        # Apply 16-bit key hash filter
        if key_hashes is not None:
            keep = key_hashes[all_indices] == low16
            filtered_indices = all_indices[keep]
        else:
            filtered_indices = all_indices

        # Record result (no FOV filter — coarse_fov_rad = None)
        entry = {
            "slot": slot,
            "star_indices": star_indices,
            "pattern_key": list(pattern_key),
            "largest_edge_rad": float(largest_angle),
            "full_hash": full_hash,
            "low16": low16,
            "hash_index": hash_index,
            "candidates_no_fov": [int(x) for x in filtered_indices],
        }

        # Also record result WITH FOV filter (coarse_fov_rad = largest_edge_rad, i.e. the pattern's own FOV)
        # This simulates a solve where the image FOV estimate equals the pattern's largest edge.
        fov_estimate = largest_angle  # use the pattern's own largest edge as FOV estimate
        fov_max_error = fov_estimate * pattern_max_error

        if largest_edge is not None and fov_estimate is not None:
            le_mrad = np.array(largest_edge[all_indices], dtype=np.float32)
            le_rad = le_mrad / 1000.0
            fov2 = le_rad / largest_angle * fov_estimate
            keep_fov = np.abs(fov2 - fov_estimate) < fov_max_error
            # Combine with key hash filter
            if key_hashes is not None:
                keep_combined = (key_hashes[all_indices] == low16) & keep_fov
            else:
                keep_combined = keep_fov
            filtered_with_fov = all_indices[keep_combined]
        else:
            filtered_with_fov = filtered_indices

        entry["candidates_with_fov"] = [int(x) for x in filtered_with_fov]
        entry["fov_estimate_rad"] = float(fov_estimate)
        entry["fov_max_error_rad"] = float(fov_max_error)

        results.append(entry)
        print(f"  slot {slot}: key={pattern_key}, hash_idx={hash_index}, "
              f"candidates_no_fov={len(entry['candidates_no_fov'])}, "
              f"candidates_with_fov={len(entry['candidates_with_fov'])}")

   # Add collision test case: use the key from collision_slot to look up
    # This should return multiple candidates (at least 3)
    if collision_slot is not None:
        cat_row = pattern_catalog[collision_slot]
        star_indices = [int(x) for x in cat_row]
        vectors = star_table[star_indices, 2:5].tolist()
        edge_angles = []
        for i, j in itertools.combinations(range(4), 2):
            d = math.dist(vectors[i], vectors[j])
            edge_angles.append(_angle_from_distance(d))
        edge_angles_sorted = sorted(edge_angles)
        largest_angle = edge_angles_sorted[-1]
        edge_ratios = [a / largest_angle for a in edge_angles_sorted[:-1]]
        pattern_key = tuple(int(r * pattern_bins) for r in edge_ratios)

        full_hash = int(_compute_pattern_key_hash(np.array(pattern_key, dtype=np.uint64), pattern_bins))
        low16 = full_hash & 0xFFFF

        # Walk probe chain
        all_indices = _get_table_indices_from_hash(
            np.uint64(collision_hash_index), pattern_catalog, linear_probe)

        # Apply 16-bit key hash filter
        if key_hashes is not None:
            keep = key_hashes[all_indices] == low16
            filtered_indices = all_indices[keep]
        else:
            filtered_indices = all_indices

        # FOV filter
        fov_estimate = largest_angle
        fov_max_error = fov_estimate * pattern_max_error
        if largest_edge is not None and fov_estimate is not None:
            le_mrad = np.array(largest_edge[all_indices], dtype=np.float32)
            le_rad = le_mrad / 1000.0
            fov2 = le_rad / largest_angle * fov_estimate
            keep_fov = np.abs(fov2 - fov_estimate) < fov_max_error
            if key_hashes is not None:
                keep_combined = (key_hashes[all_indices] == low16) & keep_fov
            else:
                keep_combined = keep_fov
            filtered_with_fov = all_indices[keep_combined]
        else:
            filtered_with_fov = filtered_indices

        collision_entry = {
            "slot": collision_slot,
            "star_indices": star_indices,
            "pattern_key": list(pattern_key),
            "largest_edge_rad": float(largest_angle),
            "full_hash": full_hash,
            "low16": low16,
            "hash_index": collision_hash_index,
            "candidates_no_fov": [int(x) for x in filtered_indices],
            "candidates_with_fov": [int(x) for x in filtered_with_fov],
            "fov_estimate_rad": float(fov_estimate),
            "fov_max_error_rad": float(fov_max_error),
        }
        results.append(collision_entry)
        print(f"  COLLISION slot {collision_slot}: key={pattern_key}, hash_idx={collision_hash_index}, "
              f"candidates_no_fov={len(collision_entry['candidates_no_fov'])}, "
              f"candidates_with_fov={len(collision_entry['candidates_with_fov'])}")

    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    # Convert all numpy types to Python native types for JSON serialization
    def convert(obj):
        if isinstance(obj, (np.integer,)):
            return int(obj)
        elif isinstance(obj, (np.floating,)):
            return float(obj)
        elif isinstance(obj, np.ndarray):
            return obj.tolist()
        elif isinstance(obj, dict):
            return {k: convert(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [convert(v) for v in obj]
        return obj

    with open(OUT_PATH, 'w') as f:
        json.dump(convert(results), f, indent=2)
    print(f"\nFixture written to {OUT_PATH} ({len(results)} entries)")


if __name__ == '__main__':
    main()
