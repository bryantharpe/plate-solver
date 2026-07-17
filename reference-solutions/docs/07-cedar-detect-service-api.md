# 07 — cedar-detect Service API (gRPC) & Integration

cedar-detect's algorithm is Rust (doc 04). To use it from Python (or any language), it runs
as a **gRPC microservice**: send an image, get back centroids. This document specifies the
wire contract, the shared-memory fast path, the server binary, and the two Python clients —
enough to reimplement the service or a client.

```
   Python solver (cedar-solve)                         Rust server (cedar-detect-server)
   ┌──────────────────────────┐   ExtractCentroids    ┌──────────────────────────────────┐
   │ CedarDetectClient        │ ───────RPC──────────► │ MyCedarDetect.extract_centroids   │
   │  - spawns server subproc │   (image in proto     │  estimate_noise → get_stars_from  │
   │  - shared memory image   │    OR /dev/shm)       │  _image → StarDescription[]       │
   │  - returns [(y,x),...]   │ ◄──CentroidsResult─── │                                    │
   └──────────────────────────┘                       └──────────────────────────────────┘
        TCP 127.0.0.1:50051 (also serves gRPC-Web over HTTP/1)
```

---

## 1. The proto contract (`cedar-detect/src/proto/cedar_detect.proto`)

`syntax = "proto3"; package cedar_detect;` imports `google/protobuf/duration.proto`.

### Service

```proto
service CedarDetect {
  // Returns INTERNAL error if the request's shared memory cannot be accessed
  // (the client's signal to fall back to inline image bytes).
  rpc ExtractCentroids(CentroidsRequest) returns (CentroidsResult);
}
```

### `CentroidsRequest`

| # | field | type | meaning |
|---|---|---|---|
| 1 | `input_image` | `Image` | image to analyze; the 3 leftmost/rightmost columns are skipped |
| 2 | `sigma` | `double` | significance threshold (pixel must exceed background by `sigma·noise`); typical 5–10 |
| 3 | `max_size` | `int32` | **deprecated** |
| 8 | `binning` | `optional int32` | 2 or 4 (default 2) when binning is requested |
| 4 | `return_binned` | `bool` | also return the 2×2/4×4 binned image (hot pixels removed) |
| 5 | `use_binned_for_star_candidates` | `bool` | detect on an internally-binned image (for oversampled/soft optics); centroids still in full-res coords |
| 6 | `detect_hot_pixels` | `bool` | classify & reject isolated hot pixels (else they may appear as stars) |
| 9 | `normalize_rows` | `bool` | equalize per-row dark levels (IMX296 fix) |
| 7 | `estimate_background_region` | `optional Rectangle` | sub-region for background estimation |

(Field numbers are intentionally non-sequential; 8/9 were added later.)

### `Image`

| # | field | type | meaning |
|---|---|---|---|
| 1 | `width` | `int32` | |
| 2 | `height` | `int32` | |
| 3 | `image_data` | `bytes` | row-major **uint8 grayscale**; omitted if `shmem_name` set |
| 4 | `shmem_name` | `optional string` | name of a POSIX `shm_open()` object holding the pixels |
| 5 | `reopen_shmem` | `bool` | server must reopen `shmem_name` (client re-created it at a new size) |

### `Rectangle`: `origin_x, origin_y, width, height` (all `int32`).

### `CentroidsResult`

| # | field | type | meaning |
|---|---|---|---|
| 1 | `noise_estimate` | `double` | RMS noise of `input_image` |
| 7 | `background_estimate` | `optional double` | background of `estimate_background_region` |
| 2 | `hot_pixel_count` | `int32` | hot pixels seen |
| 6 | `peak_star_pixel` | `int32` | peak pixel of candidates (avg of brightest N) |
| 3 | `star_candidates` | `repeated StarCentroid` | **brightest first** |
| 4 | `binned_image` | `optional Image` | present iff `return_binned` |
| 5 | `algorithm_time` | `Duration` | time inside the algorithm (subtract from RPC time → overhead) |

### `StarCentroid`

| # | field | type | meaning |
|---|---|---|---|
| 1 | `centroid_position` | `ImageCoord` | full-resolution image coords |
| 4 | `brightness` | `double` | background-subtracted sum of the star's pixels |
| 6 | `num_saturated` | `int32` | count of saturated pixels |

### `ImageCoord`: `x` (`double`), `y` (`double`). **`(0.5, 0.5)` = center of the upper-left pixel.**

> **Coordinate convention reminder:** `ImageCoord` is `(x, y)` but the solver wants `(y, x)`.
> Clients convert: `tetra_centroid = (sc.centroid_position.y, sc.centroid_position.x)`.

Compile the stubs: `python -m grpc_tools.protoc -I../src/proto --python_out=. --pyi_out=.
--grpc_python_out=. ../src/proto/cedar_detect.proto` (or cedar-solve's
`scripts/compile_proto.py`).

---

## 2. Transport & the shared-memory fast path

- Transport is **TCP on `127.0.0.1:50051`** (configurable `--port`). The server also accepts
  **gRPC-Web over HTTP/1** (`.accept_http1(true)` + `GrpcWebLayer`).
- The image dominates RPC size. To avoid serializing/copying megabytes per frame on the same
  machine, the client can put pixels in **POSIX shared memory** (`/dev/shm`) and send only
  the name:
  - Client `shm_open`/creates a `SharedMemory` named `"/cedar_detect_image"`, sized
    `width·height`, copies the image in, sends `Image{width, height, shmem_name,
    reopen_shmem}` (no `image_data`).
  - Server `shm_open(name, O_RDONLY)` (caching the fd), `mmap`s it read-only, wraps it as a
    `GrayImage` **without copying**, runs detection, then `leak()`s the Vec (so Rust won't
    free the mapping) and `munmap`s. `reopen_shmem` forces the server to drop a cached fd
    when the client grew the buffer.
  - If shared memory fails server-side (`shm_open`/`mmap` error), the RPC returns gRPC
    **INTERNAL**; the production client catches this and **permanently falls back** to inline
    `image_data`. (Shared memory only works when client and server share a host.)

---

## 3. Server binary (`cedar-detect/src/bin/cedar_detect_server.rs`)

- **tonic** (async gRPC) on **tokio**; `clap` arg `--port/-p` (default 50051); `env_logger`
  (default `info`).
- On Linux sets parent-death signal (`prctl::set_death_signal(SIGTERM)`) so the server dies
  if the spawning client dies.
- Holds the shared-memory fd in `Arc<Mutex<Option<fd>>>`.
- `extract_centroids` handler:
  1. Require `input_image` (else `invalid_argument`).
  2. Acquire the `GrayImage`: from shared memory (`mmap`, with `reopen_shmem` handling) or
     from `image_data`.
  3. Choose binning: if `use_binned_for_star_candidates || return_binned`, `binning ∈ {2,4}`
     (default 2; other values → `invalid_argument`), else 1.
  4. `noise = estimate_noise_from_image(img)`; then `(stars, hot_pixel_count, binned_image,
     _hist) = get_stars_from_image(img, noise, sigma, normalize_rows, binning,
     detect_hot_pixels, return_binned)`.
  5. Optional `estimate_background_from_image_region` over a validated `Rectangle`.
  6. Tear down shared memory (`leak` + `munmap`).
  7. `peak_star_pixel` = average peak of up to the first `NUM_PEAKS=10` stars (else 255).
  8. Map each `StarDescription` → `StarCentroid{ centroid_position:{x,y}, brightness,
     num_saturated }`; fill `noise_estimate`, `hot_pixel_count`, `algorithm_time` (elapsed),
     and the optional `binned_image`.

A standalone tester `src/bin/test_cedar_detect.rs` runs the algorithm directly (no gRPC) on
a file/dir, draws detected centroids onto `.bmp` outputs (circle brightness encodes rank),
prints per-image WxH/noise/background/star-count/timing and **ms per megapixel**. Useful for
visual verification and benchmarking. `src/lib.rs` just exports `algorithm`,
`histogram_funcs`, `image_funcs`.

---

## 4. Python clients

### 4.1 Demo (`cedar-detect/python/cedar_detect_client.py`)

A script (not reusable class) showing the full loop: assumes the server is **already
running**; connects `grpc.insecure_channel('localhost:50051')`; for each test image
(`L`-converted to uint8) builds a `CentroidsRequest(sigma=8.0, use_binned_for_star_candidates=True)`,
optionally via shared memory (`create=True`, unlinked in `finally`); converts results to
`(y,x)` and calls `Tetra3.solve_from_centroids(..., fov_estimate=11)`; prints RPC overhead
= `rpc_time − algorithm_time`.

### 4.2 Production client (`cedar-solve/tetra3/cedar_detect_client.py`) — `CedarDetectClient`

The reusable, robust version actually used with the solver:

- `__init__(logger=None, binary_path=None, port=50051)` — **spawns and owns** the server
  subprocess (`bin/cedar-detect-server --port <port>`, `RUST_BACKTRACE=1`); validates the
  binary exists. Lazy gRPC stub; persistent shared memory; `_use_shmem=True`.
- `extract_centroids(image, sigma, use_binned, binning=None, detect_hot_pixels=True,
  normalize_rows=True) -> [(y, x), ...]`:
  - Shared-memory path: grow-only `/cedar_detect_image` buffer (`_alloc_shmem`, sets
    `reopen_shmem` when resized), copy image in, RPC with `wait_for_ready=True, timeout=2`.
    On gRPC `INTERNAL` → delete shmem, set `_use_shmem=False`, fall through to inline.
  - Inline path: `Image(image_data=np_image.tobytes())`.
  - On any RPC failure: if the subprocess died, **respawn once** and retry; else re-raise.
  - Returns the `(y,x)` list directly (ready for `solve_from_centroids`).
- `__del__` kills the subprocess and frees shared memory.

Differences vs the demo: it's a stateful class, manages the server lifecycle, reuses shared
memory across calls, and has fallback/retry. Both use TCP (not a Unix socket).

---

## 5. Wiring cedar-detect into a solver (the integration pattern)

```python
from tetra3 import Tetra3
from tetra3.cedar_detect_client import CedarDetectClient

t3 = Tetra3('my_database')
cd = CedarDetectClient()                      # starts the Rust server subprocess
img = np.asarray(pil_image.convert('L'), dtype=np.uint8)
centroids = cd.extract_centroids(img, sigma=8, use_binned=True,
                                 detect_hot_pixels=True, normalize_rows=True)
sol = t3.solve_from_centroids(centroids, img.shape, fov_estimate=11)
```

cedar-detect replaces `get_centroids_from_image`; the solver is unchanged — it just consumes
`(y,x)` brightest-first centroids either way.

---

## 6. Rebuild checklist (service)

1. proto with `ExtractCentroids(CentroidsRequest) -> CentroidsResult`, the `Image`
   inline-or-shmem duality, `(x,y)` `ImageCoord` with `(0.5,0.5)` pixel-center.
2. Server: load image (bytes or `mmap` shared memory), pick binning, call noise + star
   detection (doc 04), map to `StarCentroid`s brightest-first, report
   noise/hot-count/algorithm-time; INTERNAL on shmem failure.
3. Client: spawn/own the server, prefer shared memory with `reopen_shmem` on resize, fall
   back to inline bytes on INTERNAL, retry once on subprocess death, return `(y,x)`.
4. Remember the `(x,y)`→`(y,x)` swap at the boundary.
