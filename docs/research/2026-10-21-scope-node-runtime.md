# Runtime: WSL2 on an ARM64 Surface, the scope node on a Raspberry Pi, and the browser

Research notes gathered on 2026-10-21 for feat-13 (live finder) and feat-14 (GoTo). They cover where each piece can run and what limits it. Sources are linked inline, with the dates found.

**Outcome:** per [ADR-0001](../adr/0001-console-owns-pointing-scope-node-drives-axes.md), the finder camera (an Arducam on the Pi's CSI ribbon) and the mount's USB cable both live on a Raspberry Pi 3B, the scope node. The console runs in WSL2 on the Surface. That sidesteps most of the USB-into-WSL problems below, which are kept for the record.

## 1. USB into WSL2 on ARM64

**usbipd-win**
- Supports ARM64 hosts from 5.0.0 (2025-03-22) ([releases](https://github.com/dorssel/usbipd-win/releases); [Microsoft guide](https://learn.microsoft.com/en-us/windows/wsl/connect-usb), 2026-06-02).
- An open issue reports that 5.3.0 fails where 5.2.0 works ([#1293](https://github.com/dorssel/usbipd-win/issues/1293)).

**WSL kernel drivers**
- WSL ships the 6.18 kernel series ([kernel releases](https://github.com/microsoft/WSL2-Linux-Kernel/releases)).
- The stock arm64 config builds these drivers as modules: cp210x, ftdi_sio, ch341, cdc-acm, uvcvideo/V4L2 and vhci-hcd.
- **It does not build pl2303** ([6.18 arm64 config](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.18.y/arch/arm64/configs/config-wsl-arm64)); the x86 config does.
  - This matters because Sky-Watcher's own USB ports and many GPS pucks use PL2303.
  - Workarounds: build `pl2303.ko` for the exact kernel, or a custom kernel via `.wslconfig` ([wsl-config](https://learn.microsoft.com/en-us/windows/wsl/wsl-config)).
- Kernel 6.18.40.1 trimmed the configs, so re-check drivers after every kernel update.

**usbip performance**
- It is limited by design: every USB request makes a round trip through user mode ([#1006](https://github.com/dorssel/usbipd-win/issues/1006)).
- Webcams fail at 720p and above on USB2, and USB3 industrial cameras time out ([#699](https://github.com/dorssel/usbipd-win/issues/699)).

## 2. Cameras

- **ZWO:**
  - The SDK has Linux armv8 libraries ([ZWO SDK](https://www.zwoastro.com/software/product-sdk/)).
  - There is no signed Windows-on-ARM driver as of mid-2026 ([forum](https://bbs.zwoastro.com/d/13554-asi-drivers-for-windows-on-arm-ie-mac-m1-silicon)).
  - ZWO cameras are not UVC.
- **Raspberry Pi CSI cameras** (the owner's Arducam) use libcamera / rpicam-apps ([Raspberry Pi docs](https://www.raspberrypi.com/documentation/accessories/camera.html)).
  - Some Arducam sensors need Arducam's own drivers, and some high-resolution ones don't support older Pis. Check the exact model on the Pi 3B at bring-up.
- **Rust crates** (checked 2026-10-21):
  - `nokhwa` 0.10.11: V4L2 and Media Foundation.
  - `v4l` 0.14.0: stale, 2023.
  - `libcamera` 0.7.0: needs libcamera headers to build.
  - `asi_camera2`: MIT, ZWO.
  - [`cedar-camera`](https://github.com/smroid/cedar-camera): FSL-1.1, whose non-compete terms don't suit this project.
- The plan uses an rpicam-apps subprocess rather than libcamera bindings, because CI builds `--all-features` without libcamera headers.

## 3. The Raspberry Pi 3B as the scope node

These are estimates; measure them at feat-13's bring-up.

- **Hardware:** Cortex-A53 at 1.2 GHz, 1 GB RAM, 2.4 GHz-only Wi-Fi, no real-time clock. A 64-bit OS is supported.
- **Detection speed:**
  - cedar-detect, which `star-detection` matches exactly, targets Pi boards. Its code works around an IMX296 quirk on the Pi Zero 2 W, which has the same cores.
  - Scaling the README benchmark (2.4 ms a frame on the Surface) by the cores' speed gap gives roughly 40 ms a frame, about 20 frames a second on one core.
  - A finder needs 1–5 frames a second.
- **No solving on the Pi**, so it never loads the pattern database.
- **2.4 GHz Wi-Fi** carries star lists (about 1 KB a frame) easily. Send preview JPEGs only now and then.
- **The Pi's kernels include PL2303, FTDI, CP210x and CH341** ([Pi kernel config](https://github.com/raspberrypi/linux/blob/rpi-6.12.y/arch/arm64/configs/bcm2711_defconfig)), so the mount's USB cable works whatever its chip.
- **Builds:** a binary built on WSL Ubuntu expects a newer glibc than Pi OS has, so build a static `aarch64-unknown-linux-musl` binary.

## 4. Running natively on Windows ARM64

Not planned; the owner's rule is WSL unless something must run on Windows. For the record:
- `aarch64-pc-windows-msvc` became Tier 1 in Rust 1.91 ([blog](https://blog.rust-lang.org/2025/10/30/Rust-1.91.0/)).
- The `serialport` crate doesn't list it in CI.
- Prolific's ARM64 driver covers only the newer PL2303G chips ([Prolific](https://www.prolific.com.tw/en/portfolio-item/pl2303gs/)).
- x64 serial drivers don't run under emulation ([Rob Miles](https://www.robmiles.com/journal/2024/11/11/using-the-ch340-usb-serial-connection-with-arm-64-not)).

## 5. Networking

- **NAT mode (WSL's default):** a phone on the LAN can't reach WSL without `netsh interface portproxy` (TCP only) and a firewall rule ([WSL networking](https://learn.microsoft.com/en-us/windows/wsl/networking)).
- **Outbound connections from WSL work under NAT**, including the console connecting out to the scope node. Broadcast and multicast discovery do not.
- **Mirrored mode** shares Windows' interfaces, so LAN devices reach WSL directly and multicast works.
  - It needs a Hyper-V firewall rule for the console's port ([Hyper-V firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/hyper-v-firewall)), covering whichever network profile the Wi-Fi uses.
  - The finder README already documents mirrored mode for phone access.
- **At the telescope:** the house Wi-Fi reaches it, with internet over Starlink, so the Pi, Surface and phone share one network and the Pi keeps time over NTP.

## 6. Browser secure contexts (Android Chrome)

- **Geolocation** has been refused on plain-HTTP origins since Chrome 50; localhost is exempt ([Chrome](https://developer.chrome.com/blog/geolocation-on-secure-contexts-only/)).
- **getUserMedia** has required a secure context since Chrome 74 ([MDN](https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts/features_restricted_to_secure_contexts)).
- **Workarounds:** the `unsafely-treat-insecure-origin-as-secure` flag, `adb reverse`, self-signed HTTPS, or a local CA (mkcert). The owner chose a typed site instead, so none is needed.

## 7. Location and time

- **The Surface has no GNSS receiver.** Windows location from Wi-Fi needs a network lookup, and WSL can't call it anyway ([Microsoft](https://support.microsoft.com/en-us/windows/windows-location-service-and-privacy-3a8eee0a-5b0b-dc07-eede-2a5ca1c49088)). The site is typed once.
- **USB GPS pucks**, if ever wanted: u-blox ones are CDC-ACM and work in WSL; the BU-353S4 is PL2303 and does not ([OpenWrt](https://openwrt.org/docs/guide-user/services/ntp/gps)).
- **WSL's clock** follows Windows through Hyper-V time sync ([Ubuntu](https://ubuntu.com/wsl/docs/stable/explanation/time-sync/)).
- **Drift without NTP:** a laptop clock drifts a few seconds a day at most. 1 s is 15″ of hour angle, which is negligible, and a fitted pointing model absorbs it.
