# Tasks: feat-08-web-ui

Retroactive change: the implementation already exists on branch
`claude/plate-solver-web-harness-gpmeji` (PR #1). Tasks 1–4 are the audit of each spec
requirement against the code and its tests; task 5 closes out validation.

## 1. HTTP API vs spec audit

- [ ] 1.1 Verify the Health endpoint requirement against `ps-web/src/lib.rs::healthz`
      and `healthz_returns_ok_with_db_info` (fields, types, values from the loaded DB)
- [ ] 1.2 Verify the solve request contract (required/optional fields, defaults,
      validation, unknown-field tolerance, timeout clamp) against
      `ps-web/src/solve.rs::parse_form` and `tests/solve_integration.rs`
- [ ] 1.3 Verify the solve response contract (status variants, match_found fields,
      degrees-valued matched-star ra/dec, sexagesimal formats, hints) against
      `map_response`, `ra_to_hms`/`dec_to_dms` unit tests, and
      `solve_reference_image_returns_match_found`
- [ ] 1.4 Verify error mapping (400/413/415/500) and the 32 MiB body limit against
      `solve_handler`, `SOLVE_BODY_LIMIT`, and the 400/413/415 integration tests

## 2. Robustness vs spec audit

- [ ] 2.1 Verify decode limits (20k px/side, 256 MiB alloc → 413) against
      `image_decode_limits`/`decode_image_bounded` and `oversize_image_returns_413`
- [ ] 2.2 Verify solve serialization (single-permit gate acquired before decode,
      `spawn_blocking`) and cancel-on-disconnect (`CancelOnDrop`,
      `cancel_on_drop_trips_flag`)

## 3. SPA serving vs spec audit

- [ ] 3.1 Verify embedded serving scenarios (shell at `/`, asset mime types, 404 for
      missing dotted paths, SPA fallback, dist self-consistency) against
      `static_handler` and the `index_serves_spa_shell` /
      `assets_serve_with_correct_mime_and_exist` / `unknown_asset_404s` /
      `spa_fallback_serves_index` tests
- [ ] 3.2 Verify the self-contained-binary scenario (binary serves with
      `frontend/dist` removed from disk) and that plain `cargo build` needs no node

## 4. Browser UI vs spec audit

- [ ] 4.1 Verify the browser solve workflow (upload/drag-drop + preview, FOV range
      hint from `/healthz`, advanced params omitted when empty, solving indicator,
      status/error rendering) against `ps-web/frontend/src/`
- [ ] 4.2 Verify the matched-star overlay (marker per star at natural-pixel coords,
      hover tooltip with cat id/mag/RA/Dec/px, cross-highlight with the table) — e2e:
      reference image at FOV 11 → 47 markers, RA ≈ 230.67 / Dec ≈ 11.04
- [ ] 4.3 Verify Aladin degradation (bounded script-load timeout → message + external
      "Open in Aladin" link at the solved position)

## 5. Validation & docs

- [ ] 5.1 `openspec validate feat-08-web-ui --strict` exits 0
- [ ] 5.2 Full cargo gate green (`cargo fmt --check`, `cargo clippy --workspace`,
      `cargo test --workspace`); note any spec-vs-implementation divergences found in
      the audit and fix or record them
- [ ] 5.3 Update `openspec/STATUS.md` to list the `web-ui` capability/change
