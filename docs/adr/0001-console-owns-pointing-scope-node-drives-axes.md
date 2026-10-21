# The console owns pointing; the scope node only captures, detects and drives two axes

A Raspberry Pi riding on the telescope (the scope node) captures finder frames, finds the stars in them, and drives the mount's two axes through Sky-Watcher's motor-controller protocol over a USB cable plugged straight into the mount, with no hand controller in between. The console on the Surface does everything else: plate solving, the pointing model, GoTo, tracking and the page. We chose this split so that no cable runs from the turning telescope to the laptop, the maths runs on the faster machine, and the Pi never needs the pattern database. The console reaches hardware only through hardware-neutral ports: a mount port in which each axis can read its position, go to a position, move at a rate, stop, and report what it can do; and a camera port. Supporting another mount or camera means writing an adapter, not changing the logic.

## Considered Options

- **The SynScan hand controller's own GoTo and tracking**, or any "go to this RA/Dec" (sky-level) port. Rejected: these need the mount's own star alignment, which is what this project replaces, and they keep a pointing model we can't see or refine from plate solves.
- **The whole console on the Pi.** Rejected for now: the owner wants the maths on the Surface, and a Pi 3B is slow and short of memory for the solver and its database.
- **The Pi only streams frames, and the Surface detects stars.** Rejected: star lists are tiny over the Pi 3B's 2.4 GHz Wi-Fi where full frames are not, and the detector was built for Pi-class boards.
