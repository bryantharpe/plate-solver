## Why

With live solving (feat-13), the console knows where the 16-inch Dob points and where the target is,
but the observer still pushes it. The mount has motors, and per ADR-0001 the scope node can
drive its two axes. This change makes the console bring the target to the aim point itself,
without the hand controller's star alignment: every solve is a sync, a pointing model is
fitted from them, and the console slews, solves and corrects until the target is within 1′.

## What Changes

- A **mount port** on the scope node: two axes, each reporting its position and capabilities,
  going to a position, moving at a signed rate and stopping. A **Sky-Watcher adapter** speaks
  the motor-controller (`:`) protocol over the USB cable to the mount. Stop always wins, and a
  watchdog stops both axes 1 s after the console's last heartbeat. Star frames carry the axis
  positions at the start and end of each exposure.
- The **scope link** carries the mount calls. Stop needs no token, so anyone can halt the
  telescope; every move does.
- The **simulator** gains its mount half: a base 1° off level, zero-point offsets, 0.3° of
  backlash, a 3.4°/s top rate with 3°/s² acceleration, pushes the encoders miss, injectable
  slips, and its true eyepiece direction for tests.
- **Apparent altitude** in `sky-ephemeris`: refraction added (Saemundsson) and removed
  (Bennett). `alt_az` stays geometric.
- A **pointing model**:
  - every solve while the mount is connected is a sync;
  - the zero points come from one sync, and the base tilt from two syncs at least 90° apart
    in azimuth;
  - a slip opens a new zero-point epoch;
  - the fixed terms (CA, NPAE, TF and the finder camera's mounting) come from about 20 syncs
    and are kept between nights;
  - one tap takes a second sync.
- **GoTo**:
  - planning goes through refraction and the model, keeping the cable within one turn;
  - the Sun guard applies to every motion;
  - a closed loop slews at top speed, approaches from a fixed direction and corrects at most
    5 times, to within 1′;
  - Stop, a lost link or the Sun guard halt the mount;
  - `POST /api/goto` refuses a target that isn't pointable.
- The session keeps the night's zero points, tilt and syncs, and keeps the fixed terms. The page
  gains GoTo in the guidance block, Stop in the top bar of every page while the mount may move,
  and a mount and model section on Setup.

## Capabilities

### New Capabilities

- `pointing-model`: the fitted relation between the mount's axis positions and the sky: syncs,
  zero points, base tilt, slips, fixed terms and the one-tap second sync.
- `goto`: bringing the target to the aim point with the mount's motors: planning, the Sun guard
  on motion, the closed loop, halts and the API.

### Modified Capabilities

All introduced earlier in this set; each delta only ADDs:

- `scope-node`: the mount port, the Sky-Watcher adapter, Stop and the watchdog, and axis
  positions on frames.
- `scope-link`: the mount calls, and Stop without a token.
- `simulator`: the simulated mount.
- `sky-ephemeris`: apparent altitude.
- `observing-session`: the model's state for the night, and the fixed terms kept.
- `finder-console`: GoTo and Stop on the console, and Setup's mount and model section.

## Impact

- **Code:**
  - the mount port and the Sky-Watcher adapter in `crates/scope-node`, with a byte-level
    emulator of the mount for tests;
  - the simulated mount in `crates/simulator`;
  - mount calls in `crates/scope-link`'s proto;
  - `finder::model`, `finder::goto` and refraction in `finder`, with `POST /api/goto`,
    `POST /api/stop` and `POST /api/model/second-sync`.
  `crates/math-core` is untouched.
- **Dependencies:** `serialport` with `default-features = false` (MPL-2.0, allowed by
  `deny.toml`, no libudev), or a minimal termios open if it can't build for musl.
- **Hardware:** the scope node's USB cable plugged straight into the mount's hand-controller
  jack; Freedom Find (the auxiliary encoders) is switched off while the console drives.
- **Safety:** Stop needs no token and wins over every other command, both on the node and on
  the console. The Sun guard can't be overridden.
- **Tests:** GoTo, sync and the model are proven by Rust integration tests on the simulator in
  virtual time, so the coverage floor (Rust only) counts them. Playwright adds a GoTo flow to
  the `simulate` project feat-13 created. No existing test or CI file changes.
- **Size:** if this proves too big for one review, it splits into feat-14a (mount port,
  adapter, link, simulated mount) and feat-14b (refraction, pointing model, GoTo, page).
