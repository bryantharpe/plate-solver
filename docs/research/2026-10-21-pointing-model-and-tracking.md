# Plate-solving GoTo for an alt-az Dobsonian: pointing model, closed loop, tracking

Research notes gathered on 2026-10-21 for feat-14 (GoTo) and feat-15 (tracking). They answer the owner's question of whether star alignment is needed once every frame is plate-solved. Sources are linked inline.

**Notation:**
- A: azimuth, north through east
- E or h: altitude
- φ: latitude
- H: hour angle, positive west
- δ: declination
- q: parallactic angle
- ω = 15.041″/s

**Markers:**
- **[sim]**: the researcher's own simulations, in pure Python, 300–400 random trials, scored over altitudes 15–85°.
- **est.**: the researcher's estimates. No published survey of Dobsonian pointing errors was found.

## 1. Alt-az pointing model

**From encoder angles to the true direction.** Small-angle forms, from the [TPoint manual v18.20](https://newapo.apo.nmsu.edu/static/pdf/tcc/tpoint.pdf) §7.2:

```
A_true = A_enc − IA − CA·sec E − NPAE·tan E − AN·sin A·tan E − AW·cos A·tan E   (+PAA scale)
E_true = E_enc + IE − AN·cos A + AW·sin A − TF·cos E                            (+PEE scale)
```

- **Signs:** the terms are subtracted from the true direction to give mount demands ([Mangum et al. 2006, Table 3](https://arxiv.org/abs/astro-ph/0609329)).
- **Large errors:** above a few arcminutes, TPoint switches to rigorous vector fitting (§2.2, §5.10).

| Term | Meaning | Typical size on a Dob | Minimum well-separated observations |
|---|---|---|---|
| IA | Azimuth zero | Arbitrary each night (incremental encoders) | 1 |
| IE | Altitude zero | Arbitrary unless the tube is level at power-on | 1 (with IA) |
| AN, AW | Base tilt τ: ΔE = τ·cos(A−A_τ), cross-elevation τ·sin(A−A_τ)·sin E | The ground slope; 0.5–1° unlevelled (est.) | 2, at azimuths 45–135° apart |
| CA | Optical axis not perpendicular to the altitude axis | Arcminutes to tens of arcminutes (est.) | 1 more, at altitudes ≥ 30° apart |
| NPAE | Altitude axis not perpendicular to the azimuth axis | Arcminutes (est.) | Points above 60–70°; often poorly determined |
| TF | Flexure, −TF·cos E | Arcminutes on a truss tube (est.) | Low (< 25°) plus high points |
| Scale | ΔA ∝ A | About 0 when counts per revolution come from the controller (est.) | A full azimuth circle |

**How many observations**
- A centred star gives 2 equations; a plate solve with roll gives 3.
- TPoint wants even, all-sky coverage (§4.1). Amateur models use about 20 stars ([AWR](http://www.awrtech.co.uk/telerrr3.pdf)).
- An alt-az mount gets "a respectable model" from one object sampled from rise to set (§4.1).
- The best amateur scopes "can easily beat 1 arcminute" (§1).

**What one plate solve plus an encoder reading fixes**
- It gives the full attitude. Roll converts to the horizon frame via the parallactic angle: tan q = sin H / (tan φ cos δ − sin δ cos H) (Meeus ch. 14).
- With the camera-to-tube rotation calibrated, one solve fixes:
  - IA;
  - the tilt across the line of sight;
  - only the sum of IE and the tilt along it.
- A second solve about 90° away in azimuth separates them, giving IA, IE, AN and AW exactly. That is the same information SynScan's 2-star alignment gets.
- Roll precision is about the centroid error / (field radius·√N), roughly 10–30″ for a 1–2° field (est.).

**Taki's matrix method** (Sky & Telescope, Feb 1989, pp. 194–196; "Matrix Method for Coordinates Transformation")
- Method: build sky vectors Lᵢ and encoder vectors lᵢ for two stars. The third vector of each set is the normalised cross product. Then T = [L₁L₂L₃][l₁l₂l₃]⁻¹.
- [INDI's alignment subsystem](https://github.com/indilib/indi/blob/master/libs/alignment/alignment_white_paper.md) uses it and calls it "quick and dirty".
- It is a pure rotation: it absorbs tilt and IA, but not IE.
  - [sim, 1° tilt] An IE of 0.25° leaves 32′ worst case and 18′ RMS.
- Star separations of 40–140° are nearly as good as 90° ([Bartels](https://bbastrodesigns.com/Altaz2StarAlignPointingErrors.html)).

**Residual error from a tilted base** [sim; medians of worst case / sky RMS]

| Base tilt | One sync, offsets only | One solve using roll | Two solves ≥ 90° apart |
|---|---|---|---|
| 0.5° | 1.0° / 0.64° | 0.70° / 0.46° | ≈ 0 |
| 1.0° | 2.0° / 1.3° | 1.4° / 0.9° | ≈ 0 |

- The worst case is twice the tilt, 180° in azimuth from the sync.
- Open-loop tracking after an offsets-only sync drifts about 16′ per hour per degree of tilt. [sim] Median 8′/h at 0.5° and 15′/h at 1°.

## 2. How plate-solving tools skip star alignment

**Cedar** ([about](https://github.com/smroid/cedar-server/blob/main/about.md), [release notes](https://github.com/smroid/cedar-server/blob/main/release_notes.md))
- Solves continuously at 10–30 Hz.
- Setup: centre the brightest central star in the eyepiece and tap it, which captures the telescope's boresight position in the image. A daylight mode uses a distant landmark.
- Push-to arrows are given in mount axes: up/down, and clockwise/counter-clockwise.

**PiFinder** ([menu](https://pifinder.readthedocs.io/en/release/menu_map.html), [guide](https://pifinder.readthedocs.io/en/release/user_guide.html))
- A 10° camera that solves up to 20 times a second, with an accelerometer between solves.
- Align: "Center a known star and confirm".
- Needs a location and time.

**Celestron StarSense** ([manual](https://www.kikkertland.dk/upload_dir/shop/StarSense_AutoAlign_Manual.pdf), pp. 11–12)
- "Calibrate Center" is done once: centre a bright star and press ALIGN. It stores the star's offset in the camera image.

**N.I.N.A.** ([CenteringSolver](https://github.com/isbeorn/nina/blob/master/NINA.Platesolving/CenteringSolver.cs), [defaults](https://github.com/isbeorn/nina/blob/master/NINA.Profile/PlateSolveSettings.cs))
- Loops solve → sync or offset → re-slew until within 1.0′, with at most 10 slews.
- Center After Drift recentres past 10′.

**KStars/Ekos** ([manual](https://kstars-docs.kde.org/en/user_manual/ekos-align.html))
- Captures, solves and syncs until within 30″, with up to 10 iterations.

**SharpCap** ([forum](https://forums.sharpcap.co.uk/viewtopic.php?p=47307))
- One pass: solve → sync → re-goto.

**ASIAIR** ([manual](https://astronomy-imaging-camera.com/manuals/ASIAIR_User_Manual.pdf))
- Auto-centres after a GoTo; the tolerance isn't published.

**SynScan** ([app manual](https://inter-static.skywatcher.com/downloads/synscan_app_manual_en_20230525.pdf), [alignment tips](https://inter-static.skywatcher.com/downloads/tips_for_improving_alignment_accuracy.pdf))
- Alignment stars should be at 15–60° altitude, 10–30° apart in altitude and 45–135° apart in azimuth.
- 3-Star "can compensate for cone error".
- Sync refines a patch of about 5°×5°.

**Finder-to-eyepiece offset.** Every tool calibrates it the same way: centre a star in the eyepiece and store the finder pixel it falls on. TPoint calls this an off-axis pointing origin (POX/POY, §5.12). Daylight landmarks need to be several km away: a 0.3 m finder-to-tube baseline at 1 km is 1′ of parallax.

## 3. Tracking

**Rates**, from sin h = sin φ sin δ + cos φ cos δ cos H (Meeus ch. 13; [SLALIB sla_ALTAZ](https://trac.fact-project.org/browser/trunk/MagicSoft/slalib/altaz.c?rev=10439)):

```
dh/dt = ω cos φ sin A = −ω cos φ cos δ sin H / cos h
dA/dt = ω (sin φ − cos φ cos A tan h) = ω (sin φ − sin δ sin h) / cos²h
dq/dt (field rotation) = −ω cos φ cos A / cos h   (A north through east; Meeus measures A from the south)
```

**Worked values**, checked by hand on 2026-10-21, at φ 40°, δ 20°:

| Hour angle | dh/dt | dA/dt | Field rotation |
|---|---|---|---|
| H 0 (transit) | 0 | 41.33″/s | 33.69°/h |
| H −30° | +10.07″/s | 18.45″/s | — |

**Zenith keyhole** ([Borkowski 1987](https://wwwold.astro.umk.pl/~kb/Papers/ActaA/NearZenith.htm))
- At transit, dA/dt = ω cos δ / sin(φ−δ).
- At latitude 40°, that is 0.19°/s for an object passing 1° from the zenith, 0.37°/s at 0.5° and 1.8°/s at 0.1°.
- Common handling (est.): an altitude ceiling of about 85–88°, or let the target pass, swing 180° and re-solve.

**Longest exposure under field rotation:** t_max = s / (r·|dq/dt|), where s is the allowed smear and r the distance from the rotation centre, both in pixels, so focal length cancels.
- Example: a 23.5 × 15.6 mm sensor with 3.76 µm pixels has a corner radius of 3,751 px. At transit as above, it allows 1.63 s for 1 px of smear.
- No limit due east or west.

**Closed loop ("plate-solve guiding")** ([Wallace 2002, SPIE 4848](https://sites.astro.caltech.edu/~srk/TP/Literature/Wallace_SPIE.pdf); [PHD2](https://openphdguiding.org/man/Guide_algorithms.htm))
- Feed-forward positions and rates from the model.
- Apply each solve as a small change to the model or origin, not to the target.
- Error: target minus aim point on the sky, rotated by q into cross-elevation and elevation; divide the azimuth part by cos E.
- Controller: proportional gain 0.5–0.8 with a deadband, plus a slow integral or an online refit of IA/IE/AN/AW. An unmodelled tilt costs about 0.26″/s per degree.
- A solve every 1–5 s is ample (est.). Finder-to-tube flexure limits imaging-grade guiding.

**EAA tools**
- SharpCap rotates frames to match the stack, and advises short subs ([forum](https://forums.sharpcap.co.uk/viewtopic.php?p=24480)).
- Vaonis compensates rotation in stacking.
- Seestar added an EQ (wedge) mode.
- Hardware derotators are the other route.

## 4. Corrections (J2000 input, late 2026)

Chapter numbers are Meeus 2nd edition.

| Effect | Size | Needed for 0.3–1° fields? | Meeus |
|---|---|---|---|
| Precession | 22.5′ maximum since 2000 | Yes | ch. 21 |
| Nutation | Δψ ≈ −17.2″ sin Ω | No, but cheap | ch. 22 |
| Annual aberration | 20.5″ | No | ch. 23 |
| Refraction | See below | Yes below about 20°; apply before fitting | ch. 16 |
| Diurnal aberration | ≤ 0.32″ | No | — |
| Proper motion | Barnard's Star 4.6′ since 2000; most stars a few ″ | Rarely | ch. 21 |

**Refraction formulas**
- Bennett: R = cot(h₀ + 7.31/(h₀+4.4)) arcmin, for apparent h₀.
- Saemundsson: R = 1.02·cot(h + 10.3/(h+5.11)) arcmin, for true h.
- Scale either by (P/1010)·(283/(273+T)).

| Altitude | Bennett | Saemundsson |
|---|---|---|
| 0° | 34.5′ | 29.0′ |
| 10° | 5.4′ | 5.4′ |
| 30° | 1.7′ | 1.7′ |
| 45° | 1.0′ | 1.0′ |

**Clock and site errors**
- 1 s of clock is 15″ of hour angle.
- 1 km north–south is 0.54′; east–west, 0.54′/cos φ.
- Both are fixed rotations that a full pointing model absorbs. They matter only for an absolute level/north start or for Solar System targets.

## 5. Answers to the owner's questions

1. **Does a 3-object alignment gain much?** Not named-star alignment, but a pointing model does.
   - One sync on a base 0.5–1° off level leaves errors of 1–2° elsewhere and 8–15′/h of drift.
   - Two automatic solves at least 90° apart in azimuth fix both zeros and the tilt.
   - The fixed hardware terms come from about 20 solves, kept between nights.
2. **Can alignment come from any stars?** Yes, as in StarSense, PiFinder and Cedar.
   - It needs a one-time aim-point calibration, a clock good to a few seconds, a site good to about 1 km, and some azimuth spread among the solves.
3. **Is there tracking without a full alignment?** Yes: feed-forward rates plus finder-solve correction every 1–5 s hold a visual target indefinitely, except within about 0.5–1° of the zenith.
   - For imaging at 1800 mm, keep subs to a few seconds and derotate when stacking.
