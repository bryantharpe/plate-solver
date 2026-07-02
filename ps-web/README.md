# ps-web

HTTP web API for the Plate Solver, built on [axum](https://docs.rs/axum).

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
- `GET /` — browser UI: upload form, solution display, Aladin Lite sky view.
- `POST /api/solve` — multipart image + FOV estimate to JSON plate solution.

## Browser workflow

1. Start the server:
   ```bash
   cargo run -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz
   ```
2. Open `http://127.0.0.1:8080/` in a browser.
3. Choose the reference image
   `reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg`,
   enter `11` for the FOV estimate, and click **Solve**.
4. Expect a `match_found` result with RA ≈ `230.67°` and Dec ≈ `11.04°`,
   a matched-stars table, and an interactive Aladin Lite sky view centered on
   the solved position (falls back to a plain "Open in Aladin" link if the
   CDN script can't load).

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
requests return 400, undecodable images return 415, and solver panics
return 500 — all with a `{"error": "..."}` body.

```bash
curl -F image=@reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg -F fov_estimate=11 http://127.0.0.1:8080/api/solve
```
