## Why

feat-12 gives push-to directions from frames uploaded by hand. At the eyepiece the observer needs
them live: where the telescope points now, updated as it moves, from the finder camera on the
tube. Per ADR-0001, a scope node on the telescope captures frames and finds their stars, and the
console solves them. This change specifies that link, the node, a simulated node for the desk and
CI, the live solving loop, and the aim point that ties the finder camera to the eyepiece.

## What Changes

- A **scope link**: gRPC `scope_node.v1` between the console and the scope node, paired by a
  token. It carries star frames (ids, exposure times on the node's clock, settings and up to
  200 centroids, with an occasional preview), a heartbeat that maps the node's clock to the
  console's, and the console's camera and detection settings.
- A **scope node**: a service for the Raspberry Pi 3B on the telescope. It has a camera port with
  an Arducam adapter (an `rpicam-apps` subprocess), runs the same star detection as the console,
  holds 25–40 stars by auto-exposure, and is configured by a file.
- A **simulator**: a full simulated scope node that renders star fields from the pattern
  database for wherever its simulated telescope points, with pixel noise, a known aim-point
  offset and a virtual clock. It runs as `finder --simulate` and drives the end-to-end tests.
- **Live solving**: the console solves the newest frame and drops stale ones, timestamps each
  pointing at mid-exposure on its own clock, streams the live chart to the page while a node is
  connected, and calibrates the aim point on Setup by a tap on a star centred in the eyepiece.
  Guidance uses a live pointing only while it is at most 5 s old.
- The observing session gains the node's address and token (write-only), camera settings and the
  aim point. Setup gains a scope node and camera section.

## Capabilities

### New Capabilities

- `scope-link`: the console–scope node protocol: pairing, star frames, previews, the heartbeat
  and clock mapping, and the settings the console sends.
- `scope-node`: the service on the telescope: the camera port and the Arducam adapter, star
  detection on the node, auto-exposure, and its configuration.
- `simulator`: a stand-in scope node for a desk or CI, with simulated optics and a virtual
  clock, run as `finder --simulate`. Its camera half is here; feat-14 adds the mount.
- `live-finder`: the console's live loop: solving the newest frame, timing each pointing, the
  live chart, the aim point's calibration and the freshness rule for guidance.

### Modified Capabilities

- `observing-session` (introduced by feat-12; only ADDED): the scope node's address and token,
  the camera settings and the aim point.
- `finder-console` (only ADDED): Setup's scope node and camera section.

## Impact

- **New crates:**
  - `crates/scope-link`, the protocol, with its proto inside the crate since this internal link
    is not part of `proto/`'s public API;
  - `crates/scope-node`, a library and a binary depending on `star-detection` and `scope-link`
    only, so the Pi never needs the pattern database or the solver;
  - `crates/simulator`.
- **The `finder` crate** gains the link client, the live loop, `--simulate` and `--sim-start`,
  and a server-sent events route used only while a node is connected.
- **Dependencies:**
  - tonic and prost, as `grpc-service` uses them;
  - `toml`, for the node's config file;
  - `image`'s JPEG encoder on the node.
  All build on Rust 1.96 and pass cargo-deny.
- **The Pi build:** a static `aarch64-unknown-linux-musl` binary built in WSL, and a documented
  systemd unit.
- **CI (`ci.yml`), named for the owner's approval:** the `finder-page` job gains the same
  "Install protoc" step the Rust jobs already have, because `finder` now builds generated gRPC
  code before its end-to-end tests. No other job changes, and no test changes.
- **Playwright, named for the owner's approval:** a third server, `finder --simulate` with its
  own fresh `--state`, and a `simulate` project for the live tests. The existing projects and
  tests are unchanged.
- **Hardware unknowns**, settled at bring-up and recorded in the design:
  - the Arducam's exact model, and whether it works on a Pi 3B;
  - the finder camera's field of view, which probably needs its own pattern database (built
    with `database-generation`).
