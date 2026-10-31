---
state: draft
depends_on_landed: [012-targets-and-push-to]
# Converted 2026-10-28 from the reviewed OpenSpec change feat-13 (#102). This trio is
# now the feature's only specification. It stays draft until the owner has reviewed
# it, given the approvals listed under "Owner approvals", and the operator has staged
# the prerequisites in plan.md. Flipping it to `ready` is the act that dispatches it.
# Refined 2026-10-31 against main at 108a5a8 (012 landed in full) and the factory's
# gates as they run today (plan.md § Plan traps and § Refinement Record). It stays
# draft until the prerequisites in plan.md are met.
---

# Feature Specification: The Live Finder

**Feature Branch**: `013-live-finder`

**Created**: 2026-10-28

**Status**: Draft

**Input**: 012 gives push-to directions from frames uploaded by hand. At the eyepiece the
observer needs them live: where the telescope points now, updated as it moves, from the
finder camera on the tube. A scope node on the telescope captures frames and finds their
stars; the console solves them (ADR-0001). This feature specifies:

- the scope link between the console and the node, paired by a token;
- the scope node, a service for the Raspberry Pi 3B with a Raspberry Pi camera behind a camera
  port;
- a simulator, a full stand-in scope node for the desk and CI, run as `finder --simulate`;
- the live loop: solving the newest frame, timing each pointing at mid-exposure, the live
  chart, and the 5 s freshness rule for guidance;
- the aim point, calibrated on Setup by tapping a star centred in the eyepiece.

The observing session gains the node's address and token (write-only), the camera
settings and the aim point, and Setup gains a scope node and camera section.

## Clarifications

### Session 2026-10-28

- Q: What does a story read besides this trio? → A: Nothing about the feature. The trio is
  self-contained: `spec.md` holds every requirement and scenario, and `plan.md` holds
  every design decision, the interfaces, the shapes existing tests pin, and the
  alternatives rejected. A story that would change a decision escalates rather than
  re-deciding it.
- Q: How is the change sliced for the factory? → A: Into ten stories, each a vertical
  slice that lands its own tests, in one chain:
  - the scope link (`crates/scope-link`);
  - the node's camera port, rpicam adapter, detection and config (`crates/scope-node`);
  - the simulator's optics, rendered sky and virtual clock (`crates/simulator`);
  - the node's frame service, previews, settings and auto-exposure, proven on the
    simulator;
  - the console's link client and the session's node, camera settings and status;
  - the live loop, the aim point and the freshness rule;
  - `finder --simulate`, its controls and the live stream;
  - two page slices;
  - the end-to-end suite.

  Every one of the first seven adds or changes a crate's `Cargo.toml` or `Cargo.lock`, so
  the chain is also the only order that never races on the lockfile.
- Q: Where do the bring-up facts and fixtures come from? → A: The operator takes them at
  the telescope, before this spec is ready, and stages them on `main`:
  - a saved `rpicam-hello --list-cameras` listing;
  - a short saved `rpicam-vid` YUV420 stream with padded rows, and its JSON metadata;
  - a saved gray frame from the finder camera;
  - `fixtures.json`, which records what the files hold: the camera's model, size and
    modes, the stream's width, height, row stride and frame count, and each frame's
    timestamp and exposure;
  - the bring-up notes: the camera's model and modes, the row stride, which monotonic
    clock `SensorTimestamp` uses, the finder camera's measured field of view, detection
    time per frame on the Pi 3B, and Wi-Fi throughput.

  They live in `fixtures/scope-node/`, outside `crates/`, because a folder under
  `crates/` without a `Cargo.toml` breaks every cargo command in the workspace. A saved
  stream is several hundred kilobytes at least, past what a story should carry, and none
  of it can be produced without the hardware. US2 builds on the fixtures, and its tests
  take every camera value from `fixtures.json`, so new fixtures from another camera
  change no test.
- Q: Who changes `ci.yml`? → A: Nobody in a story. `finder` will build generated gRPC code
  before its end-to-end tests, so the `finder-page` job needs the same "Install protoc"
  step the Rust jobs have. The owner approves it (approval 1) and the operator lands it
  before this spec is ready.
- Q: How does Impeccable take part? → A: `DESIGN.md`, `.impeccable/design.json` and the
  surface briefs under `.impeccable/surfaces/` are the design system the page stories
  build to. The Impeccable review itself is outside the factory, because a "ship verdict"
  is not evidence the judge can see in a diff. US9 records the new parts in `DESIGN.md` as
  built, and the maintainer runs the review after US9 lands.
- Q: What is outside the factory altogether? → A: Everything that needs the telescope:
  bring-up, building and checking the static `aarch64-unknown-linux-musl` binary, a
  pattern database for the finder camera's field if it falls outside 10–30°, and the
  on-sky checklist. They are listed under Assumptions.

### Session 2026-10-31

- Q: What evidence does the judge credit? → A: Only the gate results and the story's
  diff, which share one 128 KiB budget. A Then clause that names a check proves it only
  if a declared gate runs that check, letter for letter, so the page stories'
  build-freshness scenarios name the `test` gate's own commands, as 012's do.
- Q: Do the new session fields change what 012's tests pin? → A: No. `node`,
  `camera_settings`, `aim` and `field` are left out of a session read until they have a
  value, and the state file holds the node, the camera settings and the aim point only
  once they are set. A fresh session reads exactly as 012's tests pin it, and a session
  that never pairs a node writes 012's three-key file.
- Q: Can the factory's `test` gate run protoc and a third Playwright server? → A: Yes,
  with no change to the gate. plan.md § Plan traps gives the evidence, and the one
  condition on the host: nothing else may listen on 8097–8099 while a story's gates run.
- Q: Can any of the ten stories be in flight at the same time? → A: Yes, the two page stories.
  This replaces "in one chain" in the slicing answer above. The first seven stories stay
  a chain, because each compiles against the crate or feature the one before it adds.
  US8 and US9 write only `crates/finder/web/**`, which no Rust story touches, and their
  tests fake the server from plan.md § Interfaces, so they build beside the first track.
  US10 joins the two tracks. The factory keeps up to two stories in flight, so the
  longest path drops from ten stories to eight. It has one coding agent, though, so two
  stories never build at once: one builds while the other is verified or in the merge
  queue, and the saving is that overlap, not a second builder. Each story has its own
  worktree, but every builder and gate shares the host's ports, so two end-to-end runs
  must not meet (plan.md, Prerequisite 6).

## Owner approvals

Flipping this spec to `ready` approves the following. Each is named in the FR that needs
it. No existing test changes.

1. **`ci.yml`'s `finder-page` job gains an "Install protoc" step**, the same step the
   clippy, test-coverage and docs jobs already run, because `finder` now depends on
   `scope-link`, whose build runs protoc (FR-017). The operator lands it as a prerequisite;
   no other job changes.
2. **`crates/finder/web/playwright.config.ts` gains a third server.** It is
   `finder --simulate` with its own fresh `--state`, a fixed `--sim-start` and
   `reuseExistingServer: false`, serving a `simulate` project for the live tests. The
   existing projects and tests are unchanged (FR-041).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The scope link: a paired protocol with mapped clocks (Priority: P1)

As the console and the scope node, we speak `scope_node.v1` over gRPC. Every call carries
the session's token, star frames carry their exposure times on the node's clock, and
heartbeats let the console map the node's clock onto its own to well under a second.

**Why this priority**: Every later story speaks this protocol. A clock mapped wrongly by
one second misplaces every live pointing by 15″, and an unpaired link lets any device on
the house Wi-Fi feed the console frames.

**Independent Test**: `cargo test -p scope-link --locked` runs the unit tests of the
clock maths in `crates/scope-link/src/clock.rs` and `crates/scope-link/tests/link.rs`,
which serves a test implementation of the service behind the token interceptor over an
in-memory transport.

**Acceptance Scenarios**:

1. **Given** a test service behind the token interceptor, configured with the token
   `test-token`, **When** a test calls its frame stream with no token and then with
   `wrong-token`, **Then** it asserts that both calls fail with UNAUTHENTICATED and that
   no frame is sent.
2. **Given** the same service, **When** a test calls with `test-token`, **Then** it
   asserts that the stream opens and frames arrive.
3. **Given** exchange A with t0 100.000, t1 100.070, t2 100.071 and t3 100.041 s, and
   exchange B with t0 101.000, t1 101.150, t2 101.151 and t3 101.121 s, **When** a test
   maps the clock, **Then** it asserts that A's delay is 0.040 s and B's 0.120 s, and that
   the offset used is A's, +0.050 s, within 1 µs.
4. **Given** a least-delay exchange (delay 0.010 s, offset +0.020 s) that is 9 s old, and
   every exchange of the last 8 s with delay 0.040 s and offset +0.050 s, **When** a test
   maps the clock, **Then** it asserts that the offset used is +0.050 s within 1 µs, and
   that node time 200.100 s maps to 200.050 s.
5. **Given** `crates/scope-link/proto/scope_node/v1/scope_node.proto`, **When** it is
   read, **Then** it declares package `scope_node.v1`; the calls `Describe`, `Frames` (a
   server stream of `StarFrame`), `Heartbeat`, `Configure` and `RequestPreview`; and a
   `StarFrame` with an id, exposure start and end on the node's clock, frame size,
   exposure, gain, the detection settings used, up to 200 centroids and an optional
   preview. No comment in it contains `[`.
6. **Given** the story's diff, **When** it is read, **Then** three things hold:
   - `crates/scope-link/build.rs` builds the proto with `tonic-prost-build`, as
     `crates/grpc-service/build.rs` does;
   - `crates/scope-link/Cargo.toml` has a `[package.metadata.cargo-machete]` ignore for
     the crates only generated code uses, and the generated module carries a clippy allow;
   - `proto/` at the repository root is unchanged.
7. **Given** the interceptor's token comparison, **When** a test compares an equal token,
   one differing only in its last byte, and one of a different length, **Then** it
   asserts that only the equal token is accepted. The comparison reads every byte of both
   tokens before deciding, with no early return on the first difference.

---

### User Story 2 - The scope node's camera port, the rpicam adapter, the console's stars, and its config (Priority: P1)

As the scope node, I take frames only through a camera port, so another camera is an
adapter rather than a respec. The rpicam adapter runs `rpicam-vid` and reads its YUV420
stream and metadata. I find stars exactly as the console does, and I am configured by a
file I refuse to start on if it holds a typo.

**Why this priority**: The camera port is what makes the node testable without a camera,
and identical detection is what lets the console solve the node's stars as if it had
found them itself.

**Independent Test**: `cargo test -p scope-node --locked` runs:
- the unit tests in `crates/scope-node/src/camera.rs` and `src/config.rs`;
- `crates/scope-node/tests/rpicam.rs`, on the saved listing and stream in
  `fixtures/scope-node/`, checked against that folder's `fixtures.json`, with a fake
  `rpicam-vid` script put first on `PATH`;
- `crates/scope-node/tests/detect.rs`, on the saved finder frame;
- `crates/scope-node/tests/cli.rs`, which runs the built `scope-node` binary.

**Acceptance Scenarios**:

1. **Given** a camera behind the port that reports an exposure range, **When** a test asks
   for an exposure longer than that range, **Then** it asserts that the port refuses with
   an error naming the range, and that no frame is taken.
2. **Given** the saved `rpicam-hello --list-cameras` output, **When** a test parses it,
   **Then** it asserts that the adapter reports the sensor model, its full size and every
   mode's size exactly as `fixtures.json` records them.
3. **Given** the saved YUV420 stream, whose row stride is wider than its width, and its
   saved metadata, at the width, height and stride `fixtures.json` records, **When** a
   test reads it, **Then** it asserts that each frame's gray
   image equals the saved Y plane byte for byte without the padding, and that its exposure
   start and end equal the saved timestamp and the timestamp plus the exposure time.
4. **Given** a fake `rpicam-vid` first on `PATH` that replays the saved stream and then
   exits, **When** the node captures through the adapter, **Then** a test asserts that the
   adapter reports the error and the script's stderr, and that the node's capture loop
   restarts the camera without restarting the service.
5. **Given** the saved finder frame, **When** a test detects it through the node and
   through the console's own call (`star_detection::detect_stars` with
   `plate_solver::DetectParams::cedar()`), **Then** it asserts that the two centroid lists
   are identical, value for value and in the same order.
6. **Given** the node's default detection settings, **When** a test compares them with
   `plate_solver::DetectParams::cedar()`, **Then** it asserts that they are equal field for
   field. `plate-solver` is only a dev-dependency of `scope-node`.
7. **Given** a config holding the token `test-token`, **When** `scope-node --check-config`
   runs on it, **Then** a CLI test asserts that it prints every setting with the token as
   `****`, never the token, and exits 0.
8. **Given** a config holding the key `exposre_ms`, or a gain of `loud`, **When** the node
   starts on it, **Then** a CLI test asserts a nonzero exit and a message naming that key.
9. **Given** the story's diff, **When** it is read, **Then** three things hold:
   - `crates/scope-node/Cargo.toml`'s `[dependencies]` name `star-detection` and
     `scope-link` and none of `plate-solver`, `pattern-database` or `finder`;
   - `crates/scope-node/README.md` documents the static `aarch64-unknown-linux-musl`
     build with its target and flags on the command line, the systemd unit
     (`Restart=always`, a dedicated user in the `video` group) and the config file's mode
     0600, and links the operator's bring-up notes in `fixtures/scope-node/README.md`;
   - the repository gains no `.cargo/config.toml`.

---

### User Story 3 - The simulator: known optics, a rendered sky, and a virtual clock (Priority: P1)

As a desk or a CI run with no telescope, I use a simulated finder camera whose optics and
eyepiece offset are known. It renders the pattern database's stars for wherever its
simulated telescope points, at its site and on its virtual clock, with pixel noise.

**Why this priority**: The simulator is the oracle for everything live: the solve, the
aim point and, in 014 and 015, GoTo and tracking. Its frames have to solve to where it
truly points.

**Independent Test**: `cargo test -p simulator --locked` runs the unit tests in
`crates/simulator/src/` and `crates/simulator/tests/sky.rs`, which renders frames, detects
them with the node's detection and solves them with `plate_solver::solve_from_centroids`
and the bundled 10–30° database, as the console solves.

**Acceptance Scenarios**:

1. **Given** the simulator's camera adapter behind the camera port, **When** a test asks
   what it is and for one frame, **Then** it asserts that it reports its model, size and
   ranges, and yields an 8-bit gray frame of that size whose exposure end minus start
   equals the exposure asked for, within 1 µs.
2. **Given** the simulated finder camera at altitude 50°, azimuth 120° from 40° N, 75° W at
   2026-01-16 02:00 UT, **When** a test solves its frame, **Then** it asserts that the
   solved centre is within 30″ of the camera's true direction.
3. **Given** the simulated telescope placed so that a star lies in its eyepiece's centre,
   **When** a test detects the next frame, **Then** it asserts that the star's centroid is
   at (700, 420) within 0.3 px.
4. **Given** the same direction, virtual time, exposure and frame number, **When** a test
   renders twice, **Then** it asserts that the frames are identical byte for byte, and
   that the next frame number gives different noise.
5. **Given** a simulator built with its defaults, **When** a test reads its optics,
   **Then** it asserts a 1456 × 1088 sensor, a 12° horizontal field, the eyepiece at pixel
   (700, 420), and that the field and the eyepiece pixel can each be set. The true camera
   and eyepiece directions are readable by the test.
6. **Given** a `VirtualClock` started at 2026-01-16T02:00:00Z under
   `#[tokio::test(start_paused = true)]`, **When** the test advances tokio's time by 10 s,
   **Then** it asserts that the clock reads 1768528810 exactly.

---

### User Story 4 - The node serves star frames, previews and settings, and keeps 25–40 stars (Priority: P1)

As the scope node, I stream one star frame per exposure with its brightest 200 stars, send
a small preview now and then or when asked, apply the console's camera and detection
settings from the next exposure on, and hold 25 to 40 stars by auto-exposure. The
simulator runs this same service over its simulated camera.

**Why this priority**: These are the node's whole job on the link. Running the real
service over the simulated camera is what makes the simulator "the same surface".

**Independent Test**: `cargo test -p simulator --locked` runs
`crates/simulator/tests/node.rs`, where the real node service from `scope-node` runs over
the simulated camera under `#[tokio::test(start_paused = true)]`, and a test client speaks
the link over an in-memory transport.

**Acceptance Scenarios**:

1. **Given** the simulator's node pointed at a field holding more than 200 detectable
   stars, **When** a test takes ten frames, **Then** it asserts that each frame has every
   field of a star frame, that ids rise by one, that each exposure's end minus start equals
   its exposure time within 1 µs, and that each holds exactly 200 centroids in descending
   brightness.
2. **Given** a client that misses one frame of the stream (the test drops one received
   message), **When** it reads the next, **Then** the test asserts that the next frame's
   id is two more than the last one the client kept. Ids count exposures, not messages.
3. **Given** the simulator's node streaming 2 frames a second for 20 s of virtual time,
   **When** a test counts previews, **Then** it asserts that at most 5 frames carry one,
   that every preview is a JPEG at most 640 px on its longer side, and that any two are at
   least 5 s apart.
4. **Given** a preview sent, **When** the client calls `RequestPreview` 1 s later, **Then**
   a test asserts that the next frame carries a preview.
5. **Given** a node that has received no settings, **When** a test reads its frames,
   **Then** it asserts that every frame reports cedar's detection settings.
6. **Given** a node, **When** the client sends a detection threshold of 5σ, **Then** a test
   asserts that every frame from the second one after it reports 5σ.
7. **Given** the simulator started at an exposure that gives 5 stars, **When** a test takes
   frames with the field unchanged, **Then** it asserts that within 6 frames the star count
   is between 25 and 40, and that it stays there.
8. **Given** the client has set 200 ms and gain 4, **When** the field changes to one that
   gives 5 stars, **Then** a test asserts that every later frame is still at 200 ms and
   gain 4.
9. **Given** the client has set 200 ms with gain automatic, **When** the field changes to
   one that gives 5 stars, **Then** a test asserts that every later frame is at 200 ms, and
   that within 6 frames the gain has moved the star count to between 25 and 40.
10. **Given** the simulator's node service, **When** a test reaches it once over an
    in-memory transport and once over loopback TCP on a random port, **Then** it asserts
    that `Describe` answers the same camera over both.

---

### User Story 5 - The console pairs with the node, and the session holds the node and its camera settings (Priority: P1)

As the console, I connect to the node at the address the session holds, with its token,
keep a heartbeat going, count the node connected or not, and reconnect when it drops. The
session holds the node's address and a write-only token, the camera's exposure, gain and
detection threshold, which I send when they change and whenever the node connects, and a
read of the session reports the link's state.

**Why this priority**: The link client is the console's end of everything live, and the
session is where the observer configures the node from any device.

**Independent Test**: `cargo test -p finder --locked` runs:
- `crates/finder/tests/link.rs`, where the console's link client talks to the simulator's
  node service over an in-memory transport in paused time;
- `crates/finder/tests/session_node.rs`, through the router with a `FixedClock`;
- `crates/finder/tests/session_node_cli.rs`, which restarts the built `finder` binary on
  one `--state` folder.

**Acceptance Scenarios**:

1. **Given** a link to the simulator, **When** it runs for 10 s of virtual time, **Then** a
   test asserts that the console has recorded at least 40 heartbeats.
2. **Given** the simulator's node stops answering for 3 s of virtual time and then answers
   again, **When** a test watches the link, **Then** it asserts that the console reports
   it disconnected 1 s (±0.1 s) after the last heartbeat, and connected again within 5.5 s
   of its return.
3. **Given** the console's link client and the simulator's node, behind a recording
   interceptor, **When** they pair, stream frames, map the clock and send settings,
   **Then** a test asserts that the client used exactly the calls `Describe`, `Frames`,
   `Heartbeat` and `Configure` with the token as `authorization: Bearer`, through the same
   client code it uses for a real node's address.
4. **Given** a client puts the node `192.168.1.40:50051` with the token `test-token`,
   **When** it reads the session, **Then** a test asserts that the read gives the address
   and `token_set: true`, and that the text `test-token` appears nowhere in it.
5. **Given** a node address and token set through the binary, **When** it restarts on the
   same `--state` folder, **Then** a CLI test asserts that both are as set, and that the
   state file's mode is 0600.
6. **Given** a client puts the node `not an address`, **When** it is sent, **Then** a test
   asserts a 400 answer with a JSON `error`, and that nothing changes.
7. **Given** an exposure of 200 ms put while no node is connected, **When** the simulator
   then connects, **Then** a test asserts that the simulator's frames report 200 ms from
   the second frame after connecting.
8. **Given** a connected camera, **When** a client puts an exposure longer than its range,
   **Then** a test asserts a 400 answer with a JSON `error` naming the range, and that
   nothing changes.
9. **Given** an exposure of 200 ms, gain 4 and a threshold of 5σ set through the binary,
   **When** it restarts on the same `--state` folder, **Then** a CLI test asserts that all
   three are exactly as set.
10. **Given** an exposure of 2000 ms stored, **When** the simulator connects with its
    camera's range set to at most 1000 ms, **Then** a test asserts that its frames report
    automatic exposure from the second frame after connecting, and that the session
    reports that the stored 2000 ms is outside the camera's range, keeping the stored
    value.
11. **Given** the simulator connected, **When** a test reads `GET /api/session`, **Then** it
    asserts a `node` holding `connected: true`, the latest frame's age, the clock offset,
    the camera's model and ranges, and the latest frame's star count.
12. **Given** the story's diff, **When** it is read, **Then** the test files 012 added are
    unchanged, and edits to the node and camera settings go through the same-origin check
    012 put on `PUT /api/session`.

---

### User Story 6 - Live solving: the newest frame wins, timed at mid-exposure, with a calibrated aim point and fresh pointings only (Priority: P1)

As the console with a node connected, I solve the newest star frame each time I am free,
dropping frames that arrived meanwhile, and time each pointing at the frame's
mid-exposure on my own clock. I take the pointing at the aim point, which the observer
calibrates by tapping a star centred in the eyepiece. A live pointing more than 5 s old
gives no turn or tilt, connected or not.

**Why this priority**: This is the feature's purpose: a pointing, live and accurate to the
solve, at the spot the eyepiece sees, and guidance that never trusts a pointing too old to
be true.

**Independent Test**: `cargo test -p finder --locked` runs:
- `crates/finder/tests/live.rs`, where the simulator and the console share one runtime
  over an in-memory link under `#[tokio::test(start_paused = true)]`, with a solver seam
  the tests can slow down or make fail;
- `crates/finder/tests/aim_api.rs` and `crates/finder/tests/freshness_api.rs`, through the
  router;
- `crates/finder/tests/aim_cli.rs`, which restarts the built binary.

**Acceptance Scenarios**:

1. **Given** the aim point set to the simulator's eyepiece pixel and the simulator
   streaming 2 frames a second for 10 s of virtual time with the telescope still, **When**
   a test watches the live loop, **Then** it asserts that every frame is solved and every
   solved pointing is within 30″ of the simulator's true eyepiece direction at that
   frame's mid-exposure.
2. **Given** each solve made to take 1.5 s while frames arrive at 2 a second, **When** a
   test watches the solver, **Then** it asserts that each solve starts on the newest frame
   received at that moment, and that no older frame is solved after a newer one.
3. **Given** a frame that solves with a 12.04° field, **When** the next frames arrive,
   **Then** a test asserts that the next is solved with 12.04° as its estimate, and that
   after three failures in a row the following frame is solved with no estimate.
4. **Given** a link offset of +0.050 s and a frame exposed from 200.000 to 200.200 s on the
   node's clock, **When** its pointing is recorded, **Then** a test asserts that its time
   is 200.050 s on the console's clock, within 1 µs.
5. **Given** the simulated eyepiece centred on a star, **When** the observer's tap 6 px
   from that star on the live frame is posted, **Then** a test asserts that the aim point
   becomes (700, 420) within 0.5 px, and that later pointings are the simulator's true
   eyepiece direction within 30″.
6. **Given** a tap more than 10 px from every centroid of its frame, **When** it is
   posted, **Then** a test asserts that the aim point is unchanged and that the answer
   says no star is close enough to the tap.
7. **Given** an aim point, **When** it is reset, **Then** a test asserts that pointings are
   at the frame centre again.
8. **Given** the aim point set to (700, 420) through the binary, **When** the binary
   restarts and the aim point is then reset, **Then** a CLI test asserts that after the
   restart it is (700, 420) exactly, and that after the reset the session has none.
9. **Given** the node still connected and the last live pointing 5.1 s old by the
   console's clock, **When** the guidance is read, **Then** a test asserts that it gives no
   turn or tilt and says the pointing is stale, and that it gives them again with the next
   solve.
10. **Given** the link dropped, **When** the last live pointing then passes 5 s old,
    **Then** a test asserts that the guidance still gives no turn or tilt, and says the
    pointing is stale.
11. **Given** the simulator streaming frames of 25 or more stars for 30 s of virtual time
    and every solve failing (the solver seam), **When** the session is read, **Then** a
    test asserts that it reports frames of at least 10 stars unsolved for at least 30 s.
12. **Given** the simulator's 12° field solved against the bundled 10–30° database,
    **When** the session is read, **Then** a test asserts that it reports the solved field
    as 12° (within 0.1°) and the database's range as 10–30°.

---

### User Story 7 - `finder --simulate`: the desk console, its own state, pushed by hand, and the live stream (Priority: P1)

As the maintainer at a desk, I run `finder --simulate --sim-start 2026-01-16T02:00:00Z`
and get the whole console on a simulated night: an in-process simulator as its node on a
virtual clock, its own state folder, a control that pushes the simulated telescope by
hand, and the live stream the page will draw. It never reaches a real node.

**Why this priority**: `--simulate` is how the page, the end-to-end tests, and 014–015 run
without a telescope. A virtual clock that could reach real hardware would break the Sun
guard, so it must be impossible.

**Independent Test**: `cargo test -p finder --locked` runs:
- `crates/finder/tests/simulate_cli.rs`, which runs the built binary with `--simulate`;
- `crates/finder/tests/sim_api.rs` and `crates/finder/tests/live_stream.rs`, through the
  router built in simulate mode;
- `crates/finder/tests/e2e_live.rs`, the whole loop in one runtime in paused time.

**Acceptance Scenarios**:

1. **Given** `finder --simulate --sim-start 2026-01-16T02:00:00Z`, **When** a CLI test
   reads its session, **Then** it asserts that the session's time is 2026-01-16 02:00 UT
   within 2 s and that frames arrive from the simulator.
2. **Given** a session holding the address of a listening test server, **When**
   `finder --simulate` runs for 5 s, **Then** a CLI test asserts that the test server
   received no connection.
3. **Given** `finder --sim-start 2026-01-16T02:00:00Z` without `--simulate`, **When** it
   runs, **Then** a CLI test asserts a nonzero exit with a message naming `--sim-start`,
   and that it serves nothing.
4. **Given** `finder --simulate` without `--state`, with `XDG_STATE_HOME` pointing at a
   temporary folder holding a default state file, **When** the aim point and node address
   are set, **Then** a CLI test asserts that the default state file is unchanged byte for
   byte, and that the session was written under the `simulate` folder.
5. **Given** simulate mode, **When** the simulated telescope is pushed to altitude 40°,
   azimuth 200°, **Then** a test asserts that the next frame solves within 30″ of that
   direction.
6. **Given** the console without `--simulate`, **When** a client asks for any simulator
   control, **Then** a test asserts that the route answers 404 and nothing moves.
7. **Given** simulate mode, **When** a client stops the simulator's frames and then starts
   them, **Then** a test asserts that no frame arrives while stopped and that frames
   resume after the start.
8. **Given** simulate mode, **When** a client reads the simulator's telescope, **Then** a
   test asserts that the answer gives the true camera and eyepiece directions the
   simulator holds.
9. **Given** a request to a simulator control carrying `Origin: http://elsewhere.example`,
   **When** it is sent, **Then** a test asserts a 403 answer and that nothing moves.
10. **Given** a node connected, **When** a client subscribes to `GET /api/live` and two
    frames solve, **Then** a test asserts two server-sent events, each carrying that
    solve's live report, and that `GET /api/live/preview.jpg?seq=N` answers the JPEG
    preview of a frame that carried one.
11. **Given** the simulator and the console in one runtime over an in-memory link, in
    paused time, at 40° N, 75° W from 2026-01-16 02:00 UT with M78 as the target, **When**
    a test runs from the first frame through a calibrating tap to the guidance, **Then** it
    asserts that the aim point is (700, 420) within 0.5 px and that the guidance's turn and
    tilt lead from the simulator's true eyepiece direction to M78 within 30″ on the sky.

---

### User Story 8 - The page's live chart and a stale pointing (Priority: P2)

As the observer at the eyepiece, my page follows the telescope: while a node is connected
the chart updates with each live solve over a server-sent event stream, and the node's
previews appear under the frame through night mode's red-only filter. When the newest
live pointing passes 5 s old, the page says the pointing is stale within a second. With
no node, the page opens no stream and behaves as before.

**Why this priority**: It is the visible half of live push-to. It is P2 because it
consumes US7's stream.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`:
- the new tests are `src/useLive.test.ts`, `src/components/LivePreview.test.tsx` and
  `src/App.live.test.tsx`;
- every existing test is unchanged.

Then the `test` gate's build-freshness step passes on the rebuilt `web/dist` (the same
check CI's `finder-page` job runs).

**Acceptance Scenarios**:

1. **Given** a session with no node, or a node with `connected: false`, **When** the app
   renders, **Then** a Vitest test asserts that no `EventSource` is created.
2. **Given** a session whose node turns connected and then disconnected, **When** the
   polls arrive, **Then** a Vitest test asserts that one `EventSource('/api/live')` is
   created when it connects and closed when it disconnects.
3. **Given** a recorded live report delivered as a stream event, **When** the app renders,
   **Then** a Vitest test asserts that the chart's title block and marks are the report's
   pointing and stars.
4. **Given** a live report that names a preview, **When** it renders, **Then** a Vitest
   test asserts an image from `/api/live/preview.jpg?seq=N` under the frame, inside the
   element that carries the `.photo` red-only filter.
5. **Given** a node connected and M78 as the target, **When** the newest live pointing
   passes 5 s old by the console's clock, **Then** a Vitest test under fake timers asserts
   that within 1 s the guidance block and the chart say the pointing is stale and show no
   turn or tilt.
6. **Given** the rebuilt `web/dist`, **When** the `test` gate runs `npm run build` and then
   `git status --porcelain -- dist`, **Then** it lists nothing: the committed build is the
   build of this source, and `package.json` dependencies are unchanged.

---

### User Story 9 - Setup's scope node and camera section, the field warning, and the aim-point tap (Priority: P2)

As the observer, Setup is where I pair the node, set the camera's exposure, gain and
detection threshold, see the link's state, the camera's model and the star count, tap the
star centred in my eyepiece to set the aim point, and learn when the finder camera's field
needs a database built for it.

**Why this priority**: Pairing and calibration are done once a night, on Setup. It is P2
because it consumes US5's and US6's session fields.

**Independent Test**: In `crates/finder/web`, run `npm run typecheck`,
`npm run format:check` and `npm test`:
- the new tests are `src/components/ScopeNodeSection.test.tsx`,
  `src/components/FieldWarning.test.tsx` and `src/components/AimTap.test.tsx`;
- every existing test, including the Setup page's from 012, is unchanged.

Then the `test` gate's build-freshness step passes on the rebuilt `web/dist`.

**Acceptance Scenarios**:

1. **Given** a session read with `token_set: true`, **When** Setup renders on another
   device, **Then** a Vitest test asserts that the token field is empty, says a token is
   set, and that no request the page made or answer it received carries a token value.
2. **Given** a recorded session with the simulator connected, **When** Setup renders,
   **Then** a Vitest test asserts that it shows the link as connected, the simulated
   camera's model and the latest frame's star count, and updates them with each live
   event.
3. **Given** a solved field of 12° against a database range of 10–30°, **When** Setup
   renders, **Then** a Vitest test asserts that it shows 12° against 10–30° with no
   warning.
4. **Given** a solved field of 10.5° against 10–30°, **When** Setup renders, **Then** a
   Vitest test asserts that it shows the field with the warning that a database built for
   the camera's field may be needed.
5. **Given** a session reporting frames of at least 10 stars unsolved for 30 s, **When**
   Setup renders, **Then** a Vitest test asserts that it warns that the camera's field may
   be outside the database's 10–30° range and that a matching database may be needed.
6. **Given** a session reporting that a stored 2000 ms exposure is outside the camera's
   range, **When** Setup renders, **Then** a Vitest test asserts that it says so.
7. **Given** Setup's live frame, **When** the user taps it, **Then** a Vitest test asserts
   that the page posts the tapped pixel with the frame's id; that an answer saying no star
   is close enough is shown; and that the reset control clears the aim point.
8. **Given** Setup's section, **When** the user saves an address, a token, an exposure,
   gain and threshold (each either automatic or a value), **Then** a Vitest test asserts
   one `PUT /api/session` with a JSON body holding them.
9. **Given** `DESIGN.md` and `.impeccable/design.json`, **When** they are read, **Then**
   they describe the live chart, the preview under the frame, the stale state, Setup's
   scope node and camera section and the aim-point tap, as built.
10. **Given** the rebuilt `web/dist`, **When** the `test` gate runs `npm run build` and then
    `git status --porcelain -- dist`, **Then** it lists nothing: the committed build is the
    build of this source, and no npm package was added.

---

### User Story 10 - End to end, on the simulator (Priority: P3)

As the maintainer, I run `npm run e2e` and a third server, `finder --simulate` with its
own throwaway state and a fixed start time, proves the live finder in Chromium: the chart
following a push, the stale state when frames stop, Setup connected to the simulator, the
aim-point tap, and the red-only night rule on the live chart.

**Why this priority**: These tests confirm in a real browser what the unit tests prove
piece by piece. They come last because they exercise every story before them.

**Independent Test**: In `crates/finder/web`, the `test` gate's `CI=1 npm run e2e` runs
every earlier test unchanged and green, plus the new `e2e/simulate.spec.ts` in the
`simulate` project and `e2e/live-off.spec.ts` in the default project.

**Acceptance Scenarios**:

1. **Given** `playwright.config.ts`, **When** it is read, **Then** four things hold:
   - a third server runs `finder --simulate` with its own fresh `--state`, a fixed
     `--sim-start` and `reuseExistingServer: false`, serving a `simulate` project;
   - the existing projects ignore `e2e/simulate.spec.ts`;
   - the earlier projects' servers and settings are unchanged;
   - every earlier e2e file is unchanged.
2. **Given** the page open on the simulate server, **When** a committed Playwright test
   pushes the simulated telescope, **Then** it asserts that within 2 s the chart's pointing
   is within 30″ of the simulator's new true direction, read from the simulator's control,
   and that within 5.5 s a preview of a frame exposed after the push is under the frame.
3. **Given** the site 40° N, 75° W and M78 as the target on the simulate server started at
   2026-01-16T02:00:00Z, **When** a committed Playwright test stops the simulator's
   frames, **Then** it asserts that within 6 s of the last frame's mid-exposure, by the
   virtual clock, the guidance block and the chart say the pointing is stale and show no
   turn or tilt.
4. **Given** Setup open on the simulate server, **When** a committed Playwright test reads
   it, **Then** it asserts the link as connected, the simulated camera's model, and the
   latest frame's star count between 25 and 40, updated within 2 s of each frame.
5. **Given** the simulated eyepiece centred on a star, **When** a committed Playwright test
   taps that star on Setup's live frame, **Then** it asserts that the session's aim point
   becomes (700, 420) within 0.5 px.
6. **Given** the live chart with a preview in night mode, **When** a committed Playwright
   test runs the night pixel check on its screenshot, **Then** it asserts no pixel with any
   green or blue, and none redder than the ink beyond two levels of compositing rounding.
7. **Given** the default server, which has no scope node, **When** a committed Playwright
   test opens the page, **Then** it asserts that no request to `/api/live` is made and that
   the page reaches network idle.

---

### Edge Cases

- **The link:**
  - A heartbeat answered after more than 1 s does not revive a link already counted
    disconnected until the next heartbeat answers in time.
  - A token is never logged, never sent back, and never written anywhere but the 0600
    state file.
  - A frame stream that ends is a disconnection; the client reconnects with no more than
    5 s between tries.
- **The node:**
  - More than 200 detected stars are cut to the brightest 200; fewer are all sent.
  - A settings change restarts `rpicam-vid`, so frames pause for at most a few seconds;
    ids keep counting exposures across the restart.
  - An exposure or gain outside the camera's reported range is refused by the port, never
    clamped silently.
- **The live loop:**
  - A frame with fewer than 4 stars is not solved and counts as a failure for the
    field-of-view estimate.
  - An uploaded frame's pointing still never goes stale (012); only live pointings do.
  - A live pointing replaces an uploaded one as the latest pointing, and the reverse.
- **The simulator:**
  - A telescope pointed below the horizon renders a frame with no stars; it is still a
    frame, with an id and times.
  - Simulator controls never exist outside `--simulate`, whatever the request.
- **The session's new fields:**
  - A read leaves out `node`, `camera_settings`, `aim` and `field` until each has a value,
    and the page reads a missing field as unset.
  - The state file holds the node, the camera settings and the aim point only once they
    are set. A state file that 012 wrote loads unchanged.

## Requirements *(mandatory)*

### Functional Requirements

**The scope link** (US1):

- **FR-001**: The scope node SHALL serve the gRPC service `scope_node.v1`, and the console
  SHALL connect to it at the address the session holds. Every call except one that can
  only stop the telescope SHALL carry the session's token as `authorization: Bearer`, and
  the node SHALL answer a call with no token or another token with UNAUTHENTICATED,
  serving nothing. Tokens SHALL be compared in constant time. **Refs**: ADR-0001;
  `docs/research/2026-10-21-scope-node-runtime.md` §5 (WSL connects out).
- **FR-002**: The protocol SHALL be defined in
  `crates/scope-link/proto/scope_node/v1/scope_node.proto` and generated with
  `tonic-prost-build` as `grpc-service` does. It SHALL offer `Describe` (the camera's
  information), `Frames` (a server stream of star frames), `Heartbeat`, `Configure` (the
  settings in force) and `RequestPreview`. It is an internal link and SHALL NOT be added
  to `proto/`. **Refs**: ADR-0001; plan.md § Interfaces.
- **FR-003**: The console and node SHALL exchange heartbeats at least every 250 ms, each
  recording t0 (console sends), t1 (node receives), t2 (node replies) and t3 (console
  receives). The console SHALL map node time to its own with the offset
  ((t1 − t0) + (t2 − t3)) / 2 of the exchange with the least delay
  (t3 − t0) − (t2 − t1) among those of the last 8 s, so that node time T is T − offset on
  the console's clock. **Refs**: RFC 5905 §8;
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §4 (1 s is 15″).

**The scope node** (US2):

- **FR-004**: The scope node SHALL take frames only through a camera port that reports
  the sensor's model, size, modes, and exposure and gain ranges, and yields 8-bit gray
  frames, each with its exposure start and end on the node's clock. The port SHALL refuse
  an exposure or gain outside the reported range with an error naming the range. Every
  camera SHALL be reached through an adapter behind this port. **Refs**: ADR-0001 (ports
  and adapters); GLOSSARY.md (camera port, camera adapter).
- **FR-005**: The rpicam adapter SHALL run `rpicam-vid` as a subprocess writing YUV420
  frames and their metadata, and take each frame's Y plane at the row stride the camera
  writes, with exposure start from the sensor timestamp and end from the exposure time.
  It SHALL read the camera's model and modes from `rpicam-hello --list-cameras`. When the
  subprocess exits, it SHALL report the error and its stderr, and the node SHALL restart
  the camera without restarting the service. **Refs**: Raspberry Pi camera documentation
  (rpicam-apps); libcamera's `SensorTimestamp`;
  `docs/research/2026-10-21-scope-node-runtime.md` §2.
- **FR-006**: The scope node SHALL find stars with the same star detection the console
  uses, with cedar's detection settings by default, so that for the same frame and the
  same detection settings the node's centroids equal the console's. **Refs**:
  star-detection's parity with cedar-detect; `plate_solver::DetectParams::cedar()`.
- **FR-007**: The scope node SHALL read its listen address, token, camera adapter
  (`rpicam` or `simulated`) and starting camera settings from a config file, and SHALL
  refuse to start on an unknown key or a bad value, naming it. `scope-node --check-config`
  SHALL print the config it would run with, the token masked as `****`, and exit.
  **Refs**: systemd.service(5); GLOSSARY.md (scope node).
- **FR-008**: Of the workspace's crates, `scope-node` SHALL depend on `star-detection` and
  `scope-link` only, so the Pi never links the solver or the pattern database, and its README SHALL
  document the static `aarch64-unknown-linux-musl` build with its target and flags on the
  command line, the systemd unit and the config file's mode 0600. The repository SHALL
  gain no `.cargo/config.toml`. **Refs**: `scripts/ci/e2e-eval.sh` (refuses a repository
  `.cargo/config.toml`).

**The simulator** (US3):

- **FR-009**: The simulator SHALL have a camera adapter that renders 8-bit frames of the
  pattern database's stars for wherever its simulated telescope points at its site and
  virtual time, with pixel noise from a seed, so the same inputs render the same frame.
  **Refs**: GLOSSARY.md (simulator); the pattern database's star table, brightest first.
- **FR-010**: The simulator SHALL have a known finder camera (a 1456 × 1088 sensor and a
  12° horizontal field by default, inside the bundled database's range, settable) and an
  eyepiece direction offset from the camera's centre to a known pixel, (700, 420) by
  default, settable, and SHALL expose its true camera and eyepiece directions to tests.
  **Refs**: GLOSSARY.md (aim point, boresight);
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §2.
- **FR-011**: The simulator's `VirtualClock` SHALL read a start instant plus the elapsed
  `tokio::time::Instant`, so it advances only as tokio's time does in paused tests and at
  the real rate otherwise. **Refs**: GLOSSARY.md (simulator); 012's console clock.

**The node's service** (US4):

- **FR-012**: The node SHALL stream one star frame per exposure, each with an id that
  grows by one per exposure; exposure start and end on the node's clock; frame size,
  exposure and gain; the detection settings used; and up to 200 centroids, brightest
  first, in full-frame pixels. **Refs**: ADR-0001 (stars, not frames, cross the Wi-Fi).
- **FR-013**: The node SHALL send with a star frame a JPEG preview of it, at most 640 px
  on its longer side, no more often than once every 5 s unless the console asks, and SHALL
  send one with the next frame after the console asks. **Refs**:
  `docs/research/2026-10-21-scope-node-runtime.md` §3.
- **FR-014**: The node SHALL detect with cedar's detection settings unless the console
  sends others, and SHALL apply the camera and detection settings the console sends from
  the next exposure on. Each star frame SHALL report the settings it was made with.
  **Refs**: FR-006; FR-019.
- **FR-015**: The node SHALL adjust whichever of exposure and gain the console has left
  automatic so frames hold 25 to 40 detected stars, aiming at 30. Exposure and gain the
  console has set SHALL be held as set. **Refs**:
  `docs/research/2026-10-21-pointing-model-and-tracking.md` §2.
- **FR-016**: The simulator SHALL run the real `scope-node` service over its simulated
  camera, reachable in-process over an in-memory transport and over loopback TCP.
  **Refs**: GLOSSARY.md (simulator).

**The console's link and the session** (US5):

- **FR-017**: The console SHALL count the node as connected while heartbeats arrive and
  as disconnected once 1 s has passed without one, and SHALL then reconnect, waiting at
  most 5 s between tries. It SHALL report the link's state, the latest frame's age and the
  clock offset. *(Owner approval 1: `finder` now builds `scope-link`'s generated code.)*
  **Refs**: GLOSSARY.md (scope node); ADR-0001.
- **FR-018**: The session SHALL hold the scope node's address and token, set by
  `PUT /api/session` like its other fields. The token SHALL be write-only: reads SHALL
  say only whether one is set. The state file SHALL keep both, readable and writable by
  its owner only, and only once they are set, so a session that never pairs a node
  writes the same file 012 writes. An address that is not a host and port SHALL be
  refused with 400. **Refs**: FR-001; 012's checked edits and state file.
- **FR-019**: The session SHALL hold the finder camera's exposure (automatic, or
  milliseconds within the camera's range), gain (automatic, or a value within its range)
  and detection threshold, keep them across restarts, and send them to the node when they
  change and when it connects. Until one of them is set, a read SHALL leave
  `camera_settings` out, and the node SHALL run with automatic exposure and gain and
  cedar's threshold. A value outside a connected camera's range SHALL be refused with 400
  naming the range. **Refs**: FR-014; FR-015.
- **FR-020**: When a node connects whose camera's range excludes a stored exposure or
  gain, the console SHALL send automatic for that setting, keep the stored value, and
  report in the session that the stored value is outside the camera's range. **Refs**:
  FR-019.
- **FR-021**: `GET /api/session` SHALL carry a `node` with the address, `token_set`,
  `connected`, the latest frame's age, the clock offset, the camera's model and ranges,
  and the latest frame's star count. While the session holds no node address or token
  and no node is connected, the read SHALL leave `node` out, so a fresh session reads as
  012 pins it. **Refs**: plan.md § Interfaces.
- **FR-022**: The console's link client SHALL reach the simulator through exactly the
  calls and code it uses for a real node, so the console cannot tell them apart. **Refs**:
  FR-016.

**The live loop** (US6):

- **FR-023**: While a scope node is connected, the console SHALL solve the newest star
  frame each time it is free, dropping frames that arrived while it was busy, and SHALL
  use the last solved field of view as the estimate for the next solve (5% error
  allowance), dropping the estimate after three solves in a row fail. **Refs**:
  `plate_solver::solve_from_centroids` (the field-of-view estimate); GLOSSARY.md
  (lost-in-space).
- **FR-024**: Each live pointing's frame time SHALL be its frame's mid-exposure, mapped
  from the node's clock to the console's with the link's clock offset, and its altitude
  and azimuth SHALL be computed for that time. The pointing SHALL be taken at the aim
  point, unprojected through the solution's camera and distortion. **Refs**: FR-003;
  012's latest pointing.
- **FR-025**: The console SHALL set the aim point from a tap on a live frame to the
  detected centroid of that frame nearest the tap within 10 px. A tap with no centroid
  within 10 px SHALL set nothing and say why. **Refs**: GLOSSARY.md (aim point); Cedar's
  and StarSense's calibration (research note §2).
- **FR-026**: The session SHALL hold the aim point as a pixel position in the finder
  camera's frame, kept across restarts until it is reset; with none set, the frame centre
  SHALL be used, and a read SHALL leave `aim` out. **Refs**: GLOSSARY.md (aim point);
  012's latest pointing.
- **FR-027**: A pointing from the live loop SHALL go stale for push-to guidance once it is
  more than 5 s old by the console's clock, whether or not the node is still connected,
  and the guidance SHALL then say that the pointing is stale instead of giving a turn and
  tilt. **Refs**: 012's turn and tilt, and its rule that a pointing goes stale only where
  a requirement says so; FR-017.
- **FR-028**: Once the live loop has received a frame, `GET /api/session` SHALL report the
  latest solved field of view, the loaded database's field range, and how long frames
  holding at least 10 stars have gone unsolved. Before then the read SHALL leave `field`
  out. **Refs**: 012's database report in `/api/health`; FR-037.

**`finder --simulate`** (US7):

- **FR-029**: `finder --simulate` SHALL run the console with an in-process simulator as
  its scope node, on a virtual clock that starts at `--sim-start` (default now) and runs
  at the real rate, and SHALL never connect to a real scope node, whatever address the
  session holds. `--sim-start` without `--simulate` SHALL be refused with a nonzero exit
  naming it. **Refs**: 012's console clock; GLOSSARY.md (simulator).
- **FR-030**: `finder --simulate` SHALL keep its session in its own state file, under a
  `simulate` folder within the default state folder unless `--state` is given, and SHALL
  never write the default state file. **Refs**: 012's state file; FR-026.
- **FR-031**: Only under `finder --simulate`, the console SHALL offer simulator controls,
  behind 012's same-origin check: one that moves the simulated telescope by hand, to an
  altitude and azimuth or by a turn and tilt, as an observer pushes a Dobsonian; one that
  reads its true camera and eyepiece directions; and one that stops and starts its frames.
  Without `--simulate` every such route SHALL answer 404 and move nothing. **Refs**:
  GLOSSARY.md (push-to, turn, tilt); FR-010.
- **FR-032**: While a node is connected, `GET /api/live` SHALL send one server-sent event
  per live solve carrying its live report, and `GET /api/live/preview.jpg?seq=N` SHALL
  answer the preview a frame carried. **Refs**: plan.md § Interfaces.
- **FR-033**: The whole live loop SHALL be proven in one runtime, in paused time, from the
  first frame through a calibrating tap to push-to guidance, against the simulator's true
  eyepiece direction. **Refs**: FR-023 to FR-027.

**The page** (US8, US9):

- **FR-034**: While a scope node is connected, the page SHALL update its chart with each
  live solve over the server-sent event stream and show the node's previews under the
  frame through night mode's red-only filter. With no node connected the page SHALL open
  no stream and behave as it did before. **Refs**: DESIGN.md (the Red Torch Rule); the
  finder console's offline behaviour.
- **FR-035**: While a scope node is connected, the page SHALL stop showing a turn and
  tilt, and say that the pointing is stale, within 1 s of the newest live pointing passing
  5 s old. **Refs**: FR-027; 012's guidance block.
- **FR-036**: The Setup page SHALL have a scope node and camera section that sets the
  node's address and token (never showing the token) and the camera's exposure, gain and
  detection threshold, and shows the link's state, the camera's model, the latest frame's
  star count and the aim point with its reset. **Refs**: FR-018, FR-019, FR-021;
  DESIGN.md.
- **FR-037**: The Setup page SHALL show the finder camera's solved field of view beside the
  loaded pattern database's range, and SHALL warn that a database built for the camera's
  field may be needed when the field lies within 1° of either end of the range or outside
  it, or when frames holding at least 10 stars have not solved for 30 s. **Refs**:
  FR-028; `database-generation`; PRODUCT.md (the finder camera's field).
- **FR-038**: The Setup page SHALL let the observer, with a star centred in the eyepiece,
  tap that star on its live frame, sending the tapped pixel with the frame's id, and SHALL
  say why when the console sets nothing. **Refs**: FR-025.
- **FR-039**: The Setup page SHALL say when a stored exposure or gain is outside the
  connected camera's range. **Refs**: FR-020.
- **FR-040**: `DESIGN.md` and `.impeccable/design.json` SHALL record the live chart, the
  preview under the frame, the stale state, Setup's scope node and camera section and the
  aim-point tap, as built. **Refs**: DESIGN.md; `.impeccable/design.json`.

**End to end** (US10):

- **FR-041**: *(Owner approval 2.)* The end-to-end suite SHALL run a third server,
  `finder --simulate` with its own fresh `--state`, a fixed `--sim-start` and
  `reuseExistingServer: false`, serving a `simulate` project for the live tests, leaving
  the earlier projects, servers and tests unchanged. **Refs**: 012's end-to-end
  configuration.
- **FR-042**: The end-to-end suite SHALL prove in Chromium the chart following a push, the
  stale state when frames stop, Setup connected to the simulator, the aim-point tap, the
  night pixel check on the live chart, and that a console with no node opens no stream.
  **Refs**: FR-034 to FR-038.

### Key Entities

- **Star frame**: id (one per exposure), exposure start and end on the node's clock,
  width and height, exposure, gain, the detection settings used, up to 200 centroids
  (x, y, brightness) brightest first, and an optional JPEG preview.
- **Heartbeat exchange**: t0, t1, t2, t3; its delay and offset. The console keeps the last
  8 s of them.
- **Camera info**: model, sensor size, modes, exposure range and gain range.
- **Camera settings**: exposure (automatic or milliseconds), gain (automatic or a value),
  detection threshold (σ).
- **Scope node in the session**: address, `token_set`, and the link status: `connected`,
  the latest frame's age, the clock offset, the camera, the latest star count, and any
  notice that a stored setting is outside the camera's range.
- **Aim point**: a pixel (x, y) in the finder camera's frame, or none (the frame centre).
- **Live report**: the pointing's sequence number, frame id, time, RA and Dec, altitude
  and azimuth when a site is set, the solved field, the matched stars, and the preview's
  sequence number when the frame carried one.
- **Simulator**: its optics (sensor, field, eyepiece pixel), its telescope's true
  altitude and azimuth, its site, its `VirtualClock`, and its noise seed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario above has a committed test that asserts it, and
  `cargo test --workspace --locked`, `npm test` and `npm run e2e` pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file or
  `CODEOWNERS` entry is edited by any story. The two owner approvals are the only changes
  to tested behaviour or CI, and the `ci.yml` step lands before the spec is ready.
- **SC-003**: Workspace line coverage stays at or above the 80% floor after every story,
  with the three new crates included.
- **SC-004**: Every live pointing in the simulator's tests is within 30″ of the
  simulator's true eyepiece direction once the aim point is calibrated.
- **SC-005**: No build of `scope-node` links `plate-solver`, `pattern-database` or
  `finder`.

## Assumptions

- **The bring-up fixtures and notes are staged by the operator** before this spec is
  flipped to `ready`, in `fixtures/scope-node/`: the saved camera listing, the short saved
  YUV420 stream with padded rows and its JSON metadata, a saved gray frame from the finder
  camera, `fixtures.json`, and in its `README.md` the camera's model and modes, the row
  stride, which monotonic clock `SensorTimestamp` uses, the measured field of view,
  detection time per frame on the Pi 3B and Wi-Fi throughput. When the finder's own camera
  replaces the bring-up camera, the operator swaps these files, and no story changes.
- **The `finder-page` job's "Install protoc" step is on `main`** (owner approval 1),
  landed by the operator before this spec is flipped to `ready`.
- **The factory's coding agent can run cargo.** It has since ergane#896 (2026-10-28), and
  012's stories built and tested with it. Every story from US1 to US7 changes
  `Cargo.lock`, and US2 adds `toml`, which no hand edit of the lockfile can produce.
- **Outside the factory, at the telescope:**
  - building the static `aarch64-unknown-linux-musl` binary in WSL and checking that
    `file` reports a static aarch64 binary;
  - a pattern database for the finder camera's field, built with `database-generation`,
    if bring-up measures it outside 10–30°;
  - the on-sky checklist: the node starts on boot, pairs, streams 1–5 frames a second; the
    frames solve; the aim point calibrates on a star centred at high power; push-to
    guidance brings a target into the eyepiece.
- **Done by the maintainer after US9 lands:** the Impeccable review of the live chart and
  Setup's new section.
- **Deferred to later specs or out of scope:**
  - the mount: the mount port, the Sky-Watcher adapter and the simulator's mount half
    (014);
  - hinted solving: every frame is solved lost-in-space with only the field-of-view
    estimate;
  - TLS on the link, discovery by broadcast, and more than one node per console.
- **New dependencies**, each a critical-path change with its reason: `tonic`, `prost` and
  `tonic-prost-build` in the new crates, as `grpc-service` already uses them; `toml` for
  the node's config file; `image`'s JPEG encoder in `scope-node` for previews. Each must
  build on Rust 1.96 and pass `cargo-deny` and `cargo-machete`.
- **The factory's gates and CI:**
  - The factory's local gates are `lint` and `test`. `lint` runs rustfmt, clippy and the
    docs builds with warnings denied. `test` runs the cargo tests and doctests and then,
    in `crates/finder/web`, the checks CI's required `finder-page` job runs: `npm ci`,
    `npm run typecheck`, `npm run format:check`, `npm test`, `npm run build` followed by
    the build-freshness step (`git status --porcelain -- dist` lists nothing), and
    `CI=1 npm run e2e`. Every story's gate results carry them.
  - `crates/finder/web/dist/**` is marked generated in `.gitattributes`, so a rebuilt
    bundle appears in a story's diff as changed files rather than as minified text.
  - The merge queue runs every `ci.yml` job. Some have no factory counterpart: the
    coverage floor, the cedar e2e eval and canary, `cargo machete`, `cargo deny`, the
    secret scan and the build on Rust 1.96 (`msrv`). The factory builds on stable Rust,
    and its toolchain holds neither cargo tool.
  - Every story's tasks still run the gates' checks before proposing.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-004, FR-005, FR-006, FR-007, FR-008]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-009, FR-010, FR-011]
US4:
  depends_on: []
  depends_on_merged: [US3, US2]
  implements: [FR-012, FR-013, FR-014, FR-015, FR-016]
US5:
  depends_on: []
  depends_on_merged: [US4]
  implements: [FR-017, FR-018, FR-019, FR-020, FR-021, FR-022]
US6:
  depends_on: []
  depends_on_merged: [US5]
  implements: [FR-023, FR-024, FR-025, FR-026, FR-027, FR-028]
US7:
  depends_on: []
  depends_on_merged: [US6, US5]
  implements: [FR-029, FR-030, FR-031, FR-032, FR-033]
US8:
  depends_on: []
  depends_on_merged: []
  concurrent_with: [US7]
  implements: [FR-034, FR-035]
US9:
  depends_on: []
  depends_on_merged: [US8]
  implements: [FR-036, FR-037, FR-038, FR-039, FR-040]
US10:
  depends_on: []
  depends_on_merged: [US9, US7]
  implements: [FR-041, FR-042]
```

Every edge is a merge edge, because each dependent builds on what its predecessor
creates:

| Story | Builds on |
|---|---|
| US2 | US1: `scope-node` serves `scope-link`'s service and both change `Cargo.lock` |
| US3 | US2: the simulator's camera is an adapter behind `scope-node`'s camera port |
| US4 | US3: the node's service is proven over the simulated camera in `crates/simulator/tests/` |
| US4 | US2: it extends the node files US2 creates: `crates/scope-node/Cargo.toml`, `src/lib.rs`, `src/capture.rs` and `src/main.rs` |
| US5 | US4: the console's link client is tested against the simulator's node service |
| US6 | US5: the live loop reads frames from the link client and settings from the session |
| US7 | US6: `--simulate` runs the live loop on the in-process simulator |
| US7 | US5: it changes `crates/finder/Cargo.toml` and `Cargo.lock` again, after US5, the last story to change them |
| US9 | US8: it adds to the page US8 changed and rebuilds the `web/dist` US8 rebuilt |
| US10 | US9: it drives the finished page |
| US10 | US7: it drives that page on `finder --simulate`, which US7 adds |

The graph has two tracks, and the factory can keep a story from each in flight. It has
one coding agent, so the two never build at the same moment: while one story builds, the
other is verified or in the merge queue.
The first seven stories are a chain: each compiles against the crate or feature the one
before it adds, each changes `Cargo.lock`, and US5 to US7 all edit
`crates/finder/src/api.rs` and `src/session.rs`. The page stories, US8 and US9, write only
`crates/finder/web/**`, which no Rust story touches, and their tests fake the server from
plan.md § Interfaces, so they build beside the first track. US9 follows US8 because both
edit `web/src/api.ts` and rebuild `web/dist`. US10 joins the tracks: it drives the
finished page on the simulator.

`concurrent_with: [US7]` on US8 overrides one edge the factory would otherwise infer. Both
stories' tasks name the route `/api/live/preview.jpg`, which it reads as a shared file;
they share no file. Keeping two stories in flight also needs plan.md's Prerequisite 6, so
that one story's end-to-end run never contends with the other's for the same ports.

The edges from US2 to US4 and from US5 to US7 add no order the chain does not already
give. They name the two pairs of stories that share a file without being neighbours, so
that every shared file has a stated order (plan.md § Refinement Record).
