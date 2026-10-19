//! Tells whether the headset is currently streaming audio, so that battery
//! reads (which drop audio for about a second on the 0x0ACB dongle) can wait
//! for silence.

#[cfg(target_os = "linux")]
pub fn headset_busy() -> bool {
    use std::fs;

    // USB sound cards expose "/proc/asound/cardN/usbid" as "vvvv:pppp"; every
    // open PCM substream has a status file whose first line is "state: ...".
    // PipeWire/PulseAudio close the device after a few idle seconds.
    let Ok(cards) = fs::read_dir("/proc/asound") else { return false };
    cards.flatten().any(|card| {
        let usbid = fs::read_to_string(card.path().join("usbid")).unwrap_or_default();
        if !matches!(usbid.trim(), "046d:0acb" | "046d:0adf") {
            return false;
        }
        let Ok(pcms) = fs::read_dir(card.path()) else { return false };
        pcms.flatten()
            .filter(|p| p.file_name().to_string_lossy().starts_with("pcm"))
            .filter_map(|p| fs::read_dir(p.path()).ok())
            .flat_map(|subs| subs.flatten())
            .filter_map(|sub| fs::read_to_string(sub.path().join("status")).ok())
            .any(|status| status.contains("RUNNING"))
    })
}

/// Not implemented on Windows yet: reads happen on schedule.
#[cfg(not(target_os = "linux"))]
pub fn headset_busy() -> bool {
    false
}
