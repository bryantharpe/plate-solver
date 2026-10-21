## Context

See proposal.md for why. After feat-13:
- the console has live pointings timed at mid-exposure, an aim point, and a scope link with
  heartbeats and clock mapping;
- the simulator is a real scope-node service over simulated adapters, on a virtual clock tied
  to tokio's.

feat-12 and feat-13 must be archived before this change, since every delta to their
capabilities here only ADDs.

**The mount** (`docs/research/2026-10-21-synscan-mount-control.md`): a Sky-Watcher Flextube 400P
SynScan.
- The scope node speaks the motor-controller protocol over a USB–TTL cable in the mount's jack,
  with no hand controller. RS-232 levels can damage the jack.
- The protocol: 9600 or 115200 baud; on most alt-az boards a single wire, so a TTL cable hears
  its own bytes.
- About 1,620,000 counts per turn on the motor encoder.
- A top rate of about 3.4–4.2°/s.
- Constant rates only, so the host computes alt-az tracking (feat-15).

**The model** (`docs/research/2026-10-21-pointing-model-and-tracking.md`):
- TPoint's alt-az terms;
- one solve fixes IA and only the sum of IE and the tilt along the line of sight;
- a second solve about 90° away fixes IA, IE, AN and AW;
- the fixed terms want about 20 well-spread observations.

## Goals / Non-Goals

**Goals:**
- GoTo to within 1′ with no star alignment, proven on the simulator in virtual time.
- A mount port small enough that another mount is an adapter, not a change to the logic.
- Safety first:
  - Stop always works and needs no token;
  - the watchdog stops the mount on a lost link;
  - the Sun guard holds on every motion.

**Non-Goals:**
- Tracking (feat-15). After a GoTo the axes stop.
- The hand controller's protocol, SynScan App, and the mount's own star alignment and GoTo, which
  ADR-0001 rejects.
- Field derotation. Altitude limits beyond the minimum altitude and the mechanical range the
  adapter reports.

## Decisions

### The mount port and its adapters

```
trait Mount {
  fn axes(&self) -> [AxisInfo; 2];          // counts per turn, top rate, can_goto
  async fn state(&self, axis) -> AxisState;  // position (counts), rate, moving
  async fn goto(&self, axis, position);      // absolute, in counts
  async fn rate(&self, axis, deg_per_s);     // signed
  async fn stop_all(&self);
}
```

Positions are in the axis's own counts; the console converts with counts per turn. Rates are
signed degrees per second, and each adapter turns them into its own units.

### The Sky-Watcher adapter, in three layers

**Codec.** Pure functions, tested exhaustively:
- 24-bit little-endian hex with the 0x800000 offset;
- framing `:` + command + axis + data + CR;
- replies `=data` / `!code`.

**Transport.**
- A byte stream: the serial port, or the in-memory byte-level emulator in tests.
- A command is written and its echo, if any, skipped: a TTL cable on a single-wire bus hears
  itself.
- The reply is read with a 300 ms timeout, with up to 5 tries for queries and other commands
  safe to repeat (INDI's practice).

**Protocol.**
- **Connect:** probe `:e1` at 9600 then 115200; then `:F3`, `:a`, `:b`, `:g`, `:q` (auxiliary
  encoder support) and `:W` (auxiliary encoders off).
- **Firmware:** below 3.39 is accepted, with a warning that smooth variable-rate tracking may
  not work.
- **Rates:** set as a step period `:I` = timer frequency × 360 / (°/s) / counts per turn, times
  the high-speed ratio in fast mode. Fast mode (`:G` codes 0/3) is used above 128× sidereal,
  as INDI does.
- **Goto:** `:G` (goto mode), `:S` (absolute target), `:J`. If `:S` is refused, the adapter
  falls back to `:H` (relative) and `:M` (brake point).
- **Reversing:** `:K`, then `:f` polled until stopped, then the new direction.
- **`!2` (motor not stopped):** the same stop-and-wait, and the command once more.

**Stop.** Stop is `:K` on both axes, a decelerating stop, gentler than `:L` on a 16-inch tube.
`:L` is sent if `:K` isn't acknowledged.

**Serial.** Opened with `serialport` (`default-features = false`, so no libudev; MPL-2.0 is in
`deny.toml`'s allow-list), or a few lines of termios through `rustix` if `serialport` doesn't
build for `aarch64-unknown-linux-musl`. Opening the device is the only untested shell.

**The emulator.**
- It implements the command set over the simulator's axis model.
- It can be told to:
  - answer only at 115200;
  - echo like a single wire;
  - inject `!2`;
  - refuse `:S`.
- It is what the adapter's tests and the oracle byte strings run against.

**A diagnostic.** `scope-node --probe-mount DEVICE` runs the research note's first-test
sequence (`:e1`, `:a1`, `:b1`, `:g1`, `:q1010000`, `:j1`) and prints the results, for
bring-up.

### Stop wins on the node

**Command order.** Mount commands go through one queue to the adapter's task. Stop bypasses
the queue:
- it raises an epoch counter;
- it clears the queue;
- it commands the stop at once.

A command carrying an older epoch is dropped when it reaches the adapter. That is "carried out
before any command that arrived with or after it".

**The watchdog.**
- A 50 ms tick compares now with the last heartbeat on the node's monotonic clock.
- At 1 s it calls `stop_all` and marks the link lost; a move needs a fresh request after the
  link returns.

### The pointing model: rigorous where large, TPoint where small

The encoder's zero points are arbitrary: IA can be anything. So the forward model
(axis positions → apparent alt/az) runs in four steps:
1. the axis angles from counts;
2. IA and IE as exact offsets;
3. the base tilt (AN, AW) as an exact rotation of the mount's frame;
4. CA, NPAE and TF as TPoint's small-angle terms (§7.2).

The finder camera's turn on the tube adds the solve's roll as a third equation per sync,
through the parallactic angle (Meeus ch. 14).

**Fitting.** Levenberg–Marquardt on residuals (ΔA·cos E, ΔE, Δroll), weighted by the solve's
RMS. It is written in-crate: at most 8 parameters, normal equations solved by Cholesky, no new
dependency.

**The inverse** (target → axis positions) is Newton's method on the forward model from the
uncorrected guess, converging in 3–4 steps.

**Epochs and nights.**
- Each night has its own tilt.
- Each zero-point epoch has its own IA and IE.
- The fixed terms are shared and kept.

They are fitted from one night's syncs once that night has at least 20 spread over 180° × 30°,
and refined on later nights that qualify. Syncs aren't archived across nights.

**Syncs.**
- Only star frames from the scope node carry axis positions, so a frame uploaded by hand
  updates the pointing but is never a sync.
- A frame counts if each axis's rate during the exposure is at most 0.2°/s. That rejects slews
  (3.4°/s) and keeps tracking, whose azimuth rate reaches about 0.1°/s at the edge of feat-15's
  2° zenith circle and is far lower elsewhere.
- Thinning keeps the newest sync when another lies within 2° and less than 2 minutes older.

**Slips.** A sync more than 0.5° from the prediction starts a new epoch. A mount power-cycled
mid-night shows up the same way, at its first sync.

### GoTo: a state machine in `finder::goto`

`Plan → Slew → Settle → Solve → (Done | Correct → Slew …) | Failed | Halted`

**Plan.**
1. Take the target's place at the estimated arrival time (two iterations of the slew-time
   estimate), add refraction, and invert the model.
2. Choose the azimuth axis value among ±360° candidates that stays within one turn of the
   night's first sync and is nearest. That unwinds the cable when needed.
3. Make each axis's last approach from below: overshoot by 0.5°, more than any backlash a
   mount like this has (the simulator's is 0.3°), then approach upward.

**The Sun guard.**
- The path is simulated from the axes' rates and acceleration, at 0.5° steps.
- If any point comes within 15° of the Sun, the planner tries altitude first, then azimuth
  first, then a dogleg through `max(current, target, Sun's altitude + 20°)`.
- If none keeps clear, the slew is refused.
- The Sun's altitude is also checked every second during motion. Reaching −0.83° halts.

**Settle and Solve.** Wait until both axes report stopped plus 0.5 s. The first frame whose
exposure starts after that is solved and taken as a sync.

**Correct.** The new sync refines IA/IE. If the error is 1′ or more, re-plan a small move,
still approaching from below. After 5 corrections the GoTo fails with the last error.

**Halted.** Stop, a lost link (the console's own 1 s rule, matching the node's watchdog) or the
Sun guard. Each sends `stop_all` and records the reason.

### API and page

**Routes.**
- `POST /api/goto` starts a GoTo to the session's target. It answers 409 (with the reason)
  when the target isn't pointable, there is no mount or no sync, or the Sun is up.
- `POST /api/stop` takes no body and skips the same-origin check, because a stop must never be
  refused.
- `POST /api/model/second-sync` and `POST /api/model/reset` (night or fixed); each move needs
  the same-origin check.
- Progress is reported with the session as `goto: {state, correction, error_arcsec, reason}`.

**The page.**
- The guidance block gains GoTo (a primary button when pointable) and Stop.
- Stop is in the top bar of every page, the console and Setup alike, whenever the session says
  the mount may be moving. It is never disabled, and never under a sheet.
- Setup gains the mount and model section.

### Tests in virtual time

Every GoTo, sync and model behaviour is proven in Rust integration tests that run the simulator
and the console in one paused-time tokio runtime, over the in-process link. A night of slews
takes milliseconds of CPU, and the coverage job (Rust only, 80% floor) counts it. Playwright's
`simulate` project adds one GoTo-and-Stop flow.

## Risks / Trade-offs

- **The cable into the mount's jack.**
  - Sky-Watcher's own USB adapter may enumerate as HID rather than serial.
  - RS-232 levels damage the jack.
  → Use a plain USB–TTL serial cable (FTDI, CP210x, CH340 or PL2303, all in the Pi's kernel)
  at TTL levels, and confirm with `--probe-mount` before any move.
- **Motor firmware below 3.39** may track unevenly (feat-15). → Warn at connect; record the
  version at bring-up.
- **1′ needs the aim point accurate to well under 1′.** At 30″ a pixel that is 2 px, so centre
  the calibration star at high power. A wider field on the real finder camera makes 1′ harder;
  bring-up records the scale, and the tolerance is revisited then if needed.
- **Mechanical limits:**
  - no published altitude range;
  - the rocker and ground board limit low altitudes;
  - no azimuth hard stop, so a power cable to the ground can wrap.
  → GoTo targets are pointable (above the minimum altitude); the one-turn rule unwinds the
  cable; the adapter reports any range limits it learns at bring-up.
- **Someone at the eyepiece when the tube moves.** → The console never moves the mount
  unasked: only GoTo, the second sync and (feat-15) Track move it. Stop is always on screen
  while it moves.
- **Freedom Find off** means a push by hand isn't counted. → Slips are detected at the next
  sync, and the GoTo still converges, as the "Pushed by hand" scenario proves.
- **Size.** If review finds this change too big, it splits into feat-14a (mount port, adapter,
  link, simulated mount) and feat-14b (refraction, model, GoTo, page) along the task groups
  below.

## Migration Plan

Nothing to migrate. Without a mount, the console behaves as after feat-13. At bring-up:
1. plug in the USB–TTL cable;
2. run `scope-node --probe-mount`;
3. enable the mount in the node's config;
4. make the first GoTos on bright targets with Stop at hand.

## Open Questions

- The 400P's board and motor firmware version, and its counts per turn. `--probe-mount`
  records them at bring-up, and they come from the mount, not from the code.
- Whether `serialport` builds for musl without libudev, or termios is needed. Settled when the
  adapter is first cross-built; either way the adapter's interface is the same.
