# ps-web

HTTP web API + browser UI for the Plate Solver, built on
[axum](https://docs.rs/axum) with a [Vite](https://vite.dev) + React +
Tailwind frontend.

## Running

```bash
cargo run -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz
```

By default the server listens on `127.0.0.1:8080`. Override with `--listen`:

```bash
cargo run -p ps-web -- --db <path> --listen 0.0.0.0:9000
```

`--db` accepts either a `.npz` tetra3 database (imported on load) or a native
`ps-db` file (loaded directly).

## Routes

- `GET /healthz` — JSON status and database properties.
- `GET /` — browser UI (React SPA): drag-and-drop upload, solution stat
  cards, matched-star overlay on the uploaded image, Aladin Lite sky view.
- `POST /api/solve` — multipart image + FOV estimate to JSON plate solution.
- Everything else — embedded SPA assets, with an SPA fallback to `index.html`
  for extension-less paths.

## Frontend (`frontend/`)

The UI is a Vite + React + Tailwind app. Its production build
(`frontend/dist/`) is **committed to git** and embedded into the server
binary with `rust-embed`, so plain `cargo build`/`cargo test` never needs
node and a clean checkout produces a fully self-contained executable.

Development loop (hot reload against the real solver):

```bash
# terminal 1 — backend
cargo run -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz

# terminal 2 — frontend dev server on :5173, proxying /api + /healthz to :8080
cd ps-web/frontend
npm install
npm run dev
```

**If you change anything under `frontend/src/`** (or `index.html`, config,
etc.), rebuild the bundle and commit the resulting `dist/` changes together
with your source change:

```bash
cd ps-web/frontend && npm run build
```

The `assets_serve_with_correct_mime_and_exist` test in `src/lib.rs` fails if
the committed `dist/` is internally inconsistent (index.html referencing
assets that don't exist).

### Browser smoke check (supplementary, not part of the cargo gate)

`frontend/e2e/smoke.mjs` drives a real browser against a running server and
verifies the matched-star overlay, hover tooltip, and Aladin fallback link —
the behavior that has no `cargo test` coverage since it lives entirely in the
React frontend. This repo has no CI (`cargo fmt`/`clippy`/`test` is the whole
local gate); run the smoke check manually after frontend changes:

```bash
cargo run --release -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz &
node frontend/e2e/smoke.mjs
```

Requires Playwright with a reachable Chromium (see the script header for the
`PLAYWRIGHT_CHROMIUM_PATH` override on machines other than this dev
container).

## Browser workflow

1. Start the server:
   ```bash
   cargo run -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz
   ```
2. Open `http://127.0.0.1:8080/` in a browser.
3. Drop in the reference image
   `reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg`,
   enter `11` for the FOV estimate, and click **Solve**.
4. Expect a match with RA ≈ `230.67°` and Dec ≈ `11.04°`: hero cards for
   RA/Dec, stat tiles, your image with all 47 matched stars ringed (hover a
   ring or a table row for catalog details), and an interactive Aladin Lite
   sky view centered on the solved position (falls back to a plain "Open in
   Aladin" link if the CDN script can't load).

### `POST /api/solve`

Accepts `multipart/form-data`:

- `image` (required) — JPEG or PNG file.
- `fov_estimate` (required) — estimated field of view, in degrees.
- `fov_max_error` (optional) — max FOV error tolerance, in degrees.
- `match_radius` (optional, default `0.01`) — match radius as a fraction of image width.
- `match_threshold` (optional, default `1e-5`) — false-alarm probability threshold.
- `timeout_ms` (optional, default `30000`, clamped to `60000`) — solve timeout in milliseconds.
- `distortion` (optional) — fixed radial distortion coefficient; omit to estimate it.

The response is always HTTP 200 for a completed solve attempt, with a `status`
field of `match_found`, `no_match`, `timeout`, `cancelled`, or `too_few`.
Non-`match_found` responses include a human-readable `hint`. Malformed
requests return 400, an oversize request body or an image past the decode
dimension/allocation limits returns 413, undecodable images return 415, and
solver panics return 500 — all with a `{"error": "..."}` body.

```bash
curl -F image=@reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg -F fov_estimate=11 http://127.0.0.1:8080/api/solve
```
