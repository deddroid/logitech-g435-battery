# g435-tray

🇬🇧 English · 🇮🇹 [Italiano](README.it.md)

A tray icon showing the battery level of the **Logitech G435** headset through its LIGHTSPEED dongle.
Written in Rust, it runs on **Linux** (natively on KDE, on GNOME with the AppIndicator extension) and on **Windows**.

- The icon shows the percentage and changes color with the level: green, orange (≤35%) or red (≤15%).
- The tooltip shows `G435: 94% (4140 mV)`.
- Menu: refresh now, start at login, quit.
- Reads the battery every 15 minutes and retries every minute after an error.
- On Linux, scheduled reads wait until the headset is not playing audio, for at most 30 minutes, and the icon keeps showing the last value in the meantime. The first read at startup and a manual refresh happen right away. A game or browser with an open audio stream counts as "playing" even when silent.
- No G HUB, no Logitech DLLs, no network traffic.

![KDE tray icon with the tooltip "G435: 92% (4122 mV)"](docs/screenshot.png)

> Unofficial project, not affiliated with Logitech. The user interface is currently in Italian.

## Supported dongles

| PID | Revision | Transport | Status |
|-----|----------|-----------|--------|
| `046d:0acb` | HDT647 (most common) | feature report 6 through the Realtek bridge ("bootloader") | **tested** |
| `046d:0adf` | newer revision | 64-byte reports with CRC-16 | ported from [portyporty][porty], untested |

To check which one you have: `lsusb | grep 046d` on Linux, or Device Manager on Windows.

## Building

```bash
cargo build --release
```

The binary ends up in `target/release/g435-tray` (`g435-tray.exe` on Windows).

- **Linux**: needs `libudev` (package `systemd-libs` / `libudev-dev`) and `pkg-config`.
- **Windows**: the Rust MSVC toolchain is enough. hidapi uses its native Rust backend, so no C compiler is needed.

To install it for the current user (into `~/.cargo/bin`):

```bash
cargo install --path .
```

## Permissions (Linux)

The program needs access to the dongle's `/dev/hidrawN`. If `g435-tray --once` fails with a permission error:

```bash
sudo cp packaging/linux/70-logitech-g435.rules /etc/udev/rules.d/
```

```bash
sudo udevadm control --reload && sudo udevadm trigger
```

## Usage

```text
g435-tray                 start the tray icon
g435-tray --once          print "94% (4140 mV)" and exit (handy for scripts/waybar/polybar)
g435-tray --interval 30   read every 30 minutes
G435_DEBUG=1 g435-tray --once   dump the HID traffic
```

## Notes and limitations

- **The percentage is an estimate** based on the Li-Po battery voltage (4.2 V = 100%, 3.4 V = 0%). While charging, the voltage rises and the estimate reads higher than the real charge.
- On the `0acb` dongle every read briefly puts the USB bridge in "bootloader" mode, and **audio drops for about one second**. That's why the program reads only every 15 minutes and, on Linux, waits for the audio to stop. It checks `/proc/asound/cardN/pcm*/sub*/status` of the G435's USB sound card; PipeWire closes the device a few seconds after playback ends. This check is not implemented on Windows yet.
- The battery command only reaches the headset when the dongle is in *remote mode* 1. If it isn't, the program sets it (the same value the official pairing tool uses).
- If the dongle stops answering (it acknowledges commands but returns no data), unplug it and plug it back in.

## How it works (protocol)

Both dongles talk to the same audio SoC (Airoha/Audiowise) using `"PA"` commands:

```text
50 41 <category> <seq> <len> <command...>     request
50 49 ...                                     reply
```

| Command | Bytes | Reply |
|---|---|---|
| REMOTE_GET_CONNECT_STATUS | `0e .. 01 e3` | `02 00 e3 <0/1>` |
| REMOTE_GET_MODE / SET_MODE | `0e .. 01 e0` / `0e .. 02 e1 01` | `02 00 e0 <mode>` |
| GET_BATTERY | `06 .. 01 00 31` | `07 00 06 <seq\|80> 00 <mV lo> <mV hi> ..` |

On the `0acb` the transport is: enter the bootloader with feature report `06 01 03`, send the command with `06 04 03 <len> <cmd>` (ack `06 5a` on the input endpoint), read the reply with GET_FEATURE on report 6, and leave with `06 08 03`.

## Other features found (not implemented yet)

Besides the battery, the available research documents these commands:

- **Headset connection status** (on/off): works, and the program already uses it to report "headset off".
- **RSSI / radio link quality** (`GET_RSSI`, `LQ_GET_QUALTY`): candidates for a signal indicator. Unverified: during testing the dongle got stuck right after a `GET_RSSI` and an `OTA_CMD_ECHO`.
- **Firmware version** (`OTA_CMD_GET_IMAGE_VERSION`).
- **LEDs** (`CMD_SET_LED`, `CMD_SET_LED_PWM`) and **EQ/DSP** (`EQ_ON_OFF`, `DSP_CMD_IIR_EQ`, ...): present in firmware shared by several products, but they may have no effect on the G435.
- **Dongle ↔ headset pairing** (scan and connect by MAC address): what the official re-pairing tool does.
- **Dangerous** commands to stay away from: OTA/flash/NVDS (`ERASE_LOAD_REGION`, `OTA_CMD_UPDATE`, `AW_HA_CLEAR_NVDS`, ...).

Nobody has found commands for sidetone, reading the hardware mute, or detecting charging yet.
The volume and mute shown by other projects (e.g. G435-Companion) belong to the operating system, not the headset.

## Credits

- **[deddroid](https://github.com/deddroid)**: idea, testing on real hardware and design decisions.
- **Claude (Anthropic)**: wrote the Rust code, analyzed the existing projects and verified the protocol on the `0acb` dongle, all in a [Claude Code](https://claude.com/claude-code) session.
- **endes** for reverse engineering the `0acb` dongle, **portyporty** (Oğuz Kayra Bozkurt) for the `0adf` dongle protocol: see the sources below.

## License

[MIT](LICENSE): anyone can use, modify and redistribute the code, including commercially, as long as the license file is kept. Parts derived from other projects are listed in [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES).

## Sources

- [endes — "USB HID down the rabbit hole: Logitech G435 dongle"](https://dev.to/endes/usb-hid-down-the-rabbit-hole-logitech-g435-dongle-33if) and the [command list gist](https://gist.github.com/endes0/21193c55e2e38c9b3e3a95492380059c)
- [portyporty/g435-battery-percentage][porty]: Windows tray app, `0adf` dongle, voltage→percentage curve
- [GotardoN1/G435-Companion](https://github.com/GotardoN1/G435-Companion): Bluetooth battery through Windows PnP properties
- [CSaratakij/G435BatteryApproximate](https://github.com/CSaratakij/G435BatteryApproximate): estimate based on usage time, no real reading

[porty]: https://github.com/portyporty/g435-battery-percentage
