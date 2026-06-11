#!/usr/bin/env python3
"""Capture golden parity fixtures for the ``ps-core`` (math-core) crate.

Every value written here is produced by the **actual reference** math in
``reference-solutions/cedar-solve`` (the module-level helpers in
``tetra3/tetra3.py``) or, where the reference computes a primitive inline
(celestial RA/Dec <-> unit vector, doc 02 1.3), by the identical NumPy
float64 expression the reference uses to build ``star_table``. Nothing is
hand-fabricated: the JSON emitted here is the contract the Rust ``ps-core``
parity tests assert against (feat-01, ref doc 02).

Conventions captured (binding for ps-core):
  * pixels are ``(y, x)`` with ``(0.5, 0.5)`` = centre of the top-left pixel.
  * angles use ``2*arcsin(d/2)`` (never ``arccos``).
  * compute in f64; the reference stores DB unit vectors as f32.

Inputs are fixed (no RNG) so re-running is byte-deterministic. Outputs go to
``ps-core/tests/fixtures/*.json``. Re-capture after a reference bump with:

    tools/parity/.venv/bin/python tools/parity/capture_core.py

See ``tools/parity/README.md`` for the wider parity workflow.
"""

import json
import sys
from pathlib import Path

import numpy as np

# Reference math helpers live at module scope in tetra3/tetra3.py.
import tetra3.tetra3 as ref

REPO = Path(__file__).resolve().parents[2]
FIXTURES = REPO / "ps-core" / "tests" / "fixtures"


def _f(x):
    """Plain-Python float (json.dump emits a round-trippable f64 repr)."""
    return float(x)


def _vec(a):
    return [_f(v) for v in np.asarray(a, dtype=np.float64).ravel()]


def _mat(a):
    return [[_f(v) for v in row] for row in np.asarray(a, dtype=np.float64)]


def _write(name, obj):
    FIXTURES.mkdir(parents=True, exist_ok=True)
    path = FIXTURES / name
    with open(path, "w") as fh:
        json.dump(obj, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print(f"wrote {path.relative_to(REPO)} ({len(obj.get('cases', []))} cases)")
    return path


# --- celestial unit vectors (doc 02 1.3) --------------------------------------
def capture_celestial_vectors():
    # Fixed (RA, Dec) probes in radians spanning quadrants, poles, and a generic
    # point. RA in [0, 2pi), Dec in (-pi/2, pi/2).
    radec = [
        (0.0, 0.0),
        (np.pi / 2, 0.0),
        (np.pi, 0.0),
        (3 * np.pi / 2, 0.0),
        (0.0, np.pi / 4),
        (0.0, -np.pi / 4),
        (1.0, 0.5),
        (4.2, -1.1),
        (2.5, 1.4),
    ]
    cases = []
    for ra, dec in radec:
        ra = float(ra)
        dec = float(dec)
        x = np.cos(ra) * np.cos(dec)
        y = np.sin(ra) * np.cos(dec)
        z = np.sin(dec)
        # inverse
        ra_back = float(np.mod(np.arctan2(y, x), 2 * np.pi))
        dec_back = float(np.arcsin(z))
        cases.append(
            {
                "ra": ra,
                "dec": dec,
                "vector": [_f(x), _f(y), _f(z)],
                "ra_back": ra_back,
                "dec_back": dec_back,
            }
        )
    return _write(
        "celestial_vectors.json",
        {
            "source": "doc 02 1.3 (x=cosRA cosDec, y=sinRA cosDec, z=sinDec); "
            "inverse RA=atan2(y,x) mod 2pi, Dec=asin(z)",
            "cases": cases,
        },
    )


# --- angle <-> chord distance (doc 02 2) --------------------------------------
def capture_angle_distance():
    angles = [1e-7, 1e-4, 0.001, 0.01, 0.1, 0.5, 1.0, 2.0, 3.0, np.pi]
    cases = []
    for ang in angles:
        ang = float(ang)
        dist = float(ref._distance_from_angle(ang))
        ang_back = float(ref._angle_from_distance(dist))
        cases.append({"angle": ang, "distance": dist, "angle_back": ang_back})
    return _write(
        "angle_distance.json",
        {
            "source": "tetra3._distance_from_angle / _angle_from_distance "
            "(2*sin(a/2), 2*asin(d/2))",
            "cases": cases,
        },
    )


# --- pinhole projection (doc 02 3) --------------------------------------------
def capture_projection():
    # size = (height, width); fov is horizontal FOV in radians.
    size = [480, 640]
    fov = np.deg2rad(20.0)
    height, width = size
    centroids = np.array(
        [
            [height / 2, width / 2],  # image centre -> boresight (1,0,0)
            [height / 2, width],      # horizontal edge -> j == tan(fov/2) pre-norm
            [0.5, 0.5],               # top-left pixel centre
            [120.0, 500.0],
            [400.0, 100.0],
            [240.0, 320.0],
        ],
        dtype=np.float64,
    )
    vectors = ref._compute_vectors(centroids, size, fov)
    # round-trip back to pixels (always returns (centroids, keep) in cedar)
    centroids_back, keep = ref._compute_centroids(vectors, size, fov)
    return _write(
        "projection.json",
        {
            "source": "tetra3._compute_vectors / _compute_centroids "
            "(scale=2tan(fov/2)/width; inverse=-width/2/tan(fov/2))",
            "size": [int(size[0]), int(size[1])],
            "fov": _f(fov),
            "cases": [
                {
                    "centroid": _vec(centroids[i]),
                    "vector": _vec(vectors[i]),
                    "centroid_back": _vec(centroids_back[i]),
                }
                for i in range(len(centroids))
            ],
            "keep": [int(k) for k in keep],
        },
    )


# --- radial distortion (doc 02 4) ---------------------------------------------
def capture_distortion():
    size = [480, 640]
    height, width = size
    centre = [height / 2, width / 2]
    # Off-centre points only: the exact centre has r_undist == 0, where the
    # reference Newton step divides 0/0 -> NaN. The centre's invariance is
    # captured separately below as an undistort-only check.
    centroids = np.array(
        [
            [120.0, 500.0],
            [400.0, 100.0],
            [10.0, 10.0],
            [470.0, 630.0],
        ],
        dtype=np.float64,
    )
    cases = []
    centre_undistorted = {}
    for k in [-0.2, -0.05, 0.0, 0.05, 0.2]:
        k = float(k)
        undist = ref._undistort_centroids(centroids, size, k)
        # round-trip: distort(undistort) ~= identity
        redist = ref._distort_centroids(undist, size, k)
        cases.append(
            {
                "k": k,
                "input": _mat(centroids),
                "undistorted": _mat(undist),
                "distorted_roundtrip": _mat(redist),
            }
        )
        # "center pixel is fixed" scenario: undistort leaves the centre put.
        centre_undistorted[f"{k}"] = _vec(
            ref._undistort_centroids(np.array([centre], dtype=np.float64), size, k)[0]
        )
    return _write(
        "distortion.json",
        {
            "source": "tetra3._undistort_centroids / _distort_centroids "
            "(k'=k*(2/width)^2, Newton tol=1e-6 maxiter=30)",
            "size": [int(size[0]), int(size[1])],
            "centre": [_f(centre[0]), _f(centre[1])],
            "centre_undistorted_by_k": centre_undistorted,
            "cases": cases,
        },
    )


# --- Wahba/SVD attitude (doc 02 5) --------------------------------------------
def _orthonormal_r0():
    """Deterministic proper rotation (det=+1) via QR of a fixed matrix."""
    a = np.array([[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0.0, 0.6]],
                 dtype=np.float64)
    q, r_ = np.linalg.qr(a)
    # fix signs so diag(R) positive, then ensure det = +1
    q = q * np.sign(np.diag(r_))
    if np.linalg.det(q) < 0:
        q[:, -1] *= -1
    return q


def capture_rotation():
    r0 = _orthonormal_r0()
    # catalog (celestial) unit vectors; image = R0 @ catalog (camera frame).
    cat = np.array(
        [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.6, 0.8, 0.0],
            [0.36, 0.48, 0.8],
        ],
        dtype=np.float64,
    )
    cat = cat / np.linalg.norm(cat, axis=1)[:, None]
    img = (r0 @ cat.T).T
    r_rec = ref._find_rotation_matrix(img, cat)
    # RA/Dec/Roll from recovered R, using the reference solve formula (degrees).
    rm = r_rec
    ra_deg = float(np.rad2deg(np.arctan2(rm[0, 1], rm[0, 0])) % 360)
    dec_deg = float(np.rad2deg(np.arctan2(rm[0, 2], np.linalg.norm(rm[1:3, 2]))))
    roll_deg = float(np.rad2deg(np.arctan2(rm[1, 2], rm[2, 2])) % 360)
    # Reflection case: det < 0, built from diag(1,1,-1) @ catalog.
    refl_M = np.diag([1.0, 1.0, -1.0])
    refl_img = (refl_M @ cat.T).T
    refl_R = ref._find_rotation_matrix(refl_img, cat)
    refl_det = float(np.linalg.det(refl_R))
    return _write(
        "rotation.json",
        {
            "source": "tetra3._find_rotation_matrix (H=img^T cat; U,S,V=svd; R=U V). "
            "image_vectors[i] = R0 @ catalog_vectors[i]; recovered R must equal R0. "
            "ra/dec/roll extracted from R_recovered via tetra3 solve formula (degrees, "
            "RA/Roll %360); reflection case = diag(1,1,-1)@cat, recovered det<0.",
            "R0": _mat(r0),
            "det_R0": _f(np.linalg.det(r0)),
            "catalog_vectors": _mat(cat),
            "image_vectors": _mat(img),
            "R_recovered": _mat(r_rec),
            "det_R_recovered": _f(np.linalg.det(r_rec)),
            "ra_deg": ra_deg,
            "dec_deg": dec_deg,
            "roll_deg": roll_deg,
            "reflection_catalog_vectors": _mat(cat),
            "reflection_image_vectors": _mat(refl_img),
            "reflection_R": _mat(refl_R),
            "reflection_det": refl_det,
        },
    )


# --- pattern key hash + table index (doc 02 6.2) ------------------------------
def capture_pattern_hash():
    # cedar default: pattern_max_error = 0.001 -> pattern_bins = 250.
    pattern_max_error = 0.001
    pattern_bins = int(round(1 / (4 * pattern_max_error)))
    assert pattern_bins == 250, pattern_bins
    # also document tetra3 default 0.005 -> 50
    assert int(round(1 / (4 * 0.005))) == 50

    keys = [
        [0, 0, 0, 0, 0],
        [1, 2, 3, 4, 5],
        [10, 50, 100, 200, 249],
        [249, 249, 249, 249, 249],
        [7, 0, 13, 0, 200],
    ]
    table_size = 1000003  # a prime, for index probes
    cases = []
    for key in keys:
        kh = int(ref._compute_pattern_key_hash(np.array(key, dtype=np.uint64),
                                               pattern_bins))
        idx_quad = int(ref._pattern_key_hash_to_index(np.uint64(kh),
                                                      table_size, False))
        idx_lin = int(ref._pattern_key_hash_to_index(np.uint64(kh),
                                                     table_size, True))
        cases.append(
            {
                "key": [int(v) for v in key],
                "key_hash": kh,
                "key_hash_low16": kh & 0xFFFF,
                "index_quadratic": idx_quad,
                "index_linear": idx_lin,
            }
        )
    return _write(
        "pattern_hash.json",
        {
            "source": "tetra3._compute_pattern_key_hash / _pattern_key_hash_to_index "
            "(key_hash=sum key[m]*bins^m uint64; quad idx=(h*MAGIC)%N, lin idx=h%N)",
            "pattern_max_error": pattern_max_error,
            "pattern_bins": pattern_bins,
            "magic_rand": int(ref._MAGIC_RAND),
            "table_size": table_size,
            "cases": cases,
        },
    )


def main() -> int:
    print(f"numpy {np.__version__}; capturing ps-core fixtures -> {FIXTURES}")
    capture_celestial_vectors()
    capture_angle_distance()
    capture_projection()
    capture_distortion()
    capture_rotation()
    capture_pattern_hash()
    written = sorted(p.name for p in FIXTURES.glob("*.json"))
    print(f"OK: {len(written)} fixture file(s): {', '.join(written)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
