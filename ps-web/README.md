# ps-web

HTTP web API for the Plate Solver, built on [axum](https://docs.rs/axum).

This crate is currently a skeleton: it exposes a health check and a
placeholder index page. The `POST /api/solve` endpoint is a follow-up.

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
- `GET /` — placeholder index page.
