# Glossary

- **Plate solving** — determining where a camera was pointing on the sky
  from an image of stars alone.
- **Lost-in-space** — the hard variant of plate solving: no prior attitude
  estimate, only the image and (optionally) a rough field-of-view guess.
- **Boresight** — the sky direction of the image center; a solution reports
  it as right ascension (RA) and declination (Dec), plus roll.
- **Centroid** — the sub-pixel `(y, x)` position of a detected star.
  Centroids follow tetra3's `(y, x)` convention throughout the pipeline,
  with `(0.5, 0.5)` at the center of the top-left pixel.
- **Pattern** — four stars considered together. Its six pairwise angular
  edges, with the five smaller ones normalized by the largest, form a
  rotation- and scale-invariant description.
- **Pattern key** — those five normalized edge ratios quantized into bins;
  hashed to locate candidate patterns in the database.
- **Pattern database** — the precomputed hash table of sky patterns plus the
  star table, built offline from a star catalog (`tetra3-gen-db`), stored in
  tetra3/cedar's `.npz` format.
- **Pattern-checking stars** — the brightest N detected stars (default 8)
  from which image patterns are formed; verification still uses all
  detected stars.
- **Candidate** — a catalog pattern whose key falls within tolerance of an
  image pattern's key; cheap filters (16-bit hash, largest-edge/FOV band,
  edge-ratio band) run before expensive verification.
- **Verification** — the authoritative accept/reject: solve the attitude
  from the four pattern pairs, project nearby catalog stars into the image,
  match them to centroids, and accept only if the binomial false-alarm
  probability clears the Bonferroni-corrected threshold.
- **Refinement** — after acceptance: re-fit attitude over all matches,
  refine FOV (and optionally distortion), and compute residuals
  (RMSE/P90E/MAXE, arcseconds).
- **References / oracle** — the upstream tetra3, cedar-solve, and
  cedar-detect checkouts fetched by `scripts/fetch-references.sh` into the
  gitignored `references/`; parity tests and benchmarks treat their outputs
  as ground truth. Never vendored.
- **v1** — the first implementation of this project, preserved in git
  history; the current tree is a from-scratch rewrite built against
  `openspec/` with v1 uninvolved except as fixtures.
