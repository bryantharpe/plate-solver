# Project Context — Plate Solver (Rust)

> This file is OpenSpec project context for the **plate-solver** repository: a from-scratch
> **Rust** reimplementation of the tetra3/cedar "lost-in-space" star-field plate-solving
> system, delivered to consumers over **gRPC** and embeddable on **mobile** (iOS/Android).
> It is the shared vocabulary and convention reference for every change under
> `openspec/changes/`. The product-level "why/what" lives in [`PRD.md`](./PRD.md).

## 1. Product summary

Given a grayscale photograph of the night sky and a rough horizontal field of view (FOV),
determine where the camera was pointing — its **right ascension (RA)**, **declination
(Dec)**, and **roll** — purely from the geometric arrangement of the stars, with **no prior
pointing knowledge** (the *lost-in-space* problem). The output is a full 3-axis attitude plus
refined FOV and lens distortion, optionally with the identity and pixel position of each
matched catalog star.

The pipeline has two halves:

```
   IMAGE  ──star detection──►  CENTROIDS (y,x, brightest first)  ──plate solving──►  ATTITUDE
  (8-bit gray)  (centroiding)      brightest-first list           (identification)   RA,Dec,Roll,FOV,distortion
```

1. **Star detection / centroiding** — find sub-pixel `(y,x)` positions of star-like spots,
   brightest first.
2. **Plate solving / identification** — match a 4-star geometric "fingerprint" of those
   centroids against a precomputed pattern database, verify statistically, and recover the
   attitude.

The build target is a phone, so **Rust-level performance** is a primary constraint, not an
afterthought.

## 2. The three reference codebases (read-only source of truth)

All specs derive from `reference-solutions/` and its distilled docs in
`reference-solutions/docs/01..08`. **Never modify `reference-solutions/`** — it is the
authoritative source we are re-implementing.

| Folder | Language | Role | Lineage |
|---|---|---|---|
| `tetra3/` | Python | **Original** plate solver (detection + DB gen + solving) by ESA (Gustav Pettersson), a rewrite of MIT/Brown's *Tetra*. | The ancestor. |
| `cedar-solve/` | Python | **Evolution of tetra3** by Steven Rosenthal: re-engineered DB generator (lattice fields), smarter solver (breadth-first, timeouts, status codes, 16-bit hash pre-filter), Cedar Detect hook. | Fork/superset of tetra3. |
| `cedar-detect/` | Rust | **High-performance star detector only**, exposed over gRPC. Does *not* plate-solve. | Companion to cedar-solve. |

> **Naming trap:** `cedar-solve`'s Python package is *also* named `tetra3` (kept for drop-in
> compatibility). So `cedar-solve/tetra3/tetra3.py` is the **cedar** code, **not** the
> original. In these specs, "**tetra3**" = the original `tetra3/` package; "**cedar**" =
> `cedar-solve` (solver) + `cedar-detect` (detector).

**This project rebuilds the cedar path throughout** (it strictly supersedes tetra3).
tetra3's simpler detector (doc 03) and per-anchor DB enumeration (doc 05 §5.1) are
**reference-only / non-goals** for v1.

## 3. Reference Documentation Map

Each feature (OpenSpec change) is derived from and cites specific reference docs. Spec authors
must ground requirements in these and cite the doc number.

| Reference doc | Feeds feature (change) |
|---|---|
| `docs/01-overview-and-concepts.md` | all (concepts, glossary, end-to-end walkthrough) |
| `docs/02-coordinate-systems-and-math.md` | **`math-core`** (`feat-01-foundation-math-core`) |
| `docs/03-star-detection-tetra3.md` | reference-only (tetra3 simple detector; non-goal) |
| `docs/04-star-detection-cedar-detect.md` | **`star-detection`** (`feat-02-star-detection`) |
| `docs/05-database-generation.md` | **`pattern-database`** (`feat-03-`, format/load §6–7) + **`database-generation`** (`feat-04-`, build §1–6) |
| `docs/06-plate-solving.md` | **`plate-solver`** (`feat-05-plate-solver`) |
| `docs/07-cedar-detect-service-api.md` | **`grpc-service`** (`feat-06-grpc-service`) |
| `docs/08-tetra3-vs-cedar-comparison.md` | cross-cutting defaults/decisions (all designs); informs **`mobile-runtime`** |
| perf notes in docs 04/06/08 | **`mobile-runtime`** (`feat-07-mobile-runtime`) |

Feature dependency order (implementation order): `math-core` → `star-detection` → `pattern-database`
→ `database-generation` → `plate-solver` → `grpc-service` → `mobile-runtime`.

## 4. Coordinate & sign conventions (binding across all specs)

- **Pixel coordinates** are `(y, x)` = `(row down, column right)`, origin top-left.
  `(0.5, 0.5)` is the **center of the top-left pixel** (integer floor → pixel index).
  Centroids are always reported `(y, x)`. `size = (height, width)`.
- **Camera-frame unit vectors** `(i, j, k)`: `i` = boresight (optical axis, index 0),
  `j` = image x / horizontal (index 1), `k` = image y / vertical (index 2).
- **Celestial unit vectors**: `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`;
  inverse `RA = atan2(y,x) mod 2π`, `Dec = arcsin(z)`.
- **Angular separation** between unit vectors at chord distance `d`: `angle = 2·arcsin(d/2)`
  (used everywhere in preference to `arccos` for small-angle conditioning).
- **Attitude** `R`: 3×3 rotation; row 0 is the boresight in the celestial frame (image-center
  RA/Dec). `RA=atan2(R[0,1],R[0,0])`, `Dec=atan2(R[0,2],‖R[1:3,2]‖)`, `Roll=atan2(R[1,2],R[2,2])`.
- **Diagonal FOV** = `fov · √(w²+h²)/w`.
- gRPC `ImageCoord` is `(x, y)`; the solver wants `(y, x)` — swap at the service boundary.

## 5. Rust architecture & dependency decisions

Cargo **workspace** of focused crates (one per feature, plus shared core):

| Crate | Feature | Responsibility |
|---|---|---|
| `ps-core` | `math-core` | geometry, projection, distortion, Wahba/SVD attitude, edge-ratio key + hashing, false-alarm test, residuals |
| `ps-detect` | `star-detection` | cedar-detect Rust pipeline (image → brightest-first `(y,x)` centroids) |
| `ps-db` | `pattern-database` | on-disk DB format, loader, star KD-tree, hash lookup + pre-filters |
| `ps-dbgen` | `database-generation` | offline catalog parse → pattern enumeration → DB serialization (CLI) |
| `ps-solve` | `plate-solver` | identification + attitude recovery engine |
| `ps-grpc` | `grpc-service` | tonic/prost `PlateSolver` server (ExtractCentroids/SolveFromCentroids/SolveFromImage/GetInfo) |
| `ps-mobile` | `mobile-runtime` | UniFFI bindings, mmap DB, perf budgets, iOS/Android packaging |

Default dependency choices (rationale in each feature's `design.md`; revisit per crate):

- **Linear algebra / SVD:** `nalgebra` (has `SVD`, fixed-size 3×3 matrices). Compute in **f64**;
  store DB vectors as **f32** (parity with reference dtype).
- **KD-tree:** a maintained crate (e.g. `kiddo`) for nearest / radius queries over unit vectors.
- **gRPC:** `tonic` + `prost` (matching cedar-detect's stack: tonic 0.11 / prost 0.12), `tonic-web`
  for gRPC-Web, `prost-types` for `Duration`; build via `tonic-build` in `build.rs`.
- **Image I/O:** `image` crate (cedar-detect uses 0.25); detection operates on 8-bit grayscale.
- **Memory-mapped DB:** `memmap2` (for narrow-FOV / too-big-for-RAM, linear-probe tables).
- **Parallelism:** optional `rayon`, feature-gated and bounded — **off / constrained on mobile**.
- **Mobile FFI:** `uniffi` (Rust ↔ Swift/Kotlin) as an alternative to an on-device network hop.
- **Magic constant:** `_MAGIC_RAND = 2654435761` (⌊2³²/φ⌋, Knuth multiplicative hash) for the
  quadratic-probe table index.

**Numerical parity is a correctness contract:** specs assert results match the Python reference
(tetra3/cedar) within stated tolerances (RA/Dec within arcseconds, centroids within ~±0.1 px,
identical matched catalog IDs). Keep the `2·arcsin(d/2)` angle convention, the same
`pattern_bins = round(1/(4·pattern_max_error))`, and the same false-alarm statistic everywhere
or tolerances won't line up.

## 6. Glossary

- **Centroid** — sub-pixel `(y,x)` location of a detected star.
- **Pattern** — a group of 4 stars used as a matching unit.
- **Pattern key / geometric hash** — the 5 sorted, quantized, normalized edge ratios of a pattern;
  the content-addressable, rotation- and scale-invariant fingerprint.
- **Edge** — angular distance between a pair of pattern stars (6 per pattern).
- **Largest edge** — max of the 6 edges; the normalizer and an angular-size (→FOV) proxy.
- **Hash table / pattern catalog** — open-addressed array mapping pattern-key hash → pattern.
- **Probing** — open-addressing collision resolution: quadratic (`hash+c²`) or linear (`hash+c`).
- **Verification star** — a catalog star kept densely for Stage-B match counting.
- **Attitude / rotation matrix** — 3-axis orientation relating camera and sky frames.
- **Boresight** — the camera optical axis direction (image center).
- **Roll** — rotation about the boresight relative to celestial north.
- **FOV** — field of view (horizontal unless stated); **diagonal FOV** = `fov·√(w²+h²)/w`.
- **Proper motion** — slow apparent stellar motion; catalogs give epoch position + annual rate.
- **Lost-in-space** — solving with no prior attitude (the mode implemented here).
- **Hot pixel** — defective sensor pixel bright in isolation; must not be mistaken for a star.
- **Cluster-buster** — density thinning so dense clusters (e.g. Pleiades) don't monopolize
  pattern generation/selection.

## 7. Conventions for authoring changes

- One OpenSpec **change** per feature, named `NN-<feature>`. Each carries four artifacts:
  `proposal.md`, `specs/<capability>/spec.md`, `design.md`, `tasks.md`.
- Specs use `### Requirement:` (SHALL/MUST) with `#### Scenario:` blocks (exactly four hashes;
  WHEN/THEN). Every requirement has ≥1 scenario; include parity scenarios where applicable.
- Each change must pass `openspec validate <change> --strict` and reach 4/4 in
  `openspec status --change <change>`.
- Cite the reference doc number behind each requirement; do not invent behavior not grounded in
  `reference-solutions/`.
