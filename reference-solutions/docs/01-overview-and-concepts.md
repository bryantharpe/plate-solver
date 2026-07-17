# 01 — Overview & Core Concepts

This document explains *what* the system does and *why* the algorithms are shaped the way
they are. Subsequent documents give the rebuild-level detail. Read this first.

---

## 1. The problem

You have:

- A grayscale image of the night sky (any orientation, unknown pointing).
- An approximate horizontal field of view (FOV), e.g. "about 11°". (Optional but strongly
  recommended — it makes solving ~10–100× faster.)

You want:

- **RA, Dec** of the image center (where the camera boresight points on the celestial
  sphere).
- **Roll** (rotation of the image about the boresight relative to celestial north).
- **FOV** (refined) and **lens distortion** coefficient.
- Optionally, the identity (catalog ID) and pixel position of each matched star.

The defining constraint: **no prior attitude**. The solver must work "lost in space",
distinguishing the true pointing from the entire celestial sphere. The brute-force
alternative — correlate the image against every possible orientation — is hopeless. The
trick is a **content-addressable geometric fingerprint**.

---

## 2. The central idea: geometric hashing of star patterns

Stars have no individually distinguishing features in a single image (a star is just a
blob). What *is* distinctive is the **relative geometry** of a small group of nearby
stars. Tetra/tetra3/cedar use this scheme:

1. Pick **4 stars** ("a pattern").
2. Compute the **6 pairwise angular distances** between them (4 stars → C(4,2) = 6 edges).
3. **Normalize** the 5 smaller edges by the largest edge → **5 ratios in [0, 1]**.
   This makes the fingerprint **scale-invariant** (independent of FOV/zoom) and
   **rotation-invariant** (only relative distances matter).
4. **Sort** the 5 ratios and **quantize** each into bins → a 5-tuple of small integers:
   the **pattern key** (a *geometric hash*).
5. Use the pattern key to index a precomputed **hash table** of all known sky patterns.

This is *geometric hashing* (Wikipedia: "Geometric hashing"). The pattern key is
invariant to the two things you don't know (orientation and scale), so the same 4 stars
produce the same key whether they appear in the database or in your image.

**Why 4 stars / 5 ratios?** Four stars give six edges; normalizing by the largest leaves
five numbers. Five quantized ratios provide enough entropy to make collisions (different
sky patterns sharing a key) rare while keeping the key small. Three stars (3 edges → 2
ratios) would collide far too often; more stars would make patterns rarer in any given
FOV and combinatorially explosive to enumerate.

### The two-stage match

A pattern-key match is *necessary but not sufficient* — collisions happen. So matching is
two-stage:

- **Stage A — candidate generation (cheap, geometric):** The image pattern's key (and a
  tolerance band around it) selects a handful of catalog patterns from the hash table.
  These are *candidates*.
- **Stage B — verification (authoritative, attitude-based):** For each candidate, compute
  the implied **attitude** (rotation matrix) that maps the catalog pattern onto the image
  pattern. Then project *all* nearby catalog stars into the image and count how many land
  on detected centroids. If "too many to be coincidence", accept; otherwise reject and try
  the next candidate. The accept/reject decision is a **statistical false-alarm test**
  (binomial), not a fixed match count — this is what makes the solver robust.

This is the heart of the whole system. Stage A is fast and approximate; Stage B is the
real proof.

---

## 3. End-to-end walkthrough (the happy path)

Concrete trace of solving one image, default settings, FOV known ≈ 11°:

1. **Detect stars** → list of `(y, x)` centroids, brightest first (e.g. 30 of them).
   (tetra3: `get_centroids_from_image`; cedar: cedar-detect over gRPC.)
2. **Cluster-bust** the centroids (cedar only): drop centroids too close together so a
   single tight cluster (e.g. the Pleiades) can't dominate pattern selection.
3. **Convert centroids to unit vectors** in the camera frame using the pinhole model and
   the FOV estimate. Each `(y,x)` → a 3-D direction `(i, j, k)` with `i` along boresight.
4. **Choose a 4-star pattern** from the brightest centroids (cedar: breadth-first over all
   combinations; tetra3: all C(8,4)=70 combinations of the 8 brightest).
5. **Compute the pattern key**: 6 edge angles → sort → 5 ratios → quantize → 5 integers.
6. **Look up the hash table** over a small neighborhood of keys (tolerance band), gather
   candidate catalog patterns. (cedar additionally pre-filters with a 16-bit key hash and
   a per-pattern largest-edge/FOV check.)
7. For each candidate:
   a. Order the 4 image stars and the 4 catalog stars **identically** (by distance from
      the pattern centroid) so they correspond.
   b. **Solve for attitude** via SVD (Wahba's problem) → rotation matrix `R`.
   c. Reject if `det(R) < 0` (a reflection, not a rotation — cedar only).
   d. Find all catalog stars within the **diagonal FOV** of the implied boresight,
      **derotate** them into the image, project to pixels.
   e. **Match** projected catalog stars to image centroids within `match_radius`.
   f. **False-alarm test**: probability that this many matches arose by chance
      (binomial CDF). If below `match_threshold`, **accept**.
8. On accept: **recompute attitude** using *all* matched stars (not just the 4 pattern
   stars), extract **RA/Dec/Roll**, **refine FOV and distortion** by least squares,
   compute residuals (RMSE), and return.
9. If no candidate passes, try the next image pattern. Give up on timeout / exhaustion.

The first pattern that verifies wins; the search is "first acceptable solution",
not "best over all".

---

## 4. Why a precomputed database, and what's in it

You cannot compute pattern keys for the whole sky at solve time — there are far too many
4-star combinations. So **offline**, `generate_database` enumerates a representative set
of sky patterns sized for your FOV and stores them in a hash table keyed by pattern key.

The database (`.npz`) contains:

- **`star_table`** — every catalog star kept for *verification* (denser than the pattern
  stars). Columns: `RA, Dec, x, y, z, magnitude` (RA/Dec in radians; `x,y,z` the unit
  vector; brightest-first sorted). Used in Stage B to project nearby stars.
- **`pattern_catalog`** — the hash table. Each row is one pattern = the 4 star indices into
  `star_table`. The row index is `hash(pattern_key)` with open-addressing collision
  resolution. Allocated larger than the number of patterns to keep collisions low.
- **`pattern_largest_edge`** (optional in tetra3, always in cedar) — per-pattern largest
  edge angle in **milliradians** (`float16`), enabling an instant FOV sanity check.
- **`pattern_key_hashes`** (cedar only) — a 16-bit hash of each pattern's key, a fast
  pre-filter to discard hash-table collisions before the expensive vector math.
- **`star_catalog_IDs`** — the source-catalog ID (BSC/HIP/TYC) for each `star_table` row.
- **`props_packed`** — all the database properties (FOV range, bins, magnitudes, epoch,
  hash table type, etc.).

Critically, the database is built for a **FOV range** `[min_fov, max_fov]`. Pattern *size*
on the sky must be commensurate with the camera FOV, or the image patterns won't match.
For a wide range, a **multiscale** database is built (patterns at several FOV scales).

---

## 5. Coordinate & sign conventions (summary; full detail in doc 02)

- **Pixel coordinates** are `(y, x)` = `(row down, column right)`, origin at the top-left.
  `(0.5, 0.5)` is the **center of the top-left pixel** (so integer floor gives the pixel
  index). Centroids are always reported as `(y, x)`.
- **Camera frame unit vectors** are `(i, j, k)`: `i` = boresight (out of the camera),
  `j` = image x (horizontal), `k` = image y (vertical). Built from pixel coords by the
  pinhole model.
- **Celestial unit vectors**: `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`.
- **Attitude**: a 3×3 rotation matrix `R` mapping camera-frame vectors to celestial-frame
  vectors (or the transpose, depending on direction). Row 0 of `R` is the boresight
  direction in the celestial frame → the image-center RA/Dec.
- **Angles**: the angular separation between two unit vectors at Euclidean (chord)
  distance `d` is `2·arcsin(d/2)`. This exact relation (rather than `arccos` of a dot
  product) is used throughout for numerical accuracy.

---

## 6. Glossary

- **Centroid** — sub-pixel `(y, x)` location of a detected star.
- **Pattern** — a group of 4 stars used as a matching unit.
- **Pattern key / geometric hash** — the 5 sorted, quantized, normalized edge ratios of a
  pattern (the content-addressable fingerprint).
- **Edge** — the angular distance between a pair of stars in a pattern (6 per pattern).
- **Largest edge** — the maximum of the 6 edges; the normalizer and a proxy for pattern
  angular size (→ FOV).
- **Hash table / pattern catalog** — open-addressed array mapping pattern-key hash → pattern.
- **Probing** — open-addressing collision resolution: *quadratic* (`hash + c²`) or
  *linear* (`hash + c`).
- **Verification star** — a catalog star kept (densely) for Stage-B match counting, not
  necessarily used in any pattern.
- **Attitude / rotation matrix** — the 3-axis orientation relating camera and sky frames.
- **Boresight** — the camera's optical axis direction (image center).
- **Roll** — rotation about the boresight relative to celestial north.
- **FOV** — field of view; "horizontal FOV" unless stated otherwise. **Diagonal FOV**
  is `FOV · √(w²+h²)/w`.
- **Proper motion** — slow apparent stellar motion; catalogs give positions at an epoch
  plus an annual rate, propagated to the database's epoch.
- **Lost-in-space** — solving with no prior attitude (the mode implemented here).
- **Hot pixel** — a defective sensor pixel that is bright in isolation; must not be
  mistaken for a star (cedar-detect classifies and rejects these).
- **Cluster-buster** — thinning of closely-spaced stars/centroids so dense clusters don't
  monopolize pattern generation/selection.

---

## 7. What each codebase is responsible for (rebuild scope)

| Capability | tetra3 (Python) | cedar-solve (Python) | cedar-detect (Rust) |
|---|:--:|:--:|:--:|
| Star detection / centroiding | ✅ `get_centroids_from_image` | ✅ (same code) + can call cedar-detect | ✅ (the fast detector) |
| Database generation | ✅ | ✅ (lattice-field redesign) | — |
| Plate solving / identification | ✅ | ✅ (redesigned solver) | — |
| Attitude / FOV / distortion recovery | ✅ | ✅ | — |
| gRPC service | — | (client only) | ✅ (server) |

If you are rebuilding from scratch and want the best of each: use **cedar-detect's
algorithm** for detection (doc 04), **cedar-solve's** database+solver (docs 05–06), and
fall back to tetra3's simpler versions (docs 03, 05–06) when you want a pure-Python,
dependency-light implementation.
