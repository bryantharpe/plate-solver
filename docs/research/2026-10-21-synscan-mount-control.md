# Driving a Sky-Watcher 400P SynScan GoTo from software

Research notes gathered on 2026-10-21 for feat-14 (GoTo). Each fact cites a source from the list at the end. Where a fact is unverified, or the sources disagree, the note says so. Decisions taken on the strength of these notes are recorded in [ADR-0001](../adr/0001-console-owns-pointing-scope-node-drives-axes.md).

**Decision on the path:** the scope node (a Raspberry Pi on the telescope) speaks the **motor-controller protocol** (§3) over a USB cable plugged straight into the mount's port. No hand controller is involved.

These notes recommended the mount's built-in Wi-Fi instead, because it needs no USB driver in WSL. Putting the cable on the Pi removes that problem: Raspberry Pi kernels include every common USB-serial driver [37].

## 1. Hardware

**Hand controller**
- Sky-Watcher USA lists a "SynScan hand controller" with built-in Wi-Fi [7]. A dealer listing names a SynScan V5 [8].
- So it is probably a V5, which has a USB port; the V4 does not [4][5]. Unconfirmed for this unit.
- V5 ports [4]:
  - an 8-pin RJ-45 "Mount" port, which goes to the mount's 6-pin RJ-12 hand-controller jack;
  - a 6-pin RJ-12 multi-purpose port (RS-232C plus camera shutter);
  - USB.
- PC link: 9600 bps, 8N1 [4].

**USB adapters and signals**
- **The V5's USB-serial chip** is undocumented. Forum reports say Prolific PL2303, some say CH340; unverified [39].
- **SynScan USB Adapter (S30107)** plugs straight into the mount's jack [38]. One retailer says it shows up as a USB HID device rather than a serial port; unverified.
- **Signal levels:** the mount's jack is TTL. RS-232 levels can damage it [9].
- **Single-wire bus:** on most alt-az mounts, TX and RX share one wire plus a DROP line [2]. A TTL cable hears its own bytes echoed back, and INDI uses that echo to detect these boards [17].

**Wi-Fi**
- Current units have it built in [7][8].
- The motor-firmware page lists "Skyliner Dobsonian GOTO (with Wi-Fi)" on the MC014 board (firmware 3.40), and Dobs made after May 2025 on MC029 (3.54) [11]. Which board a given unit has is unverified; `:e1` reports it.
- **Access-point mode:** network `SynScan_WiFi_####`, no password, 192.168.4.1 [9][10].
- **Station mode:** joins a router [9]. The router must be up before the mount is powered, or the adapter falls back to access-point mode. Enable only one mode [10].
- **External adapter:** an ESP-WROOM-02 on UDP port 11880, talking to the mount at 9600 baud (115200 on some mounts). It occupies the hand-controller jack [10].

**Freedom Find**
- Each axis has two encoders, one on the axis shaft and one on the motor (US Patent 7,228,253) [6].
- The tube can be pushed by hand without losing count while powered [6]. The axis encoder is coarser, so the manual says to re-calibrate the encoders from time to time [6].
- Turning the auxiliary encoder on lowers position resolution [9].
- In the motor protocol, `:W` with codes 000004/000005 switches it on/off, and `:q` with code 000001 reports support [2][16].

**Mechanics**
- **Resolution:** the motor encoder gives 1,620,000 counts per revolution (about 0.8″ per count); the axis encoder 11,748 (about 110″) [6][7]. Read the real values with `:a` [2].
- **Slew presets 0–9, from the Dob spec sheet:** 1×, 2×, 8×, 16×, 32×, 200×, 400×, 600×, 800×, 1000× sidereal [6][7].
- **The V5 manual's table differs:** 0.5×, 1×, 8×, 16×, 32×, 64×, 128×, 400×, 600×, then a mount-dependent maximum above 800× (3.4°/s) [4].
- **Top speed:** roughly 3.4–4.2°/s.
- **Drive and power:** DC servo motors, 10–15 V DC at 1 A [6][7].
- **Altitude:** no published range. The hand controller's user-set limits apply only after a star alignment [4][6].
- **Cable wrap:** power and the hand controller plug into the motor box on the rotating rocker [6]. No azimuth hard stop is documented, so any cable running to the ground can wrap.

## 2. Hand-controller protocol (v3.3) [1]

Not used; recorded because it is the obvious alternative.

**Basics:** 9600 bps, 8N1. Needs hand-controller firmware 3.38 / 4.38.06 or later. Allow up to 5 s for a reply.

**Commands**

| Function | Command |
|---|---|
| Get RA/Dec (16-bit / precise) | `E` / `e` |
| Get Az/Alt | `Z` / `z` |
| Goto RA/Dec | `R` / `r` |
| Goto Az/Alt | `B` / `b` |
| Sync (firmware 3.37 or later) | `S` / `s` |
| Cancel goto | `M` |
| Is goto in progress | `L` |
| Is alignment complete | `J` |
| Get / set tracking (0 off, 1 alt-az, 2 EQ, 3 PEC) | `t` / `T` |
| Location / time | `W` `w` / `H` `h` |
| Version / model (144–159 = Dob GoTo) / echo | `V` / `m` / `K` |

**Slewing through the passthrough command `P`**
- Variable rate: `P`,3,16 or 17 (Az or Alt),6 or 7 (+ or −),rate high byte,low byte,0,0, in units of ¼″/s.
- Fixed rate: `P`,2,16 or 17,36 or 37,rate 0–9,0,0,0.
- Axis-level goto and set-position via `P`,4,… with a 24-bit position.

**Before alignment**
- RA/Dec readings are meaningless, and Goto RA/Dec is refused.
- Az/Alt readings are relative to where the mount was powered on.
- Nothing is documented for `B`/`b`, Sync, tracking or `P` without alignment.
- INDI's SynScan driver refuses to connect until `J` reports aligned [19].

**Firmware differences**
- Firmware 4.39.21 changed `Z`/`z` to the telescope's actual Alt-Az [13]. Its release date differs between sources [41].
- Firmware 6.04.01 / 4.42.00 (Oct 2025) unified the hand-controller and SynScan App coordinate definitions [12].

## 3. Motor-controller protocol [2]

**Framing**
- `:` + command + axis (`1` Az, `2` Alt, `3` both where allowed) + hex data + CR.
- Replies are `=` + data + CR, or `!` + error + CR.
- 24-bit values go low byte first (0x123456 is sent as `563412`), and positions are offset by 0x800000.
- 9600 8N1 at TTL levels. Some boards with a built-in USB port run at 115200, so probe both with `:e1`.

**Queries**
- `:a` counts per revolution, `:b` timer frequency, `:g` high-speed ratio.
- `:e` board version (low byte = mount code; INDI calls 0x90 "DOB") [16].
- `:j` position, `:f` status.
- `:d` axis-encoder position (auxiliary encoder on some mounts; unknown on Dobs).

**Motion**
- `:F` initialise, `:G` motion mode, `:I` step period, `:J` start.
- `:K` decelerate to stop, `:L` instant stop.
- `:S` absolute target, `:H` relative target, `:M` brake point.
- `:E` set position (axis stopped).

**Step period:** timer frequency × 360 / (degrees per second) / counts per revolution, multiplied by the high-speed ratio in fast mode. Changing `:I` while slewing in fast mode is not supported.

**INDI's practice** [16][17]
- Mode codes: `0` fast goto, `1` slow track, `2` slow goto, `3` fast track.
- It switches to fast mode above 128× sidereal.
- A goto is sent as `:G`, `:H`, `:M`, `:J`.

**Tracking:** the board runs only constant rates, so the host computes alt-az tracking. SynScan App does exactly that [9]. Smooth variable-rate tracking reportedly needs motor firmware 3.39 or later [28].

**Ways in**
- Wi-Fi UDP 11880, one command per packet [2].
- The hand controller's PC Direct mode, about 32 ms per command [24].
- "SynScan Relay" firmware [12].
- A USB adapter or TTL cable on the mount's jack [9]. This is our path.

**On Dobs:** INDI's `indi_skywatcherAltAzMount` accepts alt-az/Dob mount codes (0x80 and up) [17]. Users have driven 10″ and 250P GoTo Dobs with it: goto and tracking worked, but sync was unreliable [22].

## 4. Prior art and licences

**INDI**
- `indi_skywatcherAltAzMount` [18]:
  - speaks the motor protocol over serial or UDP;
  - aligns from sync points (no star alignment);
  - tracks with predicted rates plus PID;
  - has a switch for the auxiliary encoder.
  INDI core is LGPL-2.1 or later [20].
- `indi_synscan_telescope`: the hand-controller protocol, LGPL-2 [19].
- `indi_eqmod`: GPL-3 or later, aimed at EQ mounts [21].

**Windows / ASCOM**
- EQMOD: EQ mounts only, LGPLv2 [23][25].
- Green Swamp Server: GPL-3.0, with an alt-az mode since 2024 [26].
- Sky-Watcher's ASCOM driver for SynScan App: free, no published source. It talks to SynScan App on UDP/TCP 11881, where `AxisCoordinatesGet` returns raw axis angles [3][15].

**SynScan App / SynScan Pro 2.6.11** [14][9]
- Without alignment: rough GOTO from the level/north home position, and Point-and-Track.
- Plate-solve `SyncTo` adds an alignment sample (since 2.4.5).
- "SynMatrix AutoAlign" aligns from a camera (since 2.5.2).

**Libraries**
- Python:
  - `pysynscan`: motor protocol over UDP, GPL-3.0 [27];
  - NexPlane: GPL-3.0, whose author calls the network protocol unreliable [28].
- Rust:
  - `jsorrell/rust-synscan`: GPL-3.0, last commit 2022 [29];
  - `MattBlack85/skywatcher-rs`: GPL-3.0 [30];
  - `nexstar_rust`: MIT/Apache, for the NexStar protocol that SynScan hand controllers emulate [32].
- No maintained crates.io crate exists for either Sky-Watcher protocol.
- **Permissive reference:** Sky-Watcher's own C# sample code is MIT [31]. Write our adapter from the published command set [2] and this sample; read the GPL/LGPL drivers for behaviour only.

## 5. Practical cautions

- **Lossy links:** use short timeouts with retries (INDI: 300 ms × 5 [16]; NexPlane: 100 ms [28]), commands that are safe to repeat, and state read back with `:f`/`:j`.
- **One controller at a time:** never drive the mount from SynScan App or the hand controller while the console is in control [9].
- **On connect:** check firmware ≥ 3.39 with `:e`, and dual-encoder support with `:q`.
- **Counts:** handle wrap-around of the 24-bit encoder counts.
- **Freedom Find:** leave it on (survives hand pushes, about 110″ resolution), or off (0.8″) and re-solve after any push. The plan turns it off while the console drives.
- **Zenith keyhole:** near the zenith, the azimuth rate needed for tracking exceeds what the mount can do.
- **First test:** send `:e1\r`, `:a1\r`, `:b1\r`, `:g1\r`, `:q1010000\r`, `:j1\r`.

## Sources

[1] https://inter-static.skywatcher.com/downloads/synscanserialcommunicationprotocol_version33.pdf
[2] https://inter-static.skywatcher.com/downloads/skywatcher_motor_controller_command_set.pdf
[3] https://inter-static.skywatcher.com/downloads/synscan_app_protocol_20250930.pdf
[4] https://inter-static.skywatcher.com/downloads/synscan_v5_hand_control_manual_sshcv5-f-170808v2-en.pdf
[5] https://inter-static.skywatcher.com/downloads/Synscan_V4_Hand_Control_Manual_SSHCV4-F-161208V1-EN.pdf
[6] https://cdn.shopify.com/s/files/1/0080/7095/5123/files/CollapsibleGoToDobsonian_manual.pdf
[7] https://www.skywatcherusa.com/collections/flextube-synscan-dobsonians/products/sky-watcher-flextube-400p-synscan
[8] https://davidastro.com/en-us/products/sky-watcher-flextube-400p-synscan
[9] https://inter-static.skywatcher.com/downloads/synscan_app_manual_en_20230525.pdf
[10] https://www.astroshop.be/Produktdownloads/63167_1_Anleitung-EN.pdf
[11] http://skywatcher.com/download/software/motor-control-firmware/
[12] http://skywatcher.com/download/software/synscan-v4-hand-controller-firmware/
[13] https://skywatcher.com/download/software/synscan-hand-control-firmware-earlier-versions/
[14] https://skywatcher.com/download/software/synscan-app/about
[15] http://www.skywatcher.com/download/software/ascom-driver/
[16] https://github.com/indilib/indi/blob/master/drivers/telescope/skywatcherAPI.h
[17] https://github.com/indilib/indi/blob/master/drivers/telescope/skywatcherAPI.cpp
[18] https://github.com/indilib/indi/blob/master/drivers/telescope/skywatcherAPIMount.cpp
[19] https://github.com/indilib/indi/blob/master/drivers/telescope/synscandriver.cpp
[20] https://github.com/indilib/indi/blob/master/COPYRIGHT
[21] https://github.com/indilib/indi-3rdparty/blob/master/indi-eqmod/eqmodbase.cpp
[22] https://indilib.org/forum/general/6762-indiserver-wifi-option-for-synscan-goto/55736.html
[23] https://eq-mod.sourceforge.net/faqs/mountsupport.html
[24] https://eq-mod.sourceforge.net/prerequisites.html
[25] https://sourceforge.net/projects/eq-mod/
[26] https://github.com/rmorgan001/GSServer
[27] https://github.com/nachoplus/pysynscan
[28] https://github.com/nforrester/NexPlane
[29] https://github.com/jsorrell/rust-synscan
[30] https://github.com/MattBlack85/skywatcher-rs
[31] https://github.com/skywatcher-pacific/skywatcher_open and https://code.google.com/archive/p/skywatcher/ (MIT)
[32] https://crates.io/crates/nexstar_rust
[37] https://github.com/raspberrypi/linux/blob/rpi-6.12.y/arch/arm64/configs/bcm2711_defconfig
[38] https://astronomics.com/products/sky-watcher-synscan-usb-adapter
[39] https://stargazerslounge.com/topic/371298-synscan-v5-hand-controller-windows-10-driver-issue/ and https://stargazerslounge.com/topic/338265-synscan-v5-usb-port-a-mystery/ (seen only through search snippets)
[41] https://www.skywatcherusa.com/pages/firmware-and-software
