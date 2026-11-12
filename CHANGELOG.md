# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-11-12

### Changed
- On the `046d:0acb` dongle bridge messages are now sent the way Logitech's update tool sends them (514-byte output report, 16-bit command length). A battery read keeps the dongle in bootloader mode for about 0.2 s instead of 1.0 s, so audio and microphone stop for about 0.6 s instead of 1.5 s.

### Verified on hardware
- Logitech G435 with the `046d:0acb` dongle, on Linux: new message format, 7 reads with audio and microphone streaming. Not tested on Windows, nor with the headset switched off.

## [0.2.1] - 2026-11-12

### Changed
- On the `046d:0acb` dongle a battery read now sends two commands instead of three, so audio and microphone stop for about 1.5 s instead of about 1.9 s. The remote mode is set only when the battery reply is not a voltage.

### Added
- `docs/PROTOCOL.md`: USB layout, bridge messages, measured cause and length of the audio interruption, and a known way to hang the dongle.

### Verified on hardware
- Logitech G435 with the `046d:0acb` dongle, on Linux: new read sequence with audio and microphone streaming, including recovery from remote mode 0; tray left running for a few minutes with `--interval 1`, dongle still answering afterwards. Not tested with the headset switched off, nor on Windows.

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

[Unreleased]: https://github.com/deddroid/logitech-g435-battery/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/deddroid/logitech-g435-battery/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/deddroid/logitech-g435-battery/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/deddroid/logitech-g435-battery/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/deddroid/logitech-g435-battery/releases/tag/v0.1.0
