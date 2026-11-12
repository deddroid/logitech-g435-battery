# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-26

### Added
- *Comandi della cuffia* submenu at the top of the tray menu, listing the headset's button combinations from the quick start guide (mic mute, sidetone, volume limit, battery LED, LIGHTSPEED/Bluetooth, pairing).
- README section *Headset buttons*: what each button sends over USB, and how to fix volume keys being ignored on KDE Plasma.
- Prebuilt archives for Linux and Windows on the GitHub release page, with a `SHA256SUMS` file.
- `logitech-g435-battery-bin` AUR package, which installs the prebuilt Linux binary.

### Changed
- The Linux build now depends on `alsa-lib` at runtime.

### Fixed
- The headset's Volume + button now works on Linux: the app keeps the dongle's USB hardware volume one step below the maximum (174/175, an inaudible −0.4 dB), because the dongle ignores Volume + at 100%.

### Verified on hardware
- Logitech G435 with the `046d:0acb` dongle, on Linux: Volume + fix. The mic mute button and the button combinations send nothing to the PC (checked with `usbmon`), so the system mic icon cannot follow the headset's mute.

## [0.1.0] - 2026-10-20

### Added
- Tray icon showing the battery percentage of the Logitech G435 through its LIGHTSPEED dongle, colored by level, on Linux and Windows; the tooltip shows percentage and millivolts.
- Battery read every 15 minutes; on Linux scheduled reads wait until the headset is not playing audio, because each read drops audio for about a second on the `0acb` dongle.
- Detection of the headset being switched off.
- Start at login option (XDG autostart on Linux, registry on Windows).
- `g435-tray --once` prints the battery level and exits, for scripts and status bars.
- `logitech-g435-battery` AUR package, which builds from source.
- README in English and Italian. The user interface is in Italian.

### Verified on hardware
- Logitech G435 with the `046d:0acb` dongle, on Linux. Support for the `046d:0adf` dongle is ported from portyporty/g435-battery-percentage and untested; the Windows build compiles but has not been tried on real Windows.

[Unreleased]: https://github.com/deddroid/logitech-g435-battery/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/deddroid/logitech-g435-battery/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/deddroid/logitech-g435-battery/releases/tag/v0.1.0
