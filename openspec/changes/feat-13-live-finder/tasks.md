## 1. Bring-up on the Pi 3B

- [ ] 1.1 Record the owner's approval of the `finder-page` protoc step in `ci.yml` and the third Playwright server; verified by the approval on the PR before any code lands
- [ ] 1.2 Identify the Arducam (model, sensor, modes) on the Pi 3B with 64-bit Pi OS and save `rpicam-hello --list-cameras`; verified by the saved listing in `crates/scope-node/tests/data/` and a note in `crates/scope-node/README.md`
- [ ] 1.3 Capture and save a short `rpicam-vid` YUV420 stream with its metadata, confirm the row stride and which monotonic clock `SensorTimestamp` uses; verified by the saved fixture and the README note
- [ ] 1.4 Measure the finder camera's field of view by solving a captured night frame (building a matching database with `database-generation` if it is outside 10–30°); verified by a solved frame and the field recorded in the README
- [ ] 1.5 Measure detection time per frame on the Pi 3B and Wi-Fi throughput to the Surface; verified by the numbers recorded in the README beside the 1–5 frames a second that live push-to needs

## 2. Link

- [ ] 2.1 Create `crates/scope-link` with `scope_node.v1` built by `tonic-prost-build`, the cargo-machete ignore and the clippy allow; verified by `cargo clippy --all-features -D warnings`, `cargo machete` and `cargo doc` with `-D warnings`
- [ ] 2.2 Add the token interceptor; verified by tests for "No token or the wrong one" and "The paired console"
- [ ] 2.3 Add the clock-mapping maths and the heartbeat; verified by the "A worked exchange", "An old best exchange" and "Often enough" tests
- [ ] 2.4 Add the console's link client with health and reconnection; verified by the "The node goes away and comes back" test in virtual time
- [ ] 2.5 Add the `finder-page` protoc step once approved; verified by the `finder-page` job going green

## 3. Node and service

- [ ] 3.1 Create `crates/scope-node` with the camera port and the detection loop over `star-detection`; verified by tests for "The simulator's camera", "An exposure out of range" and "One frame, two places"
- [ ] 3.2 Serve star frames, previews and settings over the link; verified by tests for "What a frame carries", "A dropped frame shows", "Previews are rationed", "One on request", "Cedar's by default" and "A lower threshold"
- [ ] 3.3 Add the Arducam adapter (listing parser, YUV420 framing with stride, metadata, subprocess restarts); verified by tests for "A saved camera listing", "A saved stream with padded rows" and "The subprocess dies" (a fake `rpicam-vid` on `PATH`)
- [ ] 3.4 Add auto-exposure; verified by the "From too few stars", "Manual settings" and "Exposure set, gain automatic" tests on the simulated camera
- [ ] 3.5 Add the config file and `--check-config`; verified by the "Checking a config" and "A bad config" CLI tests
- [ ] 3.6 Document the musl build and the systemd unit in `crates/scope-node/README.md`; verified by building `--target aarch64-unknown-linux-musl` in WSL with the target and flags on the command line (no repository `.cargo/config.toml`, which `scripts/ci/e2e-eval.sh` refuses) and `file` reporting a static aarch64 binary

## 4. Simulator and `--simulate`

- [ ] 4.1 Create `crates/simulator` with the optics, rendering from the database's stars, the seeded noise and `VirtualClock`; verified by tests for "A frame at altitude 50°, azimuth 120°" and "Where the eyepiece looks"
- [ ] 4.2 Run the real node service over the simulated camera, reachable in-process and over loopback; verified by the "Indistinguishable on the link" test
- [ ] 4.3 Add `finder --simulate`, `--sim-start`, its own default state folder and the push control; verified by tests for "A night in January, at any hour", "Never the real node", "No virtual clock without the simulator", "A desk session leaves the night's alone", "A push" and "Not without the simulator"

## 5. Live loop

- [ ] 5.1 Solve the newest frame with the field-of-view estimate, timed at mapped mid-exposure; verified by tests for "Two frames a second for ten seconds", "A slow solver", "The field of view carries over" and "A worked timing"
- [ ] 5.2 Take the pointing at the aim point, and snap calibration taps to centroids on the server; verified by tests for "The simulator's offset recovered", "A tap on empty sky" and "Reset"
- [ ] 5.3 Add the node, camera settings and aim point to the session and state file; verified by tests for "A token is never read back", "Kept, privately", "A bad address", "Set once, sent on connect", "An exposure the camera can't take", "Camera settings kept", "A smaller range" and "Kept until reset"
- [ ] 5.4 Apply the 5 s freshness rule to live pointings, connected or not; verified by the "The node stops sending" and "The node goes away" tests

## 6. Page and aim-point calibration

- [ ] 6.1 Add `/api/live` (SSE) and previews, and open the stream only while connected; verified by Vitest tests that no `EventSource` is created without a node and one is created and closed with the link
- [ ] 6.2 Show the live chart and previews under the red-only filter; verified by Vitest tests on a recorded live report
- [ ] 6.3 Add Setup's scope node and camera section, the field-of-view warning and the aim-point tap; verified by Vitest tests for "A token kept out of sight", "Inside the range", "Near the edge" and "Frames that never solve"

## 7. End to end

- [ ] 7.1 Add a Rust end-to-end test: the simulator and the console in one runtime over an in-process link, in paused time, from first frame to calibrated aim point to push-to guidance; verified by `cargo test -p finder` and the coverage floor holding
- [ ] 7.2 Add the third Playwright server (`finder --simulate`, fresh `--state`, a fixed `--sim-start`, `reuseExistingServer: false`) and a `simulate` project for "Live from the simulator", "Frames stop on the simulator", "Connected to the simulator", the aim-point tap and the night pixel check on the live chart; verified by `npm run e2e` with every earlier test unchanged and green
- [ ] 7.3 Run the Impeccable pass on the live chart and Setup's new section, and update `DESIGN.md`; verified by a ship verdict from the finish review

## 8. On the sky

- [ ] 8.1 Run the manual checklist at the telescope: the node starts on boot, pairs, streams 1–5 frames a second; the frames solve; the aim point calibrates on a star centred at high power; push-to guidance brings a target into the eyepiece; verified by the filled-in checklist, with frames and logs, attached to the PR
