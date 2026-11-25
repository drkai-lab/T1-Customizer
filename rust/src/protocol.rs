//! HID report encoders. Byte layout matches the Python implementation and the
//! vendor tool (FunKey / BasicKeys / MULKey / MouseKey / LEDkey).

pub const TYPE_KEY: u8 = 1;
pub const TYPE_MEDIA: u8 = 2;
pub const TYPE_MOUSE: u8 = 3;
pub const TYPE_LED: u8 = 8;
pub const MAX_KEY_GROUPS: usize = 5;
pub const DEFAULT_DATA_LEN: usize = 64;

pub fn type_byte(layer: u8, rtype: u8, report_id: u8) -> u8 {
    if report_id == 0 {
        rtype & 0x0F
    } else {
        ((layer & 0x0F) << 4) | (rtype & 0x0F)
    }
}

/// Group reports for a key assignment.
///
/// Group 0 is the modifier-only prologue. Groups 1..n carry usages. The list
/// is capped at 5 groups. A modifier-only assignment is an empty key list plus
/// `lone_mod`.
pub fn key_reports(
    keynum: u8,
    layer: u8,
    report_id: u8,
    keys: &[(u8, u8)],
    lone_mod: u8,
) -> Vec<[u8; 6]> {
    let count = keys.len().min(MAX_KEY_GROUPS);
    let keys = &keys[..count];
    let first_mod = keys.first().map(|pair| pair.0).unwrap_or(lone_mod);
    let tb = type_byte(layer, TYPE_KEY, report_id);
    let mut out = Vec::with_capacity(count + 1);
    for idx in 0..=count {
        let (mod_byte, usage) = if idx == 0 {
            (first_mod, 0)
        } else {
            keys[idx - 1]
        };
        out.push([keynum, tb, count as u8, idx as u8, mod_byte, usage]);
    }
    out
}

pub fn media_bytes(report_id: u8, action: &str) -> Option<(u8, u8)> {
    let table: &[(&str, u8, u8)] = match report_id {
        0 => &[
            ("play", 64, 0),
            ("prev", 128, 0),
            ("next", 0, 1),
            ("mute", 4, 0),
            ("vol_up", 2, 0),
            ("vol_down", 1, 0),
        ],
        2 => &[
            ("play", 0, 4),
            ("prev", 0, 11),
            ("next", 0, 10),
            ("mute", 0, 1),
            ("vol_up", 64, 0),
            ("vol_down", 128, 0),
        ],
        _ => &[
            ("play", 205, 0),
            ("prev", 182, 0),
            ("next", 181, 0),
            ("mute", 226, 0),
            ("vol_up", 233, 0),
            ("vol_down", 234, 0),
        ],
    };
    table
        .iter()
        .find(|(name, _, _)| *name == action)
        .map(|(_, b5, b6)| (*b5, *b6))
}

pub fn media_report(keynum: u8, layer: u8, report_id: u8, action: &str) -> Option<[u8; 4]> {
    let (b5, b6) = media_bytes(report_id, action)?;
    Some([keynum, type_byte(layer, TYPE_MEDIA, report_id), b5, b6])
}

pub fn mouse_payload(action: &str) -> Option<[u8; 5]> {
    Some(match action {
        "left" => [1, 0, 0, 0, 0],
        "right" => [2, 0, 0, 0, 0],
        "centre" => [4, 0, 0, 0, 0],
        "wheel_up" => [0, 0, 0, 1, 0],
        "wheel_down" => [0, 0, 0, 0xFF, 0],
        "ctrl_wheel_up" => [0, 0, 1, 1, 0],
        "ctrl_wheel_down" => [0, 0, 1, 0xFF, 0],
        "shift_wheel_up" => [0, 0, 1, 2, 0],
        "shift_wheel_down" => [0, 0, 1, 0xFF, 0],
        "alt_wheel_up" => [0, 0, 1, 4, 0],
        "alt_wheel_down" => [0, 0, 1, 0xFF, 0],
        _ => return None,
    })
}

pub fn mouse_report(keynum: u8, layer: u8, report_id: u8, action: &str) -> Option<[u8; 7]> {
    let payload = mouse_payload(action)?;
    Some([
        keynum,
        type_byte(layer, TYPE_MOUSE, report_id),
        payload[0],
        payload[1],
        payload[2],
        payload[3],
        payload[4],
    ])
}

pub fn led_report(layer: u8, report_id: u8, mode: u8) -> [u8; 3] {
    [0xB0, type_byte(layer, TYPE_LED, report_id), mode & 0x03]
}

pub fn layer_report(layer: u8) -> [u8; 2] {
    let layer = layer.clamp(1, 3);
    [0xA1, layer]
}

pub fn flash_report() -> [u8; 2] {
    [0xAA, 0xAA]
}

pub fn flash_led_report() -> [u8; 2] {
    [0xAA, 0xA1]
}

pub fn probe_report() -> [u8; 2] {
    [0x00, 0x00]
}

/// Payloads `write_key` sends: optional layer switch, group reports, flash.
pub fn write_key_payloads(
    report_id: u8,
    keynum: u8,
    layer: u8,
    keys: &[(u8, u8)],
    lone_mod: u8,
) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(layer_report(layer).to_vec());
    }
    for report in key_reports(keynum, layer, report_id, keys, lone_mod) {
        out.push(report.to_vec());
    }
    out.push(flash_report().to_vec());
    out
}

pub fn frame(payload: &[u8], report_id: u8, data_len: usize, had_report_id: bool) -> Vec<u8> {
    if had_report_id {
        let mut buf = vec![0u8; 1 + data_len];
        buf[0] = report_id;
        let n = payload.len().min(data_len);
        buf[1..1 + n].copy_from_slice(&payload[..n]);
        buf
    } else {
        let mut buf = vec![0u8; data_len];
        let n = payload.len().min(data_len);
        buf[..n].copy_from_slice(&payload[..n]);
        buf
    }
}

/// Probe order: the detected id first, then 3, 0, 2.
pub fn probe_candidates(report_id: u8, had_report_id: bool) -> Vec<u8> {
    if !had_report_id {
        vec![0]
    } else {
        let mut ids = vec![report_id];
        for rid in [3u8, 0, 2] {
            if rid != report_id {
                ids.push(rid);
            }
        }
        ids
    }
}

/// `SP_DEVICE_INTERFACE_DATA.cbSize`: 4 + GUID(16) + Flags(4) + pointer-sized Reserved.
pub fn sp_device_interface_data_cb_size() -> u32 {
    4 + 16 + 4 + std::mem::size_of::<usize>() as u32
}

/// cbSize stored in `SP_DEVICE_INTERFACE_DETAIL_DATA_W`. The path still starts at byte 4.
pub fn hid_detail_cb_size() -> u32 {
    if std::mem::size_of::<usize>() == 8 {
        8
    } else {
        6
    }
}

/// Linux `_IOC` encoding used by the usbfs ioctls.
pub fn usb_ioc(dir: u32, type_char: u8, nr: u32, size: u32) -> u32 {
    (dir << 30) | (u32::from(type_char) << 8) | nr | (size << 16)
}

pub fn usbfs_submiturb() -> u32 {
    usb_ioc(2, b'U', 10, 56)
}
pub fn usbfs_reapurbndelay() -> u32 {
    usb_ioc(1, b'U', 13, 8)
}
pub fn usbfs_control() -> u32 {
    usb_ioc(3, b'U', 0, 24)
}
pub fn usbfs_claiminterface() -> u32 {
    usb_ioc(2, b'U', 15, 4)
}

/// 64-bit usbfs `usbdevfs_urb`. Layout matches the Python ctypes structure (56 bytes).
#[cfg(target_pointer_width = "64")]
#[repr(C)]
pub struct UsbdevfsUrb {
    pub typ: u8,
    pub endpoint: u8,
    pub _pad0: u16,
    pub status: i32,
    pub flags: u32,
    pub _pad1: u32,
    pub buffer: u64,
    pub buffer_length: i32,
    pub actual_length: i32,
    pub start_frame: i32,
    pub stream_id: u32,
    pub error_count: i32,
    pub signr: u32,
    pub usercontext: u64,
}

/// 64-bit usbfs `usbdevfs_ctrltransfer` (24 bytes).
#[cfg(target_pointer_width = "64")]
#[repr(C)]
pub struct UsbdevfsCtrl {
    pub b_request_type: u8,
    pub b_request: u8,
    pub w_value: u16,
    pub w_index: u16,
    pub w_length: u16,
    pub timeout: u32,
    pub data: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_sizes_follow_pointer_width() {
        if cfg!(target_pointer_width = "64") {
            assert_eq!(sp_device_interface_data_cb_size(), 32);
            assert_eq!(hid_detail_cb_size(), 8);
        } else {
            assert_eq!(sp_device_interface_data_cb_size(), 28);
            assert_eq!(hid_detail_cb_size(), 6);
        }
    }

    #[test]
    fn ioctl_numbers_match_python() {
        assert_eq!(usbfs_submiturb(), 0x8038_550A);
        assert_eq!(usbfs_reapurbndelay(), 0x4008_550D);
        assert_eq!(usbfs_control(), 0xC018_5500);
        assert_eq!(usbfs_claiminterface(), 0x8004_550F);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn urb_struct_sizes() {
        assert_eq!(std::mem::size_of::<UsbdevfsUrb>(), 56);
        assert_eq!(std::mem::size_of::<UsbdevfsCtrl>(), 24);
    }
}
