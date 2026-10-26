//! The headset's USB sound card: whether it is streaming audio (battery reads
//! drop audio for about a second on the 0x0ACB dongle, so they wait for
//! silence) and the hardware volume quirk that blocks Volume+.

#[cfg(target_os = "linux")]
mod imp {
    use std::fs;
    use std::path::PathBuf;

    /// "/proc/asound/cardN" directories of G435 dongles: USB sound cards expose
    /// "usbid" as "vvvv:pppp".
    fn cards() -> impl Iterator<Item = PathBuf> {
        fs::read_dir("/proc/asound")
            .into_iter()
            .flatten()
            .flatten()
            .map(|card| card.path())
            .filter(|path| {
                let usbid = fs::read_to_string(path.join("usbid")).unwrap_or_default();
                matches!(usbid.trim(), "046d:0acb" | "046d:0adf")
            })
    }

    pub fn headset_busy() -> bool {
        // Every open PCM substream has a status file whose first line is
        // "state: ...". PipeWire/PulseAudio close the device after a few idle
        // seconds.
        cards().any(|card| {
            let Ok(pcms) = fs::read_dir(card) else { return false };
            pcms.flatten()
                .filter(|p| p.file_name().to_string_lossy().starts_with("pcm"))
                .filter_map(|p| fs::read_dir(p.path()).ok())
                .flat_map(|subs| subs.flatten())
                .filter_map(|sub| fs::read_to_string(sub.path().join("status")).ok())
                .any(|status| status.contains("RUNNING"))
        })
    }

    /// The dongle drops Volume+ presses while its USB playback volume is at the
    /// maximum, as if the host were already at 100%. PipeWire leaves that
    /// hardware control at 100% and scales in software, so keep it one step
    /// below (an inaudible -0.4 dB).
    pub fn unblock_volume_up() {
        use alsa::mixer::{Mixer, Selem, SelemChannelId};

        for card in cards() {
            let Some(n) = card.file_name().and_then(|f| f.to_str()?.strip_prefix("card").map(str::to_owned)) else {
                continue;
            };
            let Ok(mixer) = Mixer::new(&format!("hw:{n}"), false) else { continue };
            for selem in mixer.iter().filter_map(Selem::new) {
                if !selem.has_playback_volume() {
                    continue;
                }
                let (_, max) = selem.get_playback_volume_range();
                if selem.get_playback_volume(SelemChannelId::mono()).is_ok_and(|v| v >= max) {
                    crate::device::debug(|| format!("hw:{n} playback volume at max, setting {}", max - 1));
                    let _ = selem.set_playback_volume_all(max - 1);
                }
            }
        }
    }
}

/// Not needed on Windows: there the system volume is the hardware volume.
#[cfg(not(target_os = "linux"))]
mod imp {
    /// Not implemented on Windows yet: reads happen on schedule.
    pub fn headset_busy() -> bool {
        false
    }

    #[allow(dead_code)]
    pub fn unblock_volume_up() {}
}

pub use imp::{headset_busy, unblock_volume_up};
