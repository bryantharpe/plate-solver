# Mobile Front-End Specs: author the OpenSpec changes for a live star-identifying Android app (feat-09-mobile-app + feat-07 extension)

_Authored 2026-07-03. This file is the ONLY durable state for this loop. Every iteration re-reads it from disk; it must be self-sufficient for a cold restart._

## Purpose

**What:** Author and strict-validate the OpenSpec artifacts for the phone app that identifies what
the camera is pointed at: (a) **extend `feat-07-mobile-runtime` in place** with the runtime contract
the app needs — a repeated-solve/streaming session API over a reused DB handle with cooperative
cancellation; detection-params threading through `solve_from_image` (fixes CODEBASE-REVIEW **C2**,
`CODEBASE-REVIEW.md:52-68`); the **C1** lazy-combinations fix (`CODEBASE-REVIEW.md:28-50`,
`notes/C1-lazy-combinations-fix.md` — the ~618 MiB allocation that would OOM any phone) promoted to
a spec requirement; and a frame-ingestion contract (YUV_420_888 → 8-bit grayscale + binning at the
boundary, per `openspec/project.md` conventions) — and (b) **create `feat-09-mobile-app`**
(capability `mobile-app`): an Android-first Kotlin / Jetpack Compose / CameraX app — camera pipeline
with pixel binning + manual exposure/ISO (Camera2 interop), a ~1–2 Hz live lost-in-space solve loop
on binned frames, a gyro/rotation-vector-fused overlay (matched-star markers + brightest-star names
+ center-crosshair RA/Dec readout + solve-status HUD; constellation lines as a stretch requirement),
bundled wide-FOV database provisioning (assets → app storage for mmap, offline-first),
permissions/lifecycle, and a thin-shell multi-platform layering rule (all shared logic in Rust
behind UniFFI; iOS later is a thin SwiftUI shell). Reference device: **Google Pixel 10 Pro XL**.
Primary lens path: **main camera + a new wide-FOV (~50–75°) database** generated via the existing
`ps-dbgen` (feat-04); telephoto documented as a supported alt-config. **Docs/specs only — no
Rust/Kotlin implementation code is written in this loop.**

**Why:** the pipeline was ported to Rust precisely so it could run fast enough for a live overlay
(reference: ~10 ms solve + ~25 ms extract, `openspec/PRD.md:91-92`); these specs make that product
buildable against a validated contract, exactly as feat-01–06 were before implementation. Live
tracking-mode solving stays a solver non-goal (`openspec/PRD.md:60`) — smoothness comes from
app-level sensor fusion, so the archived solver specs are not reopened.

**Definition of done (observable):** `OPENSPEC_TELEMETRY=0 openspec validate
feat-07-mobile-runtime --strict` **and** `OPENSPEC_TELEMETRY=0 openspec validate
feat-09-mobile-app --strict` both exit 0; `openspec status --change feat-09-mobile-app` shows
**4/4 artifacts**; `openspec/STATUS.md` lists both changes in its active-changes table; **all task
checkboxes below are `[x]`**; every commit pushed on `spec/mobile-frontend`.

## Loop Protocol

1. Re-read this file from disk (it is the only durable state). Read Purpose (stop condition), all
   Guardrails, the task list, and the Decisions/Blocked logs.
2. Pick the first task whose checkbox is unchecked AND whose deps are all checked.
3. Implement it fully in-session to its acceptance criteria. Delegate heavy documentation reading
   (web pages, long reference files) to subagents; keep only distilled findings in the main loop.
4. Run the task's integrity gate (see Guardrails). Then mark the checkbox `[x]`, append a Run Log
   entry, and commit with the task's exact commit message (staging only in-scope paths, work + this
   plan file together).
5. Push to `origin/spec/mobile-frontend` each iteration.
6. Stop condition: if all checkboxes are `[x]`, run the final `--strict` sweep (D2), ensure the PR
   exists, and exit. If the selected task is blocked, mark it `BLOCKED`, append to the Blocked Log,
   and continue to the next unblocked task. Otherwise loop to step 1.

## Guardrails (apply every iteration)

- **Git:** branch is `spec/mobile-frontend` (branched from `claude/plate-solver-web-harness-gpmeji`).
  Commit after every task with its specified message; push every iteration (`git push -u origin
  spec/mobile-frontend`). Never commit to `main`. Never force-push. Stage only in-scope paths —
  `openspec/`, `notes/mobile/`, `plan-mobile-specs.md` — never `git add -A`.
- **Integrity gates:** before every commit that touches an OpenSpec change, run
  `OPENSPEC_TELEMETRY=0 openspec validate <change> --strict` (must exit 0); from task C4 onward
  also `openspec status --change feat-09-mobile-app` (must show 4/4). Scenarios MUST use exactly
  four hashes (`#### Scenario:`); every requirement MUST have ≥1 scenario; use SHALL/MUST. **Never
  weaken a gate:** never drop `--strict`, never delete/abridge requirements or scenarios to pass
  validation — fix the content instead.
- **Data/scope discipline:** write only under `openspec/`, `notes/mobile/`, and this plan file.
  **Specs/docs only — write no Rust, Kotlin, or build files.** `reference-solutions/` is read-only.
  Do not run `openspec archive` (changes stay active for review). Do not modify archived specs
  (`openspec/specs/*` except via STATUS.md notes), feat-08 artifacts, or `ps-*` crate code. The
  solver tracking-mode non-goal (`PRD.md:60`) must not be contradicted — sensor fusion is specced
  at the app layer only.
- **Cost controls:** web research is bounded — ≤ ~5 pages per Phase-A topic, fetched/distilled by a
  subagent, kept as a cited note; cite the note from specs rather than re-fetching. Max 3
  validation-fix attempts per task → BLOCK. No paid APIs.
- **Don't-stall:** if a task can't reach its AC, mark it `BLOCKED`, append to the Blocked Log
  (reason + recommended fix), and move to the next unblocked task. The loop never spins on one task.
- **Context hygiene:** this file is the only durable state; one task per iteration; commit work +
  this file together before ending an iteration. Re-read on restart. This loop runs **fully
  autonomously**: record every non-obvious choice in the Decisions Log and proceed on the most
  defensible default rather than stalling for a human.

## Tasks

### Phase A — Docs research (grounding notes under `notes/mobile/`)

- [ ] A1 — CameraX/Camera2 capture note — deps: none — Research and write `notes/mobile/camera.md`: CameraX `ImageAnalysis` pipeline; Camera2 interop (`Camera2CameraControl`/`CaptureRequestOptions`) for manual exposure time, ISO, and frame duration; `MANUAL_SENSOR` capability and how to query it; sensor pixel-binning modes on modern sensors (and what the Pixel 10 Pro XL main sensor supports); YUV_420_888 → 8-bit grayscale conversion (Y-plane extraction, row stride handling); maximum practical exposure per frame in a live pipeline. — AC: `notes/mobile/camera.md` exists, covers every listed topic with concrete API class/method names and ≥1 cited URL per topic. — commit: "docs(notes): CameraX/Camera2 capture research for mobile app"
- [ ] A2 — Sensors/fusion note — deps: none — Research and write `notes/mobile/sensors.md`: `TYPE_ROTATION_VECTOR` and gyroscope APIs; coordinate-frame chain (device/sensor frame → ENU via `getRotationMatrixFromVector` → equatorial RA/Dec given GPS position + time, incl. sidereal-time conversion); typical gyro drift rates and rotation-vector accuracy; a concrete dead-reckoning scheme for updating the overlay between ~1–2 Hz solves (anchor overlay to last solve attitude, integrate relative rotation since). — AC: note exists with the full frame-conversion chain written out and ≥1 citation per topic. — commit: "docs(notes): sensor-fusion research for overlay dead-reckoning"
- [ ] A3 — UniFFI + packaging note — deps: none — Research and write `notes/mobile/uniffi-packaging.md`: UniFFI Kotlin binding generation; patterns for a long-lived Rust object (DB handle/solve session) exposed to Kotlin; callback-interface vs polling vs suspend-function options for streaming solve results; cancellation propagation; `cargo-ndk` + `.aar` packaging flow; shipping a DB in app assets and copying to app storage on first run for mmap. — AC: note exists covering every listed topic with concrete API/tool names and ≥1 citation per topic. — commit: "docs(notes): UniFFI streaming + Android packaging research"
- [ ] A4 — FOV/DB feasibility note — deps: none — Research and write `notes/mobile/fov-database.md`: Pixel 10 Pro XL lens FOVs (main/ultrawide/tele, diagonal degrees) and apertures; how star-count-per-FOV and magnitude cutoff scale to a ~50–75° FOV database; a concrete recommended `ps-dbgen` parameter set (max_fov, star catalog magnitude limit, pattern budget, expected DB size) derived from `openspec/specs/database-generation/spec.md` + `reference-solutions/docs` conventions; feat-04's count-parity gap noted as a caveat; prior art (Cedar Aim, Stellarium mobile AR alignment) for live-overlay UX expectations. — AC: note exists and includes one explicit recommended dbgen parameter set with expected DB size. — commit: "docs(notes): wide-FOV database feasibility study"

### Phase B — feat-07-mobile-runtime extension (app-facing runtime contract)

- [ ] B1 — feat-07 proposal + design revision — deps: A3, A4 — Revise `openspec/changes/feat-07-mobile-runtime/proposal.md` and `design.md`: add the streaming/repeated-solve session to What Changes (remove "streaming/video solving (post-v1)" from non-goals; keep solver tracking-mode excluded); add detection-params threading (C2) and the C1 lazy-combinations fix as in-scope requirements with rationale; record the wide-FOV-DB + bundled-delivery decision (citing `notes/mobile/fov-database.md`); update Impact (ps-solve param-threading touch, ps-dbgen DB generation). — AC: `OPENSPEC_TELEMETRY=0 openspec validate feat-07-mobile-runtime --strict` exits 0. — commit: "docs(openspec): feat-07 proposal/design — streaming session, params threading, C1/C2"
- [ ] B2 — feat-07 spec deltas — deps: B1 — Revise `openspec/changes/feat-07-mobile-runtime/specs/mobile-runtime/spec.md`: ADD requirements for (1) a solve-session API — create once from a DB handle, submit successive frames, receive results, cooperative cancel between and during solves; (2) detection-params threading — binning (1/2/4/8), sigma, normalize_rows, detect_hot_pixels accepted per solve and honored end-to-end (closes C2); (3) bounded solver memory — the solve path SHALL NOT materialize the full 4-combination set (C1), with a peak-allocation scenario; (4) frame ingestion — YUV_420_888 Y-plane (with row stride) → 8-bit grayscale contract. Each requirement ≥1 four-hash scenario. — AC: `--strict` exits 0 and the four new requirement areas each have ≥1 scenario. — commit: "docs(openspec): feat-07 spec deltas for app-facing runtime contract"
- [ ] B3 — feat-07 tasks revision — deps: B2 — Revise `openspec/changes/feat-07-mobile-runtime/tasks.md`: add dependency-ordered tasks for the C1 fix (per `notes/C1-lazy-combinations-fix.md`, gated by the existing sv6 parity tests), C2 params threading (ps-solve signature + ps-grpc plumb-through), the session API, frame ingestion, and wide-FOV DB generation + bundling; keep existing tasks; renumber coherently. — AC: `--strict` exits 0. — commit: "docs(openspec): feat-07 tasks for streaming/params/C1"

### Phase C — feat-09-mobile-app (new change, capability `mobile-app`)

- [ ] C1 — feat-09 proposal — deps: B2 — `openspec new change "feat-09-mobile-app"`, then author `proposal.md`: why (the live star-identification app is the product the Rust port was for), what changes (new capability `mobile-app`), capabilities section (new: `mobile-app`; modified: none — runtime needs land via feat-07), impact (new Android app module consuming ps-mobile via UniFFI; no server components). — AC: `--strict` exits 0 for feat-09-mobile-app. — commit: "docs(openspec): feat-09-mobile-app proposal"
- [ ] C2 — feat-09 design — deps: C1 — Author `design.md`: stack decision record (Kotlin/Compose/CameraX + Camera2 interop; thin-shell layering rule — all shared logic in Rust, UI shells per platform, iOS = later SwiftUI shell); camera pipeline design (binning choice logic, manual-exposure strategy, frame cadence, citing `notes/mobile/camera.md`); solve-loop threading (single in-flight solve, frame dropping, cancellation on pause); sensor-fusion design (anchor + dead-reckon, citing `notes/mobile/sensors.md`); overlay rendering approach; DB provisioning (bundled asset → storage → mmap, citing `notes/mobile/fov-database.md`); budget philosophy — numbers-with-tests anchored to Pixel 10 Pro XL, placeholders resolved at implementation; risks (C1/C2 upstream, wide-FOV DB unproven count-parity, gyro drift). — AC: `--strict` exits 0. — commit: "docs(openspec): feat-09-mobile-app design"
- [ ] C3 — feat-09 spec — deps: C2 — Author `specs/mobile-app/spec.md` with requirements + four-hash scenarios covering: camera pipeline (main-lens capture, binning 1/2/4/8 selection, manual exposure/ISO, MANUAL_SENSOR runtime check with graceful degradation, YUV→gray); live solve loop (~1–2 Hz target on reference device, single in-flight solve, frame dropping, lifecycle pause/resume, cancellation); overlay (markers at matched-star positions, brightest-star names, center-crosshair RA/Dec readout, solve-status HUD incl. solve time + FOV, gyro dead-reckoning between solves, re-anchor on each match); stretch requirement: constellation lines (non-blocking); DB provisioning (bundled wide-FOV DB, first-run copy, mmap open, telephoto alt-config with 10–30° DB documented); permissions (camera, location for RA/Dec conversion) + fully-offline operation; multi-platform layering rule (Rust core owns all solve/fusion math exposed via UniFFI; platform shell owns only capture/UI). — AC: `--strict` exits 0; every requirement has ≥1 scenario. — commit: "docs(openspec): feat-09-mobile-app spec — camera, live loop, overlay, provisioning"
- [ ] C4 — feat-09 tasks — deps: C3 — Author `tasks.md`: dependency-ordered implementation tasks (Rust session-consumer glue, Android module scaffold, camera pipeline, solve loop, fusion, overlay UI, DB bundling, on-device budget measurement tasks that resolve the placeholders), each with AC; note Android-toolchain-required tasks as such. — AC: `--strict` exits 0 AND `openspec status --change feat-09-mobile-app` shows 4/4 artifacts. — commit: "docs(openspec): feat-09-mobile-app tasks"

### Phase D — Close-out

- [ ] D1 — STATUS.md + consistency pass — deps: B3, C4 — Update `openspec/STATUS.md`: active-changes table gains feat-09 (and feat-07's revised req/task counts); note the mobile spec set + wide-FOV DB decision in the header narrative; verify no contradiction with archived specs or PRD non-goals (tracking mode stays excluded — fusion is app-level; cite where the specs say so). — AC: `--strict` still exits 0 for both changes; STATUS.md lists both with correct counts. — commit: "docs(openspec): STATUS.md — mobile front-end spec changes queued"
- [ ] D2 — Final sweep + PR — deps: D1 — Run the full gate: `--strict` on feat-07-mobile-runtime and feat-09-mobile-app (both exit 0), `openspec status` 4/4 for feat-09; push; open a PR for `spec/mobile-frontend` titled "specs: mobile front end — feat-09-mobile-app + feat-07 runtime extension" summarizing both changes and linking the notes. — AC: sweep exits 0; PR exists and is open. — commit: "docs(openspec): mobile front-end spec set complete"

## Decisions Log

- (append-only)
- 2026-07-03 — UI stack: native Kotlin + Jetpack Compose + CameraX (Camera2 interop); all shared logic in Rust via UniFFI; iOS later as thin SwiftUI shell — best camera/sensor access; Rust core is the cross-platform layer (user-confirmed).
- 2026-07-03 — Live model: ~1–2 Hz lost-in-space solves on binned frames + gyro/rotation-vector dead-reckoning between solves; no solver tracking mode — keeps archived solver specs closed; PRD non-goal preserved (user-confirmed).
- 2026-07-03 — Spec shape: new feat-09-mobile-app + in-place extension of feat-07-mobile-runtime — clean layering; feat-07 is active/unimplemented so editing is cheap (user-confirmed).
- 2026-07-03 — Scope: specs only; implementation gets its own forge plan later (user-confirmed).
- 2026-07-03 — Lens/DB: main camera + new wide-FOV (~50–75°) DB via ps-dbgen; telephoto as alt-config — main lens has the light-gathering stars need (user-confirmed).
- 2026-07-03 — Reference device: Google Pixel 10 Pro XL (user's phone) — budgets/capabilities anchored to it (user-confirmed).
- 2026-07-03 — Branch: spec/mobile-frontend off claude/plate-solver-web-harness-gpmeji; new PR (user-confirmed).
- 2026-07-03 — Overlay v1: markers + brightest-star names + RA/Dec crosshair + status HUD; constellation lines stretch (user-confirmed).
- 2026-07-03 — DB delivery: bundled in app assets, first-run copy to storage for mmap; offline-first (user-confirmed).
- 2026-07-03 — Autonomy: loop runs fully autonomous with Decisions Log; BLOCK only when stuck (user-confirmed).

## Blocked Log

- (append-only)

## Run Log

- (append-only)
- 2026-07-03 — plan authored — 13 tasks queued (A1–A4, B1–B3, C1–C4, D1–D2); loop starts at A1.
