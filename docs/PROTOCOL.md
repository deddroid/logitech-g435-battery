# G435 LIGHTSPEED dongle `046d:0acb`: protocol notes

What is known about reading the battery through the `0acb` dongle, and why a
read interrupts audio. Everything marked *measured* was observed on one dongle
(`bcdDevice 0.08`) on Linux 7.3 with PipeWire, on 2026-11-12. The command
layer comes from [endes' research][endes].

## USB layout

| Interface | Class | Endpoint | Use |
|---|---|---|---|
| 0 | Audio Control | `0x87` interrupt IN | volume, mute, Realtek extension unit (`0x0bda`) |
| 1 | Audio Streaming | `0x81` isochronous IN | microphone, mono 48 kHz |
| 2 | Audio Streaming | `0x04` isochronous OUT | headphones, stereo 48 kHz |
| 3 | HID | `0x88` interrupt IN | volume keys and the vendor channel |

The battery read uses interface 3 only (`hidraw` on Linux). The audio
interfaces stay bound to the system audio driver and their streams stay open.

HID reports of interface 3:

| Report | Usage page | Direction and size | Use |
|---|---|---|---|
| 3 | Consumer `0x0C` | input, 2 × 16 bit | volume keys |
| 5 | Telephony `0x0B` | input, 2 × 16 bit | never seen |
| 6 | vendor `0xFFC0` | input 7, output 514, feature 63 bytes | bridge messages (below) |
| `0x34` | vendor `0xFFC3` | input 20, output 2 bytes | unknown, never used here |

## Bridge messages (report 6)

The USB chip is a Realtek ALC4021; the radio SoC behind it is an
Audiowise/Airoha PAU1823. Report 6 talks to the Realtek side. Messages go out
as the 514-byte **output** report (SET_REPORT Output), zero padded:

| Output report | Meaning | Reply on the input endpoint |
|---|---|---|
| `06 01 03` | enter "bootloader" | `06 5a` (ok) or `06 fe` (already in) |
| `06 04 03 <len lo> <len hi> <cmd>` | pass `<cmd>` to the SoC | `06 5a` |
| `06 08 03` | leave "bootloader" | `06 5a` |

This is the format of Logitech's own tool (*from G435Update 3.1.1.14,
AWToolLIB2: `rtk_bootloader`, `rtk_write`, `rtk_read_RX`*), *measured* here
on 7 sessions. That tool enters the bootloader when it connects and leaves
only when it closes: it has no path to the SoC that avoids the bootloader, and
it never uses report `0x34`.

The same messages are also accepted as 64-byte **feature** reports with a
one-byte length (`06 04 03 <len> <cmd>`), the format published by endes and
used up to 0.2.1. It works, but each command then blocks for about 425 ms
(14 ms as an output report), and GET_BATTERY sent first could hang the dongle
(see below).

SoC replies are read with GET_FEATURE on report 6. An empty buffer reads
`06 ff ff …`. The last byte of a 64-byte feature report is the number of valid
bytes. The first reply after entering the bootloader arrives without its first
two bytes (`50 49`).

Outside the bootloader GET_FEATURE on report 6 stalls (*measured*), and the
dongle sends no unsolicited input reports apart from the volume keys
(*measured*, 15 s with audio and microphone streaming).

## Why a read interrupts audio

*Measured* by recording the headset microphone while talking to the bridge:

| Session | Time in bootloader | Microphone samples lost |
|---|---|---|
| no bridge message | 0 | none |
| enter, wait 2 s, leave (no command) | 2.00 s | about 2.0 s |
| 0.2.0 sequence, three commands | 1.40 s | about 1.4 s |
| 0.2.1 sequence, two commands | 0.98 s | about 1.0 s |
| current: two commands as output reports | 0.17 s | about 0.2 s |

In every session about 0.45 s of digital silence follows the lost samples.

* `06 01 03` alone stops the audio streams; they resume after `06 08 03`. It
  is the dongle firmware, not the host: no audio interface is claimed or
  detached.
* As feature reports each `06 04 03` blocks for about 425 ms; as output
  reports about 14 ms, and the reply is ready about 30 ms later. Entering and
  leaving take 5–15 ms.
* The silence after leaving does not depend on the session: about 0.6 s of
  interruption per read is the floor with this firmware.
* Playback cannot be measured from the host (the isochronous OUT endpoint has
  no feedback). By ear it stops for the same time.

No way to reach the SoC without the bootloader is known. Logitech ships no
software that shows the G435 battery; G HUB does not support the headset.

## SoC commands

```text
50 41 <category> <seq> <len> <command...>     request  ("PA")
50 49 ...                                     reply    ("PI")
```

| Command | Request | Reply |
|---|---|---|
| REMOTE_GET_CONNECT_STATUS | `0e <seq> 01 e3` | `02 00 e3 <0/1>` |
| REMOTE_GET_MODE | `0e <seq> 01 e0` | `02 00 e0 <mode>` |
| REMOTE_SET_MODE | `0e <seq> 02 e1 01` | status only |
| GET_BATTERY | `06 <seq> 01 00 31` | `07 00 06 <seq\|80> 00 <mV lo> <mV hi>` |

Category `0e` commands are answered by the dongle and also produce a status
reply `03 00 0e <seq|80> 00`.

### Remote mode

GET_BATTERY reaches the headset only in remote mode 1. In mode 0 the dongle
answers it itself with a small meaningless value (105 and 210 seen). The
dongle is in mode 0 after being plugged in, and was seen falling back from 1
to 0 within a couple of minutes without being unplugged (*measured*, cause
unknown). So a reply outside 2500–4600 mV means "set mode 1 and ask again".

### Read sequence

```text
06 01 03                         enter
PA 0e 01 01 e3                   connected?  0 -> headset off, stop
PA 06 02 01 00 31                battery     plausible -> done
PA 0e 03 02 e1 01                otherwise: remote mode 1
PA 06 04 01 00 31                battery again
06 08 03                         leave (always, also on error)
```

### Known way to hang the dongle

**Do not send GET_BATTERY as the first command of a session.** *Measured*: it
got a correct reply once out of four attempts; the other three got no reply
and left the dongle acknowledging every bridge message (`06 5a`) but returning
no data, also to later sessions, until it was unplugged and plugged back in.
Audio kept working. Sent after a category `0e` command in the same session,
GET_BATTERY has never hung it. The exact cause is not isolated.

All four attempts used the feature-report format. Whether the output-report
format is immune has not been tested; the read sequence keeps GET_BATTERY
second either way.

Other commands from the same firmware family are listed in
[endes' gist][gist]. `GET_RSSI` and `OTA_CMD_ECHO` were once followed by the
same hang. The OTA, flash and NVDS commands can damage the firmware and must
not be sent.

[endes]: https://dev.to/endes/usb-hid-down-the-rabbit-hole-logitech-g435-dongle-33if
[gist]: https://gist.github.com/endes0/21193c55e2e38c9b3e3a95492380059c
