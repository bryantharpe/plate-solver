## Context

See proposal.md for why, and ADR-0001 for the split. The console owns pointing. The scope node
only captures frames, finds stars and (from feat-14) drives two axes. This change builds on
feat-12:
- the observing session, with its injectable `Clock`, state file and same-origin rule;
- the latest pointing "at the aim point (the frame centre until an aim point is set)";
- push-to guidance.

feat-12 must be archived before this change, since its deltas only ADD to `observing-session`
and `finder-console` as feat-12 leaves them.

**Hardware facts and estimates** (`docs/research/2026-10-21-scope-node-runtime.md`):
- the Pi 3B: four Cortex-A53 cores, 1 GB RAM, 2.4 GHz-only Wi-Fi, no real-time clock;
- detection of roughly 40 ms a frame, estimated, not measured;
- an Arducam on the CSI ribbon, model unknown;
- WSL under NAT can connect out to the Pi but can't be found by broadcast.

**CI constraints** (`openspec/project.md` §7):
- clippy, coverage and docs run `--all-features` on x86_64 without libcamera;
- coverage counts Rust only, against an 80% floor;
- the `finder-page` job has no protoc today.

## Goals / Non-Goals

**Goals:**
- A live pointing on the console's clock, accurate to the solve, from a node that knows
  nothing of the sky.
- Every piece testable on CI with no camera: the protocol in-process, the Arducam adapter from
  saved fixtures and a fake `rpicam-vid`, and the whole loop on the simulator in virtual time.
- A simulator faithful enough that feat-14's GoTo and feat-15's tracking are proven against it.

**Non-Goals:**
- Hinted solving: each frame is solved lost-in-space, with only the field-of-view estimate
  (PRD: hinted solving is out of v1's scope).
- The mount: feat-14 adds the mount port, Sky-Watcher adapter and the simulator's mount half.
- TLS on the link, discovery by broadcast, and more than one node per console.

## Decisions

### Three new crates, layered so the Pi never links the solver

```
scope-link  (proto, generated code, auth, clock-mapping maths)
   ▲
scope-node  (camera port, adapters, detection loop, auto-exposure, config, link server)
   ▲            depends on star-detection and scope-link only
simulator   (sky rendering from the pattern database, optics, virtual clock,
   ▲         a scope-node service with simulated adapters)
finder      (link client, live loop, SSE, --simulate)
```

The simulator renders the sky with math-core's camera model and rotation (math-core is used,
not changed). It cannot use `finder`'s `Frame`, since `finder` depends on it.

*Alternatives:*
- The node as a module of `finder`. Rejected: the Pi binary would link the solver and the
  database loader.
- The simulator as an adapter only. Rejected: running the real node service over simulated
  adapters is what makes it "the same surface".

### The protocol: `scope_node.v1`, generated as `grpc-service` does

The proto is `crates/scope-link/proto/scope_node/v1/scope_node.proto`. It is built by
`build.rs` with `tonic-prost-build`, with a `[package.metadata.cargo-machete]` ignore for the
crates only generated code uses and a clippy allow on the generated module. There is no
`[...]` in proto comments, because rustdoc runs with `-D warnings`.

Calls:
- `Describe` → the camera's info;
- `Frames` → a server stream of `StarFrame`;
- `Heartbeat`, unary, every 250 ms, carrying t0 and answering t1 and t2;
- `Configure` → the settings in force;
- `RequestPreview`.

Pairing is an `authorization: Bearer` metadata entry, checked by a tonic interceptor with a
constant-time comparison.

The proto lives in the crate, not `proto/`: it is an internal link between two programs this
repository ships together, not the public solver API.

### The `finder-page` CI job installs protoc (named for approval)

`finder` now depends on `scope-link`, whose build runs protoc, and `finder-page` runs
`cargo run -p finder` for Playwright. The job gets the same "Install protoc" step as clippy,
test-coverage and docs.

*Alternatives:*
- Commit the generated code, with a freshness check. Rejected: two ways to generate gRPC code
  in one repository, and the check itself needs protoc.
- `protoc-bin-vendored`. Rejected: prebuilt binaries in the dependency graph, for one job's
  convenience.

### The node keeps one monotonic clock

Exposure times and heartbeat t1/t2 come from the same monotonic clock: the one the camera's
sensor timestamps use, CLOCK_BOOTTIME or CLOCK_MONOTONIC, which bring-up confirms. NTP steps
on the Pi never move them.

The console maps node time with the least-delay exchange of the last 8 s (32 heartbeats).
Crystal drift over 8 s is microseconds, against 15″ per second of error.

### The live loop: newest frame wins

The link client writes each frame into a `tokio::sync::watch` channel, which holds only the
newest. The solver task waits on it, takes the newest frame, and solves on a blocking thread,
so frames that arrive meanwhile overwrite each other. That is "dropping stale ones" with no
queue.

Each solve:
- uses `solve_from_centroids` with the last solved field as the estimate and a 5% error
  allowance;
- after three failures, uses no estimate;
- takes the pointing at the aim point by unprojecting the aim pixel through the solution's
  camera and distortion, timed at mid-exposure mapped to the console's clock;
- records the pointing in the session (feat-12's sequence number);
- broadcasts a compact live report to SSE subscribers.

### The page: SSE only while connected

`GET /api/session` (polled every 5 s) gains `node: {connected, last_frame_age_s,
offset_s, camera}`.

The page opens `EventSource('/api/live')` only while `connected` is true, and closes it when
the link drops. So a console with no node never holds a long-lived request, and the offline
end-to-end test still reaches `networkidle`.

Previews are fetched as `/api/live/preview.jpg?seq=N` and drawn under the chart in the existing
`.photo` red-only filter.

### Aim-point calibration snaps on the server

On Setup's live frame, the page sends the tapped pixel with the frame's id. The server snaps to the nearest centroid of
that frame within 10 px and stores the aim point in the session. So the oracle (the
simulator's (700, 420)) is checked in a Rust test, not only through the page.

The aim point is kept until reset; moving the camera is the observer's cue to reset it.

### The simulator

**Optics.**
- A 1456 × 1088 sensor (the IMX296's size, the sensor cedar-detect works around) with a 12°
  horizontal field, inside the bundled 10–30° database.
- The eyepiece at pixel (700, 420).
- The camera's roll fixed relative to the tube.

**Rendering.**
- Stars from the database's `star_table` (magnitude-sorted) brighter than a limit that follows
  the exposure.
- Gaussian PSFs (σ 1.2 px) with flux from magnitude, exposure and gain.
- Sky background, Poisson and read noise from an in-crate SplitMix64 seeded per frame, so runs
  repeat exactly with no new dependency.

**Sky to camera.** The telescope's true altitude and azimuth go to RA/Dec for the site and
virtual time: the inverse of the session's alt/az, so the same sky geometry. The camera's roll
comes from the parallactic angle.

**Time.** `VirtualClock` is the `--sim-start` instant plus `tokio::time::Instant` elapsed.
- Under `#[tokio::test(start_paused = true)]` it advances only when the test's tasks idle, so
  long runs take milliseconds and repeat exactly.
- Under `finder --simulate` it runs at the real rate.
- It is built in only those two places.

**Transport.**
- In tests, the console's real link client talks to the simulator's real node service over
  an in-memory `tokio::io::duplex` transport.
- Under `--simulate`, over loopback TCP on a random port.

Both exercise every call a real node gets.

**State.** `--simulate` defaults `--state` to a `simulate` subfolder, so a desk session never
overwrites the night's real site, target or node address.

**Pushed by hand.** `POST /api/sim/telescope` (altitude/azimuth, or a turn and tilt) is routed
only under `--simulate`, behind the same-origin check.

### The Arducam adapter: `rpicam-vid` as a subprocess

**Capture.** `rpicam-vid --codec yuv420 -o - --metadata <fifo> --metadata-format json
--nopreview -t 0`, with the mode, shutter, gain and frame rate set. Denoise and sharpening are
off, and contrast and saturation are neutral, to keep the ISP's tone curve from distorting
star brightness.

**Frames.**
- Read from stdout at the stride the stream was configured with. rpicam-vid prints it in
  verbose mode; bring-up confirms it and saves a fixture.
- `SensorTimestamp` gives the exposure start and `ExposureTime` its length.

**Settings changes** restart the subprocess, since rpicam-vid takes no runtime control. At 1–5
frames a second, auto-exposure's few steps cost a few seconds at most.

**Camera info** comes from parsing `rpicam-hello --list-cameras`.

**Tests.**
- Parsing and framing are tested on saved listings and saved streams with metadata.
- The subprocess handling is tested on CI by putting a fake `rpicam-vid` script first on
  `PATH`, which replays a fixture and then exits.
- What stays untested is only spawning the real binary.

*Alternative:* `libcamera` Rust bindings. Rejected: they need libcamera headers and libclang,
which the `--all-features` CI lacks.

### Auto-exposure

A proportional step in log exposure:

```
exposure ← exposure · (30 / max(N, 1))^0.7
```

- each step is clamped to ×4 either way;
- gain rises once exposure reaches its cap (or the frame-rate limit), and falls first when
  there are too many stars;
- there is no change while N is within 25–40.

On the simulator this settles in 3–5 frames from 5 stars.

### Configuration and the Pi build

**Config.** `/etc/scope-node/config.toml` (`--config` to move it) holds:
- the listen address;
- the token;
- the adapter (`arducam` or `simulated`);
- the starting exposure and gain.

It is parsed with serde's `deny_unknown_fields`. `--check-config` prints it with the token as
`****`.

**Build.** A static `aarch64-unknown-linux-musl` binary from WSL, because Pi OS's glibc is older
than Ubuntu 26.04's. `crates/scope-node/README.md` documents:
- the systemd unit (`Restart=always`, a dedicated user in the `video` group);
- that the config file is mode 0600.

## Risks / Trade-offs

- **The Arducam's model, and whether it works on a Pi 3B.** Some Arducam sensors need Arducam's
  own drivers, or don't support older Pis. → Bring-up (task 1) runs first; the camera port means
  another camera is an adapter, not a respec.
- **The finder camera's field is probably outside the bundled 10–30° database.** → Measure it at
  bring-up and build a matching database with `database-generation`. Setup warns when frames
  don't solve.
- **The camera ISP's tone curve compresses star brightness**, shifting the detection threshold.
  → Neutral ISP settings; compare against a raw capture at bring-up; adjust σ from Setup.
- **2.4 GHz-only Wi-Fi.** → Star lists are about 1 KB; previews are capped at one per 5 s and
  640 px; throughput is measured at bring-up.
- **NTP steps on the Pi.** → The node uses a monotonic clock only.
- **Detection time per frame on a Pi 3B is an estimate (about 40 ms).** → Measured at bring-up.
  Live push-to needs only 1–5 frames a second.
- **The token travels in clear on the house Wi-Fi.** It keeps other devices and mistakes out,
  not an attacker already on the network. feat-14 lets anyone Stop, so safety never depends on
  it.
- **The `ci.yml` change needs the owner's approval.** Until then `finder-page` can't build
  `finder`. The proposal names it.

## Migration Plan

Nothing to migrate. A console with no node configured behaves as after feat-12. Deploying the
node:
1. build the musl binary;
2. copy it and the config to the Pi;
3. enable the systemd unit;
4. type the node's address and token on Setup.

## Open Questions

- Which monotonic clock libcamera's `SensorTimestamp` uses on the Pi (BOOTTIME or MONOTONIC).
  Bring-up checks it; the design takes whichever it is.
- The Arducam's model, modes and field of view. Recorded at bring-up in
  `crates/scope-node/README.md`.
