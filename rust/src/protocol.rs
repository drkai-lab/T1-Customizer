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

/// Knob slots: K1-L/C/R and K2-L/C/R. On the wire they are ordinary
/// key numbers in the same reports as KEY1..KEY12 — two independent
/// reconstructions of the vendor app (the decompiled `MINI KeyBoard.exe`
/// protocol and the RSoft MacroPad `MapToByte`, which maps its knob
/// actions 23..=31 onto 13..=21) agree there is no separate knob command
/// or address space.
pub fn is_knob_slot(keynum: u8) -> bool {
    (13..=18).contains(&keynum)
}

/// Pause after a normal frame, in milliseconds. The vendor-class tools
/// pace their frames (the byte-verified macropad.py sleeps 10 ms after
/// every frame, "give the MCU time to store to flash"); sending a whole
/// write burst back-to-back can overrun the MCU while it is still
/// committing the previous save to flash, and the frames it drops are
/// silently lost — the host still sees a successful HID write.
pub const FRAME_DELAY_MS: u64 = 10;

/// Longer pause after a save/commit frame (`0xAA …`), which is what
/// triggers the actual flash programming on the device.
pub const SAVE_DELAY_MS: u64 = 60;

/// How long to wait after sending `payload` before the next frame.
pub fn frame_delay_after_ms(payload: &[u8]) -> u64 {
    if payload.first() == Some(&0xAA) {
        SAVE_DELAY_MS
    } else {
        FRAME_DELAY_MS
    }
}

/// One payload of a device write, labelled with the stage it belongs to
/// so a failure can be reported (and logged) precisely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteStep {
    pub stage: &'static str,
    pub payload: Vec<u8>,
}

fn step(stage: &'static str, payload: &[u8]) -> WriteStep {
    WriteStep {
        stage,
        payload: payload.to_vec(),
    }
}

/// The full step sequence `write_key` sends: optional layer switch,
/// group reports, save. Identical in shape for keys and knob slots;
/// only byte 0 of the group reports differs.
pub fn key_write_steps(
    report_id: u8,
    keynum: u8,
    layer: u8,
    keys: &[(u8, u8)],
    lone_mod: u8,
) -> Vec<WriteStep> {
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(step("layer", &layer_report(layer)));
    }
    for report in key_reports(keynum, layer, report_id, keys, lone_mod) {
        out.push(step("key", &report));
    }
    out.push(step("save", &flash_report()));
    out
}

/// The step sequence `write_media` sends.
pub fn media_write_steps(
    report_id: u8,
    keynum: u8,
    layer: u8,
    action: &str,
) -> Option<Vec<WriteStep>> {
    let report = media_report(keynum, layer, report_id, action)?;
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(step("layer", &layer_report(layer)));
    }
    out.push(step("media", &report));
    out.push(step("save", &flash_report()));
    Some(out)
}

/// The step sequence `write_mouse` sends.
pub fn mouse_write_steps(
    report_id: u8,
    keynum: u8,
    layer: u8,
    action: &str,
) -> Option<Vec<WriteStep>> {
    let report = mouse_report(keynum, layer, report_id, action)?;
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(step("layer", &layer_report(layer)));
    }
    out.push(step("mouse", &report));
    out.push(step("save", &flash_report()));
    Some(out)
}

/// The step sequence `write_led` sends.
pub fn led_write_steps(layer: u8, report_id: u8, mode: u8) -> Vec<WriteStep> {
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(step("layer", &layer_report(layer)));
    }
    out.push(step("led", &led_report(layer, report_id, mode)));
    out.push(step("save", &flash_led_report()));
    out
}

/// The step sequence for a *runtime-only* LED mode change: the LED
/// report without the flash save. The resident's preset-1 blink loop
/// toggles the mode about once a second; routing that through the
/// saving sequence would program the flash on every toggle. The
/// stored per-layer mode is untouched, so a layer switch or power
/// cycle restores the saved effect.
pub fn led_set_steps(layer: u8, report_id: u8, mode: u8) -> Vec<WriteStep> {
    let mut out = Vec::new();
    if report_id != 0 {
        out.push(step("layer", &layer_report(layer)));
    }
    out.push(step("led", &led_report(layer, report_id, mode)));
    out
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
    fn led_set_steps_never_saves_to_flash() {
        // The blink loop toggles the mode about once a second; the
        // runtime-only sequence must carry the LED report itself and
        // no save frame (0xAA …) in any position.
        for report_id in [0u8, 3] {
            let steps = led_set_steps(1, report_id, 0);
            assert!(steps.iter().all(|s| s.payload[0] != 0xAA));
            let led = steps.last().expect("an LED step");
            assert_eq!(led.payload, led_report(1, report_id, 0).to_vec());
            // The saving sequence for the same arguments is exactly
            // these steps plus the flash save at the end.
            let saving = led_write_steps(1, report_id, 0);
            assert_eq!(saving.len(), steps.len() + 1);
            assert_eq!(saving[..steps.len()], steps[..]);
            assert_eq!(saving[steps.len()].payload, flash_led_report().to_vec());
        }
    }

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
    fn named_reports_accept_knob_slots() {
        // Knob slots 13..=18 are ordinary keynums in the report's
        // first byte; the protocol does not restrict media/mouse
        // writes to KEY1..KEY6.
        for keynum in 13..=18u8 {
            let report = mouse_report(keynum, 1, 0, "left").expect("mouse report");
            assert_eq!(report[0], keynum);
            assert_eq!(report[1] & 0x0F, TYPE_MOUSE);
            let report = media_report(keynum, 1, 0, "play").expect("media report");
            assert_eq!(report[0], keynum);
            assert_eq!(report[1] & 0x0F, TYPE_MEDIA);
        }
    }

    fn payloads(steps: &[WriteStep]) -> Vec<Vec<u8>> {
        steps.iter().map(|s| s.payload.clone()).collect()
    }

    #[test]
    fn knob_key_write_frames_match_key_frames_except_byte0() {
        // K1-C (14) with a plain "A": layer switch, header report,
        // key report, save — byte-for-byte.
        let knob = payloads(&key_write_steps(3, 14, 1, &[(0, 0x04)], 0));
        assert_eq!(
            knob,
            vec![
                vec![0xA1, 1],
                vec![14, 0x11, 1, 0, 0, 0],
                vec![14, 0x11, 1, 1, 0, 0x04],
                vec![0xAA, 0xAA],
            ]
        );
        // KEY1 with the same combo is the identical sequence with
        // byte 0 = 1 in the two key reports: there is no separate
        // knob command or address space.
        let key = payloads(&key_write_steps(3, 1, 1, &[(0, 0x04)], 0));
        assert_eq!(key.len(), knob.len());
        for (k, n) in key.iter().zip(knob.iter()) {
            assert_eq!(k.len(), n.len());
            for (i, (kb, nb)) in k.iter().zip(n.iter()).enumerate() {
                if i == 0 && (k[0] == 1 || k[0] == 14) && k.len() > 2 {
                    assert_eq!(*kb, 1);
                    assert_eq!(*nb, 14);
                } else {
                    assert_eq!(kb, nb, "frame byte {i}");
                }
            }
        }
        // Stages are labelled for failure reporting.
        let stages: Vec<&str> = key_write_steps(3, 14, 1, &[(0, 0x04)], 0)
            .iter()
            .map(|s| s.stage)
            .collect();
        assert_eq!(stages, ["layer", "key", "key", "save"]);
    }

    #[test]
    fn knob_media_and_mouse_write_frames() {
        // K1-L (13) volume up (report-id-3 consumer code 233).
        let media = payloads(&media_write_steps(3, 13, 1, "vol_up").unwrap());
        assert_eq!(
            media,
            vec![vec![0xA1, 1], vec![13, 0x12, 233, 0], vec![0xAA, 0xAA]]
        );
        // K1-R (15) mouse left click.
        let mouse = payloads(&mouse_write_steps(3, 15, 1, "left").unwrap());
        assert_eq!(
            mouse,
            vec![
                vec![0xA1, 1],
                vec![15, 0x13, 1, 0, 0, 0, 0],
                vec![0xAA, 0xAA]
            ]
        );
        // Every knob slot is recognised, and no plain key is.
        for keynum in 13..=18u8 {
            assert!(is_knob_slot(keynum), "slot {keynum}");
        }
        for keynum in [1u8, 6, 7, 12, 19] {
            assert!(!is_knob_slot(keynum), "slot {keynum}");
        }
    }

    #[test]
    fn key_reports_carry_jis_only_usages_verbatim() {
        // JIS-only usages (0x87–0x8B) travel in the same single usage
        // byte as every other key; the encoder must not mask or
        // reject them. 変換 (0x8A) as a plain key and inside a combo:
        let reports = key_reports(1, 1, 0, &[(0, 0x8A)], 0);
        assert_eq!(reports[1], [1, TYPE_KEY, 1, 1, 0, 0x8A]);
        let reports = key_reports(1, 1, 0, &[(0x01, 0x8A), (0, 0x87)], 0);
        assert_eq!(reports[1], [1, TYPE_KEY, 2, 1, 0x01, 0x8A]);
        assert_eq!(reports[2], [1, TYPE_KEY, 2, 2, 0, 0x87]);
        for usage in 0x87..=0x8Bu8 {
            let reports = key_reports(6, 3, 3, &[(0, usage)], 0);
            assert_eq!(reports[1][5], usage, "usage {usage:#x}");
        }
    }

    #[test]
    fn frame_pacing_waits_longer_after_a_save() {
        assert_eq!(frame_delay_after_ms(&flash_report()), SAVE_DELAY_MS);
        assert_eq!(frame_delay_after_ms(&flash_led_report()), SAVE_DELAY_MS);
        assert_eq!(frame_delay_after_ms(&layer_report(1)), FRAME_DELAY_MS);
        assert_eq!(
            frame_delay_after_ms(&key_reports(14, 1, 3, &[(0, 0x04)], 0)[0]),
            FRAME_DELAY_MS
        );
    }

    #[test]
    fn action_tables_resolve_only_in_their_own_encoder() {
        for (action, _) in crate::keys::MOUSE_ACTIONS {
            assert!(mouse_payload(action).is_some(), "mouse action {action}");
            // A mouse action fed down the media path encodes to
            // nothing, which is why the GUI must never mix the two
            // paths (the Mouse page's write button once did).
            for rid in [0u8, 2, 3] {
                assert!(media_bytes(rid, action).is_none(), "mouse action {action}");
            }
        }
        for (action, _) in crate::keys::MEDIA_ACTIONS {
            for rid in [0u8, 2, 3] {
                assert!(media_bytes(rid, action).is_some(), "media action {action}");
            }
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn urb_struct_sizes() {
        assert_eq!(std::mem::size_of::<UsbdevfsUrb>(), 56);
        assert_eq!(std::mem::size_of::<UsbdevfsCtrl>(), 24);
    }
}
