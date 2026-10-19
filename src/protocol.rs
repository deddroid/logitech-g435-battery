//! Pure (I/O free) parts of the G435 protocol: framing, CRC, response parsing
//! and the voltage -> percentage curve.
//!
//! Both dongle revisions talk to the same Airoha/Audiowise SoC command layer:
//! every command starts with `"PA"` (0x50 0x41), followed by
//! `category, sequence, length, command bytes...`; replies start with `"PI"`
//! (0x50 0x49). Only the USB transport differs between dongles.

/// Command categories (byte 2 of a "PA" command).
pub const CAT_OTA: u8 = 0x06;
pub const CAT_WIRELESS: u8 = 0x0e;

/// OTA category: GET_BATTERY (0x0031), reply carries millivolts.
pub const OTA_GET_BATTERY: [u8; 2] = [0x00, 0x31];
/// Wireless category: REMOTE_GET_MODE / REMOTE_SET_MODE / REMOTE_GET_CONNECT_STATUS.
pub const REMOTE_GET_MODE: u8 = 0xe0;
pub const REMOTE_SET_MODE: u8 = 0xe1;
pub const REMOTE_GET_CONNECT_STATUS: u8 = 0xe3;

/// Builds a "PA" command. `len` is the protocol length field, which for most
/// commands is `payload.len() - 1`.
pub fn pa_command(category: u8, seq: u8, len: u8, payload: &[u8]) -> Vec<u8> {
    let mut cmd = vec![0x50, 0x41, category, seq, len];
    cmd.extend_from_slice(payload);
    cmd
}

/// CRC-16/XMODEM, used by the 0x0ADF dongle framing.
pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// 64-byte frame for the 0x0ADF dongle: `cmd, body_len, crc_lo, crc_hi, body...`,
/// CRC computed over the whole frame with the CRC field zeroed.
pub fn adf_frame(cmd: u8, body: &[u8]) -> [u8; 64] {
    let mut frame = [0u8; 64];
    frame[0] = cmd;
    frame[1] = body.len() as u8;
    frame[4..4 + body.len()].copy_from_slice(body);
    let crc = crc16_xmodem(&frame);
    frame[2..4].copy_from_slice(&crc.to_le_bytes());
    frame
}

/// Finds a GET_BATTERY reply in a raw buffer: `07 00 06 <seq|0x80> 00 mv_lo mv_hi`.
/// Values outside a plausible Li-Po range are rejected: with the dongle in the
/// wrong remote mode the same reply carries garbage.
pub fn parse_battery_mv(buf: &[u8]) -> Option<u16> {
    buf.windows(7).find_map(|w| {
        if w[0] == 0x07 && w[1] == 0x00 && w[2] == CAT_OTA && w[3] & 0x80 != 0 && w[4] == 0x00 {
            let mv = u16::from_le_bytes([w[5], w[6]]);
            (2500..=4600).contains(&mv).then_some(mv)
        } else {
            None
        }
    })
}

/// Finds the value of a wireless-category "get" reply: `02 00 <cmd> <value>`.
pub fn parse_remote_value(buf: &[u8], cmd: u8) -> Option<u8> {
    buf.windows(4)
        .find(|w| w[0] == 0x02 && w[1] == 0x00 && w[2] == cmd)
        .map(|w| w[3])
}

/// Approximate state of charge from a resting Li-Po voltage.
pub fn millivolts_to_percent(mv: u16) -> u8 {
    const CURVE: [(u16, u8); 16] = [
        (4200, 100), (4150, 95), (4100, 90), (4050, 85), (4000, 80), (3950, 75),
        (3900, 70), (3850, 62), (3800, 55), (3750, 45), (3700, 35), (3650, 25),
        (3600, 15), (3550, 10), (3500, 5), (3400, 0),
    ];
    if mv >= CURVE[0].0 {
        return 100;
    }
    for pair in CURVE.windows(2) {
        let ((hi_mv, hi_pct), (lo_mv, lo_pct)) = (pair[0], pair[1]);
        if mv >= lo_mv {
            let t = (mv - lo_mv) as f32 / (hi_mv - lo_mv) as f32;
            return (lo_pct as f32 + t * (hi_pct - lo_pct) as f32).round() as u8;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn parses_acb_battery_reply() {
        // Captured from a 046d:0acb dongle (feature report 6).
        let buf = hex("06504901c875480d00070006820032100000ffffff");
        assert_eq!(parse_battery_mv(&buf), Some(4146));
    }

    #[test]
    fn rejects_battery_reply_in_wrong_mode() {
        // Same reply with remote mode 0: not a headset voltage.
        let buf = hex("06504901c0d64300000700068200d2000000ffff");
        assert_eq!(parse_battery_mv(&buf), None);
    }

    #[test]
    fn parses_remote_values() {
        let buf = hex("0650490ec7000000000200e001ffff");
        assert_eq!(parse_remote_value(&buf, REMOTE_GET_MODE), Some(1));
        let buf = hex("0650490ec2000000000200e301ffff");
        assert_eq!(parse_remote_value(&buf, REMOTE_GET_CONNECT_STATUS), Some(1));
        assert_eq!(parse_remote_value(&buf, REMOTE_GET_MODE), None);
    }

    #[test]
    fn adf_frame_matches_reference() {
        let frame = adf_frame(0x84, &[0x01]);
        assert_eq!(&frame[..2], &[0x84, 0x01]);
        let mut zeroed = frame;
        zeroed[2] = 0;
        zeroed[3] = 0;
        assert_eq!(crc16_xmodem(&zeroed).to_le_bytes(), [frame[2], frame[3]]);
        assert_eq!(crc16_xmodem(b"123456789"), 0x31c3);
    }

    #[test]
    fn percent_curve() {
        assert_eq!(millivolts_to_percent(4300), 100);
        assert_eq!(millivolts_to_percent(4146), 95);
        assert_eq!(millivolts_to_percent(3725), 40);
        assert_eq!(millivolts_to_percent(3300), 0);
    }
}
