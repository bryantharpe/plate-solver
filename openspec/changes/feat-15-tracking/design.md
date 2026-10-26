## Context

See proposal.md for why. After feat-14:
- the console has a pointing model (zero points, tilt, fixed terms) and refraction;
- the mount port can move each axis at a signed rate, and its adapter stops an axis before
  reversing it;
- GoTo leaves the target within 1′ of the aim point;
- every solve is a sync, and the simulator's mount, on a virtual clock, exposes its true
  eyepiece direction.

feat-12 to feat-14 must be archived before this change; its deltas only ADD.

**The mount.** Its board runs only constant rates, so the host computes alt-az tracking and
sends new rates as the sky turns.

**The rates** (research note §3):
- dh/dt = −ω cos φ cos δ sin H / cos h;
- dA/dt = ω (sin φ − sin δ sin h) / cos² h;
- dq/dt = −ω cos φ cos A / cos h, with A from north through east (Meeus's own formula uses
  azimuth from the south, hence the sign), so dq/dt is +33.69°/h at φ 40°, δ 20°, transit;
- all checked by hand at φ 40°, δ 20°.

## Goals / Non-Goals

**Goals:**
- Hold a visual target at the aim point indefinitely, except for a short gap near the zenith.
- Report what an imaging camera can do (field rotation, the longest exposure), without
  derotating.
- Prove the loop in virtual time over long runs without slowing the coverage job.

**Non-Goals:**
- Field derotation and guiding from the imaging camera. Flexure between the finder camera and the tube limits
  imaging-grade guiding anyway (research note §3).
- Tracking objects that move against the stars (planets, the Moon), which aren't targets yet.

## Decisions

### Feed-forward from the model, sent every second

Each second, the tracker:
1. takes the target's apparent place now and 1 s ahead;
2. inverts the model for both, to axis positions;
3. sends each axis the rate that covers the difference in that second, plus the correction
   below.

That carries every model term (tilt, CA, NPAE, TF) into the rates, which plain dh/dt and dA/dt
wouldn't. The rates in `sky-ephemeris` are used for the readout, the keyhole forecast and the
tests' oracles.

### Closed loop: PI on the error, resolved into the axes

At least one solve every 5 s (the live loop gives about two a second) gives the error between
the target and the aim point. Following Wallace (2002):
1. rotate the error by the parallactic angle into cross-elevation and elevation;
2. divide the azimuth part by cos E.

**Control.**
- A proportional gain of 0.6 on each axis, applied as a rate over the next interval;
- a slow integral (time constant about 60 s), held as a tracking offset per axis and cleared
  when tracking ends. It is kept apart from IA and IE because every tracking solve is also a
  sync, and the zero-point refit would undo anything folded into them;
- errors under 10″ ignored (deadband);
- the integral clamped to 5% of the sidereal rate: well above the 1% error the loop is designed
  for, low enough that a fault can't drive the axes away.

Gains are fixed constants, tuned on the simulator. They aren't settings.

*Alternative:* a model refit only (no PI). Rejected: refits need spread syncs, and tracking
gives syncs along one track.

### Backlash at reversals

The altitude axis reverses at every transit, and the azimuth axis at greatest elongation for
any target with δ > φ (at φ 40°: M81, M82, M101, Capella), where sin H = sin φ / sin δ. The
simulator's 0.3° of backlash (1,080″) on each axis would throw the target far out of a
high-power eyepiece.
- **Measuring.** At an axis's first reversal of the mount's life, the closed loop sees the
  error jump along that axis; the size of the jump, settled over the next solves, is that
  axis's backlash. Later reversals refine it.
- **Keeping.** The measurement is kept with the fixed terms, since it belongs to the mount.
- **Syncs.** A GoTo's last approach is from below, so its syncs carry the backlash on the upward
  side. A sync taken while an axis last moved the other way (tracking down after transit) has
  that axis's measured backlash applied before fitting, and is held back until one is measured;
  otherwise up to 0.3° would bias IE without reaching the 0.5° slip threshold.
- **Taking it up.** Before a planned reversal of either axis (its feed-forward rate changing
  sign within the 30-minute forecast), the tracker stops the axis, drives it through the measured backlash at 0.5°/s, then resumes the
  feed-forward rate. Until a backlash is measured, the closed loop recovers within about 15 s.

### The zenith keyhole: wait at the exit

**The forecast.** The tracker forecasts the target's path for the next 30 minutes. When the path
enters the 2° circle round the zenith:
1. it stops at the entry;
2. it slews ahead (feat-14's planner, Sun guard included) to where the path leaves the circle;
3. it waits;
4. once the target's forecast position is outside the circle, it solves, applies the error, and
   resumes.

**Why wait.** A target 1° from the zenith spends about 18 minutes in the 2° circle, so waiting at
the exit beats chasing it. The azimuth rate there is only about 0.1°/s at 2°, but it changes
fast, and the plan's rule keeps the loop out of that region.

### Ending

The tracker checks four conditions every second. Any of them stops both axes and records why:
- Stop;
- a lost link (feat-13's 1 s rule; the node's watchdog also stops the axes);
- the Sun's centre above −0.83°;
- the target's geometric altitude below the minimum altitude, as pointable is judged.

The session reports the reason; the page shows it.

### The rate-error scenario

The plan's "0.5% rate error drifts more than 1′ in 10 min" does not hold: 0.5% of the 8,985″
M42 moves in 10 minutes is about 45″. The scenario uses 1% (about 90″), pending the owner's
approval.

### Track and GoTo together

`POST /api/track {"on": true}` arms tracking for the session's target:
- if a GoTo is running, tracking starts when it completes;
- if the target is more than 1′ from the aim point and no GoTo is running, a GoTo is started
  first;
- `{"on": false}` stops tracking, and the axes, leaving the telescope where it is.

### The readout

The session's tracking block reports:
- the RMS error over the last 60 s;
- the field rotation at the target, from dq/dt, in °/min;
- with an imaging camera set, the longest exposure for 1 px of smear at the sensor's corner:
  t = 1 / (r · |dq/dt|), with r the half-diagonal in pixels and dq/dt in radians per second;
- "no limit" when |dq/dt| is below 1e-9 rad/s.

### Tests: truth plus noise for long runs

Rendering and solving a frame every 2 s for an hour of virtual time would dominate the coverage
job. Long tracking tests instead feed the console the simulator's true eyepiece direction, plus
Gaussian noise (σ 3″), as its pointings, through the same code path a solve would take after
the solver. A short test (2 minutes) runs with rendered, solved frames, to prove the two paths
agree. The scenarios' oracles are the simulator's truth.

## Risks / Trade-offs

- **Motor firmware below 3.39** reportedly tracks unevenly at variable rates. → feat-14 warns at
  connect. The closed loop absorbs small unevenness, and bring-up records the firmware.
- **Rate updates every second on a 9600-baud single-wire link.** Each update is a few short
  commands per axis, well under the link's capacity. → Measured at bring-up.
- **The altitude axis reverses at transit;** stop-before-reverse costs a fraction of a second,
  and backlash far more. → The first reversal measures the backlash, and later ones take it up
  ahead of time, so the error stays within the 30″ the scenario allows. A real mount's backlash
  may vary along the axis; the closed loop absorbs what the measurement misses.
- **Imaging at 1800 mm on an alt-az mount needs short subs and derotation when stacking.** The
  readout states the limit; derotation is out of scope.
- **A 1% rate error** (a slipping clutch, a wrong counts-per-turn) is the closed loop's design
  case. Errors far beyond it point to a fault that the model and the slips rule catch at the
  next sync.

## Migration Plan

Nothing to migrate. Without a mount, the Track toggle isn't offered. Tracking is off at start;
the session keeps no tracking across a restart.

## Open Questions

- The PI gains for the real mount. The simulator's are the starting point, and the on-sky
  checklist confirms or retunes them (constants in code, not settings).
