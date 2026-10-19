# finder: the finder console

A local web page for the plate solver. Give it a frame (one of cedar's sample frames, or
your own) and it shows where the frame points (RA/Dec, the constellation, the field of
view), the catalog stars it matched drawn on the frame as proof, the Messier objects in and
near the field, and objects like them that are up now.

It is the start of the electronic finder: the same program will later take the live camera
stream, talk to SkySafari and the mount, and give an AI assistant the console's actions.

## Run it

From the repository root:

```sh
cargo run --release -p finder
```

Then open <http://127.0.0.1:8080>. The defaults are cedar's 10–30° database and its 21
sample frames; for your own camera, pass a database built for its field of view (see the
main README):

```sh
cargo run --release -p finder -- --db my_camera.npz --samples ~/frames
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--db PATH` | cedar's `default_database.npz` | Pattern database covering your camera's field of view. |
| `--samples DIR` | cedar's `medium_fov` samples | Folder of frames to offer on the page. |
| `--listen ADDR` | `127.0.0.1:8080` | Address to serve on. `0.0.0.0:8080` lets other devices connect. |

### From a phone

The page is built for a phone at the telescope. Two ways to use one:

- **The Surface serves, the phone browses.** Run with `--listen 0.0.0.0:8080` and open
  `http://<the Surface's address>:8080` on the phone, on the same Wi-Fi. Under WSL2 the
  server is only reachable from Windows itself until WSL uses mirrored networking: add
  `networkingMode=mirrored` under `[wsl2]` in `%UserProfile%\.wslconfig`, then run
  `wsl --shutdown` and start it again. Windows may ask to allow the port through its
  firewall.
- **The phone does everything (Android).** In [Termux](https://termux.dev), install Rust
  and git (`pkg install rust git`), clone the repository, and run the same `cargo run`
  command; the first build takes a while on a phone. Then open
  <http://127.0.0.1:8080> in the phone's browser. The server needs about 160 MB of memory.

Location ("what is up now") is entered by hand on the page. Browsers only share GPS
location with pages served over HTTPS or from the same device, so "use this device's
location" works when the phone runs the server itself.

## Night mode

Night mode turns the whole page, frames and sky survey included, into dim red, to keep your
eyes dark-adapted. It is a toggle in the top bar and the page remembers it.

## How it works

- `src/solve.rs` runs one frame the way the `solve_image` example does (detect with
  cedar-detect's settings, solve from the brightest 30 stars), then places the matched stars,
  the brightest unmatched catalog stars and the Messier objects on the frame. Placement uses
  the solver's own attitude and lens model: a test checks that every matched star's catalog
  position lands back on the star it was matched to.
- `src/constellation.rs` finds the constellation from the IAU boundaries, after precessing
  to their B1875 equinox. `src/catalog.rs` holds the Messier list and ranks "something like
  this" by kind, then by whether it is up for your site, then by how far you would slew.
- `src/api.rs` serves the JSON API and the page. One solve runs at a time; uploads are
  capped at 32 MB and decoded with size limits.
- The page is a React + TypeScript + Vite + Tailwind app in `web/`. Its production build,
  `web/dist/`, is committed and compiled into the binary, so `cargo` and CI never need
  Node.

The data compiled into the binary (the Messier list from OpenNGC, the constellation
boundaries) keeps its own licenses: see [`data/README.md`](data/README.md).

## Change the page

```sh
cd crates/finder/web
npm install
npm run dev     # http://localhost:5173, hot reload; run `cargo run -p finder` alongside it
npm run build   # writes web/dist; commit it with your change
```

The page uses the API types in `web/src/api.ts`, which mirror the Rust structs in
`src/solve.rs` and `src/catalog.rs`. Change both together.

## API

| Route | Answer |
|-------|--------|
| `GET /api/health` | The database's properties and how many samples there are. |
| `GET /api/samples` | The sample frames: `[{name, label}]`. |
| `GET /api/samples/{name}` | A sample frame's file. `…/thumb` is a 320-pixel JPEG preview. |
| `POST /api/solve` | Multipart: an `image` file or a `sample` name, and optionally `fov`, `fov_max_error`, `distortion`, `bin`, `sigma`, `stars`, `timeout_ms` (the `solve_image` flags). Answers a `SolveReport`. |
| `GET /api/similar?id=M42&lat=&lon=&time=` | Objects like `id`, best first. With `lat`/`lon` (degrees, east positive) and optional Unix `time`, each has its altitude and whether it is up. |
