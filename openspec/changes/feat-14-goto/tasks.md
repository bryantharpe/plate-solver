## 1. Codec, adapter and emulator

- [ ] 1.1 Add the mount port to `crates/scope-node`; verified by the "The simulator's mount" test once 4.1 lands, and by the port's unit tests on a fake axis now
- [ ] 1.2 Add the motor-controller codec; verified by the "Positions on the wire" test and an exhaustive round-trip test over all 24-bit values
- [ ] 1.3 Add the byte-level emulator (115200-only, single-wire echo, `!2`, refused `:S`); verified by its own tests of each command it implements
- [ ] 1.4 Add the Sky-Watcher adapter (probe, initialise, rates, goto with the `:H`/`:M` fallback, stop-before-reverse, `!2` retry, auxiliary encoders off); verified by the "A goto on the wire", "Motor not stopped", "Reversing" and "The baud rate" tests against the emulator
- [ ] 1.5 Open the serial port with `serialport` (`default-features = false`) or termios, and add `scope-node --probe-mount`; verified by `cargo deny check`, the msrv build, and a musl cross-build of `scope-node`

## 2. Watchdog and Stop

- [ ] 2.1 Make Stop bypass the queue with an epoch counter; verified by the "Stop among moves" test
- [ ] 2.2 Add the 1 s heartbeat watchdog; verified by the "Silence" test in virtual time (1.0 ± 0.1 s)
- [ ] 2.3 Put axis positions at exposure start and end on star frames; verified by the "Positions bracket the exposure" test

## 3. Link

- [ ] 3.1 Add the mount calls to `scope_node.v1`, with stop all exempt from the token; verified by the "A move without the token", "Stop without the token" and "Axis state over the link" tests

## 4. Simulator mount

- [ ] 4.1 Add the simulated mount (tilted base, zero-point offsets, backlash, acceleration to 3.4°/s, true eyepiece direction) behind the port; verified by the "Backlash", "Top speed", "Stopping from top speed", "The base is off level" and "The simulator's mount" tests
- [ ] 4.2 Add pushes that the encoders miss and injectable slips; verified by the "A push the encoders miss" and "A slipping axis" tests

## 5. Refraction

- [ ] 5.1 Add apparent altitude (Saemundsson and Bennett) to `finder::ephemeris`, leaving `alt_az` geometric; verified by the "At the horizon", "At 45°", "There and back" and "Geometric stays geometric" tests, with `alt_az_matches_meeus_example_13b` unchanged

## 6. Pointing model

- [ ] 6.1 Take syncs from solves (still or tracking axes, mid-exposure interpolation, thinning); verified by the "A still telescope", "During a slew", "A frame uploaded by hand" and "Interpolated to mid-exposure" tests
- [ ] 6.2 Fit zero points and base tilt (exact offsets and rotation, Levenberg–Marquardt, Newton inverse); verified by the "Synthetic terms recovered" and "One sync is not enough" tests
- [ ] 6.3 Open new epochs on slips; verified by the "Pushed by hand" test after 7.2 and the "The second sync is not a slip" test
- [ ] 6.4 Fit and keep the fixed terms; verified by the "Synthetic fixed terms" and "Too few" tests
- [ ] 6.5 Keep the night's model state and the fixed terms in the session; verified by the "A restart at midnight", "The next day" and "Resetting the fixed terms" tests

## 7. GoTo

- [ ] 7.1 Plan slews (arrival time, refraction, model inverse, one-revolution cable rule, approach from below); verified by the "Unwinding the cable", "Unwinding after a power cycle", "Where the target will be", "Too close to the limit" and "Just clear of the limit" tests
- [ ] 7.2 Run the closed loop (settle, solve, correct at most 5 times) and the halts; verified by the "Five targets on the simulator", "A GoTo that cannot converge", "Stop during a slew", "The link drops during a slew" and "Sunrise during a slew" tests in virtual time
- [ ] 7.3 Apply the Sun guard to motion (the Sun up, path checks, re-planning); verified by the "Daylight" and "A slew at dusk" tests
- [ ] 7.4 Add `POST /api/goto`, `POST /api/stop`, the model routes and progress in the session; verified by the "Below the horizon" and "Progress" tests and the one-tap "From one sync to base tilt" test

## 8. Page and Setup

- [ ] 8.1 Add GoTo to the guidance block and Stop to the top bar of every page; verified by Vitest tests for "Not offered", "Stop from Setup", and a Stop that is never disabled while the mount may move
- [ ] 8.2 Add Setup's mount and model section with the second sync and resets; verified by Vitest tests for "After two syncs" on a recorded session
- [ ] 8.3 Run the Impeccable pass on GoTo, Stop and the model section, and update `DESIGN.md`; verified by a ship verdict from the finish review

## 9. End to end

- [ ] 9.1 Add a Rust end-to-end test of a night on the simulator in paused time: first sync, second sync, five GoTos, a hand push, a Stop and a dropped link; verified by `cargo test -p finder` and the coverage floor holding
- [ ] 9.2 Add a Playwright GoTo-and-Stop flow to the `simulate` project ("A GoTo from the page", "Stop from the page", "The second sync from Setup", and the night pixel check with GoTo running); verified by `npm run e2e` with every earlier test unchanged and green

## 10. On the sky

- [ ] 10.1 Run the manual checklist at the telescope: `--probe-mount` with the USB–TTL cable (board, firmware, counts per revolution recorded); Stop and the watchdog with the tube unloaded; the second sync; GoTos to five bright targets ending within 1′ in the eyepiece; verified by the filled-in checklist, with logs, attached to the PR
