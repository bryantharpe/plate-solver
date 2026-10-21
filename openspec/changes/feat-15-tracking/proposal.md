## Why

After a GoTo the target sits at the aim point, then drifts out of the eyepiece within a minute or
two, because an alt-az mount must turn both axes at changing rates to follow the sky, and the
mount's board runs only constant rates. This change makes the console follow the target: rates
from the pointing model, corrected by the finder camera's solves, for visual use and pushed far
enough toward imaging to report field rotation and the longest exposure for an imaging camera.

## What Changes

- **Alt-az rates and field rotation** in `sky-ephemeris`: the altitude and azimuth rates of a
  position, the rate of field rotation, and the longest exposure an imaging camera allows for a
  given smear.
- **Tracking:**
  - feed-forward axis rates from the pointing model, updated at least once a second;
  - corrected by proportional-integral control from the finder camera's solves every 1–5 s, with a 10″
    deadband;
  - before an axis reverses (the altitude axis at every transit), it takes up that axis's
    backlash, measured at the first reversal and kept with the fixed terms;
  - near the zenith (within 2°), it lets the target pass and picks it up where it leaves;
  - it ends safely, and says why, on Stop, a lost link, sunrise, or the target sinking below the
    minimum altitude.
- The **simulator** gains a rate-error setting, so tests can show that feed-forward alone
  drifts and the closed loop holds.
- The session gains the Track state and the imaging camera's sensor size and pixel pitch. The
  page gains a Track toggle with a readout of the tracking error, field rotation and longest
  exposure, and Setup an imaging camera section.

## Capabilities

### New Capabilities

- `tracking`: the mount following the target so it stays at the aim point: feed-forward,
  closed-loop correction from solves, the zenith keyhole and safe ending.

### Modified Capabilities

All introduced earlier in this set; each delta only ADDs:

- `sky-ephemeris`: alt-az rates, field rotation and the longest exposure.
- `simulator`: a rate-error setting.
- `observing-session`: the Track state, the imaging camera and the mount's measured backlash.
- `finder-console`: the Track toggle and readout, and Setup's imaging camera section.

## Impact

- **Code:** `finder::tracking` and the rates in `finder::ephemeris`; `POST /api/track`; the
  simulator's rate error. No change to the scope node or the link: tracking uses feat-14's
  move-at-rate and stop.
- **Behaviour on the mount:** the altitude axis reverses at transit, relying on feat-14's
  stop-before-reverse; firmware below 3.39 may track unevenly (warned at connect since feat-14).
- **Tests:** long tracking runs are proven in Rust in virtual time, feeding the console the
  simulator's true pointing plus noise rather than solving rendered frames, so the coverage job
  stays fast. Playwright adds a Track flow to the `simulate` project. No existing test or CI
  file changes.
- **Out of scope:** field derotation (the readout says how long an exposure may be instead), and
  guiding from the imaging camera.
