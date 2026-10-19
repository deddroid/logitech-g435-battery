//! HID transport for the two known G435 LIGHTSPEED dongle revisions.
//!
//! * `046d:0acb` (HDT647): vendor collection 0xFFC0, report 6. Commands go out
//!   as feature reports through the Realtek bridge, which must be put in
//!   "bootloader" mode first; replies are polled with GET_FEATURE.
//!   Protocol from endes' research (dev.to "USB HID down the rabbit hole"),
//!   verified on real hardware.
//! * `046d:0adf`: vendor collection 0xFF00, plain 64-byte reports framed with a
//!   CRC-16. Ported from portyporty/g435-battery-percentage (not verified here).

use crate::protocol::{self, CAT_OTA, CAT_WIRELESS};
use hidapi::{HidApi, HidDevice};
use std::fmt;
use std::thread::sleep;
use std::time::{Duration, Instant};

const VID: u16 = 0x046d;
const PID_ACB: u16 = 0x0acb;
const PID_ADF: u16 = 0x0adf;
const USAGE_PAGE_ACB: u16 = 0xffc0;
const USAGE_PAGE_ADF: u16 = 0xff00;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub millivolts: u16,
    pub percent: u8,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    HeadsetOff,
    NoReply,
    Hid(hidapi::HidError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NotFound => write!(f, "dongle non trovato"),
            Error::HeadsetOff => write!(f, "cuffia spenta o non collegata"),
            Error::NoReply => write!(f, "nessuna risposta dalla cuffia"),
            Error::Hid(e) => write!(f, "errore HID: {e}"),
        }
    }
}

impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Error::Hid(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct Reader {
    api: Option<HidApi>,
}

impl Reader {
    pub fn new() -> Self {
        Reader { api: None }
    }

    pub fn read_battery(&mut self) -> Result<Reading> {
        let api = match &mut self.api {
            Some(api) => {
                api.refresh_devices()?;
                api
            }
            None => self.api.insert(HidApi::new()?),
        };

        let mv = if let Some(dev) = open(api, PID_ACB, USAGE_PAGE_ACB)? {
            acb::read_millivolts(&dev)?
        } else if let Some(dev) = open(api, PID_ADF, USAGE_PAGE_ADF)? {
            adf::read_millivolts(&dev)?
        } else {
            return Err(Error::NotFound);
        };
        Ok(Reading { millivolts: mv, percent: protocol::millivolts_to_percent(mv) })
    }
}

/// Opens the vendor collection of a dongle. On Windows every top-level
/// collection is a separate device path, so the usage page matters; on Linux
/// all collections share one hidraw node.
fn open(api: &HidApi, pid: u16, usage_page: u16) -> Result<Option<HidDevice>> {
    let mut candidates: Vec<_> = api
        .device_list()
        .filter(|d| d.vendor_id() == VID && d.product_id() == pid)
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    candidates.sort_by_key(|d| d.usage_page() != usage_page);
    Ok(Some(candidates[0].open_device(api)?))
}

pub fn debug(msg: impl FnOnce() -> String) {
    if std::env::var_os("G435_DEBUG").is_some() {
        eprintln!("[g435] {}", msg());
    }
}

fn hex(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

mod acb {
    use super::*;

    const REPORT_ID: u8 = 6;
    // The feature report is 63 bytes + report ID; Windows rejects other sizes.
    const FEATURE_LEN: usize = 64;
    const ACK_OK: u8 = 0x5a;
    const ACK_ALREADY: u8 = 0xfe;

    const ENTER_BOOTLOADER: [u8; 2] = [0x01, 0x03];
    const EXIT_BOOTLOADER: [u8; 2] = [0x08, 0x03];
    const SEND_COMMAND: [u8; 2] = [0x04, 0x03];

    /// Entering the bridge "bootloader" may glitch audio briefly, so the
    /// session always leaves it, even on error.
    struct Session<'a> {
        dev: &'a HidDevice,
        seq: u8,
    }

    impl Drop for Session<'_> {
        fn drop(&mut self) {
            if let Err(e) = bridge(self.dev, &EXIT_BOOTLOADER, &[]) {
                debug(|| format!("exit bootloader failed: {e}"));
            }
        }
    }

    impl Session<'_> {
        /// Sends a "PA" command and polls feature reports until `parse` matches.
        fn query<T>(
            &mut self,
            category: u8,
            len: u8,
            payload: &[u8],
            parse: impl Fn(&[u8]) -> Option<T>,
        ) -> Result<Option<T>> {
            let cmd = protocol::pa_command(category, self.seq, len, payload);
            self.seq = self.seq % 127 + 1;

            let mut msg = vec![cmd.len() as u8];
            msg.extend_from_slice(&cmd);
            bridge(self.dev, &SEND_COMMAND, &msg)?;

            let deadline = Instant::now() + Duration::from_millis(1500);
            while Instant::now() < deadline {
                let mut buf = [0u8; FEATURE_LEN];
                buf[0] = REPORT_ID;
                let n = self.dev.get_feature_report(&mut buf)?;
                let buf = &buf[..n];
                if buf.get(1).is_some_and(|&b| b != 0xff) {
                    debug(|| format!("reply {}", hex(buf)));
                    if let Some(v) = parse(buf) {
                        return Ok(Some(v));
                    }
                    continue;
                }
                sleep(Duration::from_millis(30));
            }
            Ok(None)
        }
    }

    /// Sends a bridge message as a feature report and waits for its ACK on the
    /// input endpoint (skipping volume-key reports that may be interleaved).
    fn bridge(dev: &HidDevice, op: &[u8; 2], body: &[u8]) -> Result<u8> {
        let mut report = [0u8; FEATURE_LEN];
        report[0] = REPORT_ID;
        report[1..3].copy_from_slice(op);
        report[3..3 + body.len()].copy_from_slice(body);
        debug(|| format!("send {}", hex(&report[..3 + body.len()])));
        dev.send_feature_report(&report)?;

        let deadline = Instant::now() + Duration::from_millis(800);
        let mut buf = [0u8; 64];
        while Instant::now() < deadline {
            let n = dev.read_timeout(&mut buf, 100)?;
            if n >= 2 && buf[0] == REPORT_ID {
                return Ok(buf[1]);
            }
        }
        Err(Error::NoReply)
    }

    fn drain(dev: &HidDevice) -> Result<()> {
        for _ in 0..16 {
            let mut buf = [0u8; FEATURE_LEN];
            buf[0] = REPORT_ID;
            let n = dev.get_feature_report(&mut buf)?;
            if n < 2 || buf[1] == 0xff {
                break;
            }
        }
        Ok(())
    }

    pub fn read_millivolts(dev: &HidDevice) -> Result<u16> {
        let ack = bridge(dev, &ENTER_BOOTLOADER, &[])?;
        if ack != ACK_OK && ack != ACK_ALREADY {
            debug(|| format!("unexpected enter ack {ack:02x}"));
        }
        let mut s = Session { dev, seq: 1 };
        drain(dev)?;

        let connected = s.query(CAT_WIRELESS, 1, &[protocol::REMOTE_GET_CONNECT_STATUS], |b| {
            protocol::parse_remote_value(b, protocol::REMOTE_GET_CONNECT_STATUS)
        })?;
        if connected == Some(0) {
            return Err(Error::HeadsetOff);
        }

        // Remote mode 1 routes OTA commands to the headset; in mode 0 the
        // dongle answers GET_BATTERY itself with a meaningless value.
        let mode = s.query(CAT_WIRELESS, 1, &[protocol::REMOTE_GET_MODE], |b| {
            protocol::parse_remote_value(b, protocol::REMOTE_GET_MODE)
        })?;
        if mode != Some(1) {
            debug(|| format!("remote mode {mode:?}, switching to 1"));
            s.query(CAT_WIRELESS, 2, &[protocol::REMOTE_SET_MODE, 0x01], |_| Some(()))?;
        }

        s.query(CAT_OTA, 1, &protocol::OTA_GET_BATTERY, protocol::parse_battery_mv)?
            .ok_or(Error::NoReply)
    }
}

mod adf {
    use super::*;

    // Session init captured by portyporty (OTA_CMD_IOCTL_READ_MAX_LEN, seq 1).
    const SESSION_INIT: [u8; 19] = [
        0x50, 0x41, 0x06, 0x01, 0x0d, 0x00, 0x25, 0x34, 0x12, 0x5a, 0x5a, 0x01, 0x00, 0x00,
        0x00, 0xf0, 0x00, 0x00, 0x00,
    ];

    /// Writes a frame (no report IDs: prefix 0) and collects input reports for `wait`.
    fn transact(dev: &HidDevice, frame: &[u8; 64], wait: u64) -> Result<Vec<u8>> {
        let mut out = [0u8; 65];
        out[1..].copy_from_slice(frame);
        dev.write(&out)?;

        let mut replies = Vec::new();
        let deadline = Instant::now() + Duration::from_millis(wait);
        let mut buf = [0u8; 65];
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            let n = dev.read_timeout(&mut buf, left.as_millis().max(1) as i32)?;
            if n > 0 {
                debug(|| format!("reply {}", hex(&buf[..n])));
                replies.extend_from_slice(&buf[..n]);
            }
        }
        Ok(replies)
    }

    fn pa(seq: &mut u8, category: u8, len: u8, payload: &[u8]) -> [u8; 64] {
        let frame = protocol::adf_frame(0x80, &protocol::pa_command(category, *seq, len, payload));
        *seq = *seq % 127 + 1;
        frame
    }

    pub fn read_millivolts(dev: &HidDevice) -> Result<u16> {
        transact(dev, &protocol::adf_frame(0x84, &[0x01]), 50)?;
        transact(dev, &protocol::adf_frame(0x85, &[0x01]), 50)?;
        transact(dev, &protocol::adf_frame(0x80, &SESSION_INIT), 450)?;

        let mut seq = 2;
        transact(dev, &pa(&mut seq, CAT_WIRELESS, 1, &[protocol::REMOTE_GET_MODE]), 120)?;
        transact(dev, &pa(&mut seq, CAT_WIRELESS, 2, &[protocol::REMOTE_SET_MODE, 0x01]), 120)?;
        transact(dev, &pa(&mut seq, CAT_WIRELESS, 1, &[protocol::REMOTE_GET_MODE]), 120)?;
        let replies = transact(dev, &pa(&mut seq, CAT_OTA, 1, &protocol::OTA_GET_BATTERY), 250)?;
        let _ = transact(dev, &protocol::adf_frame(0x85, &[0x02]), 50);

        protocol::parse_battery_mv(&replies).ok_or(Error::NoReply)
    }
}
