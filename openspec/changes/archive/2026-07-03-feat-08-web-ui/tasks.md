# Tasks: feat-08-web-ui

Retroactive change: the implementation already exists on branch
`claude/plate-solver-web-harness-gpmeji` (PR #1). Tasks 1–4 are the audit of each spec
requirement against the code and its tests; task 5 closes out validation.

The audit found the spec accurate (no divergences from the implementation) but several
scenarios untested. Closing those gaps surfaced one real bug, now fixed:
`parse_form`'s multipart-error handling hardcoded HTTP 400 for every read failure,
including the case where axum's own `DefaultBodyLimit` trips mid-read (which axum's
`MultipartError::status()` correctly classifies as 413) — an oversize request body was
therefore returning 400 instead of the spec's documented 413. Fixed by routing all
multipart read errors through `.status()` (`ps-web/src/solve.rs`).

## 1. HTTP API vs spec audit

- [x] 1.1 Verify the Health endpoint requirement against `ps-web/src/lib.rs::healthz`
      and `healthz_returns_ok_with_db_info` (fields, types, values from the loaded DB)
- [x] 1.2 Verify the solve request contract (required/optional fields, defaults,
      validation, unknown-field tolerance, timeout clamp) against
      `ps-web/src/solve.rs::parse_form` and `tests/solve_integration.rs`. Added
      `invalid_fov_estimate_returns_400` (negative/zero/non-numeric/NaN),
      `unknown_field_is_ignored`, and a `timeout_ms_clamps_to_max` unit test.
- [x] 1.3 Verify the solve response contract (status variants, match_found fields,
      degrees-valued matched-star ra/dec, sexagesimal formats, hints) against
      `map_response`, `ra_to_hms`/`dec_to_dms` unit tests, and
      `solve_reference_image_returns_match_found`. Added
      `map_response_covers_all_non_match_statuses`, unit-testing that `no_match`,
      `timeout`, `cancelled`, and `too_few` all map to a non-empty `hint` (the
      end-to-end forcing of a real solver timeout is intentionally not covered — see
      "Known gaps" below).
- [x] 1.4 Verify error mapping (400/413/415/500) and the 32 MiB body limit against
      `solve_handler`, `SOLVE_BODY_LIMIT`, and the 400/413/415 integration tests. Added
      `oversize_body_returns_413`, which caught the 400-vs-413 bug described above; the
      two 500 paths (solver panic, gate unavailable) remain untested — see "Known
      gaps".

## 2. Robustness vs spec audit

- [x] 2.1 Verify decode limits (20k px/side, 256 MiB alloc → 413) against
      `image_decode_limits`/`decode_image_bounded` and `oversize_image_returns_413`.
      Added `decode_alloc_cap_returns_413`, distinguishing the allocation-cap branch
      (error message names "memory") from the dimension-cap branch already covered.
- [x] 2.2 Verify solve serialization (single-permit gate acquired before decode,
      `spawn_blocking`) and cancel-on-disconnect (`CancelOnDrop`,
      `cancel_on_drop_trips_flag`). Added `solve_gate_allows_only_one_permit_at_a_time`,
      asserting the `Semaphore` directly (a full concurrent-HTTP-request race test was
      judged not worth the flakiness risk for what the `Semaphore` API already proves
      deterministically).

## 3. SPA serving vs spec audit

- [x] 3.1 Verify embedded serving scenarios (shell at `/`, asset mime types, 404 for
      missing dotted paths, SPA fallback, dist self-consistency) against
      `static_handler` and the `index_serves_spa_shell` /
      `assets_serve_with_correct_mime_and_exist` / `unknown_asset_404s` /
      `spa_fallback_serves_index` tests
- [x] 3.2 Verify the self-contained-binary scenario (binary serves with
      `frontend/dist` removed from disk) and that plain `cargo build` needs no node.
      Manually verified: `frontend/dist` renamed away after building the release
      binary, server still served `/` and `/assets/*` from the embedded bytes;
      `ps-web/Cargo.toml` has no `build.rs` and no node/npm build-dependency.

## 4. Browser UI vs spec audit

- [x] 4.1 Verify the browser solve workflow (upload/drag-drop + preview, FOV range
      hint from `/healthz`, advanced params omitted when empty, solving indicator,
      status/error rendering) against `ps-web/frontend/src/`. Covered by
      `frontend/e2e/smoke.mjs` (upload → solve → result) plus code reading; no
      unit/component test harness exists for the frontend (see "Known gaps").
- [x] 4.2 Verify the matched-star overlay (marker per star at natural-pixel coords,
      hover tooltip with cat id/mag/RA/Dec/px, cross-highlight with the table) — e2e:
      reference image at FOV 11 → 47 markers, RA ≈ 230.67 / Dec ≈ 11.04. Confirmed by
      `frontend/e2e/smoke.mjs`: ring count equals table row count (47 == 47), hover
      reveals a `HIP <id>` tooltip.
- [x] 4.3 Verify Aladin degradation (bounded script-load timeout → message + external
      "Open in Aladin" link at the solved position). Confirmed by
      `frontend/e2e/smoke.mjs` in this sandbox, where the Aladin CDN is genuinely
      unreachable: the fallback link is present and correctly targets the solved
      RA/Dec.

## 5. Validation & docs

- [x] 5.1 `openspec validate feat-08-web-ui --strict` exits 0
- [x] 5.2 Full cargo gate green (`cargo fmt --check`, `cargo clippy --workspace`,
      `cargo test --workspace`); spec-vs-implementation divergence found (400-vs-413 on
      oversize body) and fixed — see the note above task 1.
- [x] 5.3 Update `openspec/STATUS.md` to list the `web-ui` capability/change

## Known gaps (intentionally not force-tested)

Two scenarios remain without automated coverage, on the same basis as this repo's
existing "carried-forward gaps" in `openspec/STATUS.md`: forcing them deterministically
would require either a slow/flaky timing-based test or fault-injection scaffolding this
crate doesn't otherwise have.

- **`timeout` status end-to-end**: `map_response`'s mapping is unit-tested (task 1.3),
  but no test forces the real solver to run past `timeout_ms` and observes the HTTP
  response — would require either a slow test or a way to inject solver latency.
- **500 paths** (`solver panicked`, `solve gate unavailable`, `ps-web/src/solve.rs`):
  no test triggers an actual panic inside the `spawn_blocking` closure or a poisoned
  semaphore; both are defensive branches for conditions that don't occur in normal
  operation.
- **Frontend automated test harness**: `frontend/e2e/smoke.mjs` is a supplementary,
  manually-run Playwright script (documented in `ps-web/README.md`), not a `cargo
  test`-integrated or CI-run suite — this repo has no CI and no frontend test tooling
  (vitest/playwright config) committed. Standing this up is out of scope for this
  change.
