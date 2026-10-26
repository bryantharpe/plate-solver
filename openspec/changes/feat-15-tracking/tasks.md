## 1. Rates and field rotation

- [ ] 1.1 Add alt-az rates and field rotation to `finder::ephemeris`; verified by the "At transit" and "Two hours before transit" tests
- [ ] 1.2 Add the longest exposure for an imaging camera; verified by the "A 23.5 × 15.6 mm sensor at transit" and "Due east" tests
- [ ] 1.3 Add the simulator's rate error, backlash setting and test hook to load its truth into the model; verified by the "One per cent fast", "No backlash" and "The truth loaded" tests

## 2. Tracking loop

- [ ] 2.1 Add feed-forward rates through the model, sent every second; verified by the "Rates follow the sky" test in virtual time
- [ ] 2.2 Add the PI correction from solves with the 10″ deadband and the integral kept as a separate tracking offset; verified by the "A mount that runs fast" and "Inside the deadband" tests, fed truth plus noise
- [ ] 2.3 Measure backlash at reversals, keep it in the session and take it up before later reversals; verified by the "The first reversal measures the backlash", "Later reversals take it up", "An azimuth reversal", "Measured once, kept" and "Syncs after a transit" tests in virtual time
- [ ] 2.4 Add a short tracking test on rendered, solved frames; verified by the "Rendered frames agree with truth plus noise" test (within 5″ RMS)
- [ ] 2.5 End tracking safely; verified by the "Setting below the minimum altitude", "Stop while tracking", "Link lost while tracking" and "Sunrise" tests

## 3. Zenith keyhole

- [ ] 3.1 Forecast keyhole passages, wait at the exit and resume; verified by the "A target passing 1° from the zenith" test in virtual time

## 4. Page and Setup

- [ ] 4.1 Add `POST /api/track`, Track and the imaging camera to the session; verified by the "On after a GoTo", "Refused", "Track on starts a GoTo", "Track off", "Kept" and "An impossible pixel pitch" API tests
- [ ] 4.2 Add the Track toggle and readout to the guidance block; verified by Vitest tests for "Tracking ended" and the readout's rounding on a recorded session
- [ ] 4.3 Add Setup's imaging camera section; verified by Vitest tests for "A camera set on Setup" and "A pitch of zero"
- [ ] 4.4 Run the Impeccable pass on the Track toggle, readout and imaging camera section, and update `DESIGN.md`; verified by a ship verdict from the finish review

## 5. Virtual-time and end-to-end tests

- [ ] 5.1 Add a Rust end-to-end test of an hour of tracking on the simulator in paused time (1% rate error, a transit, a keyhole passage, then setting below the minimum altitude); verified by `cargo test -p finder` in under 30 s and the coverage floor holding
- [ ] 5.2 Add a Playwright Track flow to the `simulate` project ("Tracking M42", and the night pixel check with tracking on); verified by `npm run e2e` with every earlier test unchanged and green

## 6. On the sky

- [ ] 6.1 Run the manual checklist at the telescope: track three targets for 10 minutes each at medium power (one through transit), note the RMS readout and whether each stays in the eyepiece, and check the field rotation readout against a timed photograph; verified by the filled-in checklist, with logs, attached to the PR
