//! Key-code tables. Windows virtual keys and macOS HID usages are stored as
//! Linux evdev codes so `settings.json` stays portable.

use std::collections::HashMap;

pub const EV_SYN: u16 = 0x1;
pub const EV_KEY: u16 = 0x2;
pub const EV_REL: u16 = 0x3;
pub const KERNEL_EV_SYN: u16 = 0;
pub const KERNEL_EV_KEY: u16 = 1;
pub const KERNEL_EV_REL: u16 = 2;
pub const KEY_F1: u16 = 59;

pub const DIAL_DEFAULT_CODES: &[u16] = &[6, 7, 8, 11, 12];

const LETTERS: [u16; 26] = [
    30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45,
    21, 44,
];

pub fn vk_to_evdev() -> HashMap<u16, u16> {
    let mut table = HashMap::from([
        (0x08, 14),
        (0x09, 15),
        (0x0D, 28),
        (0x10, 42),
        (0x11, 29),
        (0x12, 56),
        (0x13, 119),
        (0x14, 58),
        (0x1B, 1),
        (0x20, 57),
        (0x2C, 99),
        (0x21, 104),
        (0x22, 109),
        (0x23, 107),
        (0x24, 102),
        (0x25, 105),
        (0x26, 103),
        (0x27, 106),
        (0x28, 108),
        (0x2D, 110),
        (0x2E, 111),
        (0x5B, 125),
        (0x5C, 126),
        (0x5D, 139),
        (0x90, 69),
        (0x91, 70),
        (0xA0, 42),
        (0xA1, 54),
        (0xA2, 29),
        (0xA3, 97),
        (0xA4, 56),
        (0xA5, 100),
        (0xAD, 113),
        (0xAE, 115),
        (0xAF, 114),
        (0xB0, 167),
        (0xB1, 166),
        (0xB2, 165),
        (0xB3, 164),
        (0xBD, 12),
        (0xBB, 13),
        (0xDB, 26),
        (0xDC, 43),
        (0xDD, 27),
        (0xBA, 39),
        (0xDE, 40),
        (0xC0, 41),
        (0xBC, 51),
        (0xBE, 52),
        (0xBF, 53),
        (0x6A, 55),
        (0x6B, 78),
        (0x6D, 74),
        (0x6E, 83),
        (0x6F, 98),
    ]);
    for (index, code) in LETTERS.iter().enumerate() {
        table.insert(0x41 + index as u16, *code);
    }
    table.insert(0x30, 11);
    for index in 1..10 {
        table.insert(0x30 + index, 1 + index);
    }
    for index in 0..12u16 {
        let code = if index < 10 {
            59 + index
        } else if index == 10 {
            87
        } else {
            88
        };
        table.insert(0x70 + index, code);
    }
    for (index, code) in [71, 72, 73, 75, 76, 77, 79, 80, 81, 82].iter().enumerate() {
        table.insert(0x60 + index as u16, *code);
    }
    table
}

pub fn hid_usage_to_evdev() -> HashMap<u16, u16> {
    let mut table = HashMap::new();
    for (index, code) in LETTERS.iter().enumerate() {
        table.insert(4 + index as u16, *code);
    }
    for index in 0..9u16 {
        table.insert(0x1E + index, 2 + index);
    }
    table.insert(0x27, 11);
    for (usage, code) in [
        (0x28, 28),
        (0x29, 1),
        (0x2A, 14),
        (0x2B, 15),
        (0x2C, 57),
        (0x2D, 12),
        (0x2E, 13),
        (0x2F, 26),
        (0x30, 27),
        (0x31, 43),
        (0x33, 39),
        (0x34, 40),
        (0x35, 41),
        (0x36, 51),
        (0x37, 52),
        (0x38, 53),
        (0x39, 58),
        (0x46, 99),
        (0x47, 70),
        (0x48, 119),
        (0x49, 110),
        (0x4A, 102),
        (0x4B, 104),
        (0x4C, 111),
        (0x4D, 107),
        (0x4E, 109),
        (0x4F, 106),
        (0x50, 105),
        (0x51, 108),
        (0x52, 103),
        (0x53, 69),
        (0x54, 98),
        (0x55, 55),
        (0x56, 74),
        (0x57, 78),
        (0x58, 96),
        (0xE0, 29),
        (0xE1, 42),
        (0xE2, 56),
        (0xE3, 125),
        (0xE4, 97),
        (0xE5, 54),
        (0xE6, 100),
        (0xE7, 126),
        (0x3A, 59),
        (0x3B, 60),
        (0x3C, 61),
        (0x3D, 62),
        (0x3E, 63),
        (0x3F, 64),
        (0x40, 65),
        (0x41, 66),
        (0x42, 67),
        (0x43, 68),
        (0x44, 87),
        (0x45, 88),
    ] {
        table.insert(usage, code);
    }
    for (offset, code) in [71, 72, 73, 75, 76, 77, 79, 80, 81, 82].iter().enumerate() {
        table.insert(0x59 + offset as u16, *code);
    }
    table
}

pub fn consumer_to_evdev() -> HashMap<u16, u16> {
    HashMap::from([
        (0xE2, 113),
        (0xE9, 114),
        (0xEA, 115),
        (0xB0, 167),
        (0xB5, 166),
        (0xCD, 164),
        (0xB7, 165),
    ])
}

pub fn evdev_key_names() -> HashMap<(u16, u16), &'static str> {
    let mut names = HashMap::new();
    let add = |names: &mut HashMap<(u16, u16), &'static str>, etype, code, name| {
        names.insert((etype, code), name);
    };
    for (etype, code, name) in [
        (EV_REL, 6, "REL_HWHEEL"),
        (EV_REL, 7, "REL_DIAL"),
        (EV_REL, 8, "REL_WHEEL"),
        (EV_REL, 11, "REL_WHEEL_HI_RES"),
        (EV_REL, 12, "REL_HWHEEL_HI_RES"),
        (EV_KEY, 272, "BTN_LEFT"),
        (EV_KEY, 273, "BTN_RIGHT"),
        (EV_KEY, 274, "BTN_MIDDLE"),
        (EV_KEY, 113, "KEY_MUTE"),
        (EV_KEY, 114, "KEY_VOLUMEUP"),
        (EV_KEY, 115, "KEY_VOLUMEDOWN"),
        (EV_KEY, 164, "KEY_PLAYPAUSE"),
        (EV_KEY, 165, "KEY_STOPCD"),
        (EV_KEY, 166, "KEY_PREVIOUSSONG"),
        (EV_KEY, 167, "KEY_NEXTSONG"),
    ] {
        add(&mut names, etype, code, name);
    }
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for (index, code) in LETTERS.iter().enumerate() {
        let name = match letters.as_bytes()[index] {
            b'A' => "KEY_A",
            b'B' => "KEY_B",
            b'C' => "KEY_C",
            b'D' => "KEY_D",
            b'E' => "KEY_E",
            b'F' => "KEY_F",
            b'G' => "KEY_G",
            b'H' => "KEY_H",
            b'I' => "KEY_I",
            b'J' => "KEY_J",
            b'K' => "KEY_K",
            b'L' => "KEY_L",
            b'M' => "KEY_M",
            b'N' => "KEY_N",
            b'O' => "KEY_O",
            b'P' => "KEY_P",
            b'Q' => "KEY_Q",
            b'R' => "KEY_R",
            b'S' => "KEY_S",
            b'T' => "KEY_T",
            b'U' => "KEY_U",
            b'V' => "KEY_V",
            b'W' => "KEY_W",
            b'X' => "KEY_X",
            b'Y' => "KEY_Y",
            _ => "KEY_Z",
        };
        add(&mut names, EV_KEY, *code, name);
    }
    for index in 0..9u16 {
        let name = match index {
            0 => "KEY_1",
            1 => "KEY_2",
            2 => "KEY_3",
            3 => "KEY_4",
            4 => "KEY_5",
            5 => "KEY_6",
            6 => "KEY_7",
            7 => "KEY_8",
            _ => "KEY_9",
        };
        add(&mut names, EV_KEY, 2 + index, name);
    }
    add(&mut names, EV_KEY, 11, "KEY_0");
    for index in 0..12u16 {
        let code = if index < 10 {
            59 + index
        } else if index == 10 {
            87
        } else {
            88
        };
        let name = match index {
            0 => "KEY_F1",
            1 => "KEY_F2",
            2 => "KEY_F3",
            3 => "KEY_F4",
            4 => "KEY_F5",
            5 => "KEY_F6",
            6 => "KEY_F7",
            7 => "KEY_F8",
            8 => "KEY_F9",
            9 => "KEY_F10",
            10 => "KEY_F11",
            _ => "KEY_F12",
        };
        add(&mut names, EV_KEY, code, name);
    }
    for (code, name) in [
        (1, "KEY_ESC"),
        (14, "KEY_BACKSPACE"),
        (15, "KEY_TAB"),
        (28, "KEY_ENTER"),
        (29, "KEY_LEFTCTRL"),
        (42, "KEY_LEFTSHIFT"),
        (54, "KEY_RIGHTSHIFT"),
        (56, "KEY_LEFTALT"),
        (57, "KEY_SPACE"),
        (58, "KEY_CAPSLOCK"),
        (97, "KEY_RIGHTCTRL"),
        (100, "KEY_RIGHTALT"),
        (102, "KEY_HOME"),
        (103, "KEY_UP"),
        (104, "KEY_PAGEUP"),
        (105, "KEY_LEFT"),
        (106, "KEY_RIGHT"),
        (107, "KEY_END"),
        (108, "KEY_DOWN"),
        (109, "KEY_PAGEDOWN"),
        (110, "KEY_INSERT"),
        (111, "KEY_DELETE"),
        (125, "KEY_LEFTMETA"),
        (126, "KEY_RIGHTMETA"),
        (139, "KEY_MENU"),
        (99, "KEY_SYSRQ"),
        (69, "KEY_NUMLOCK"),
        (70, "KEY_SCROLLLOCK"),
        (119, "KEY_PAUSE"),
        (12, "KEY_MINUS"),
        (13, "KEY_EQUAL"),
        (26, "KEY_LEFTBRACE"),
        (27, "KEY_RIGHTBRACE"),
        (39, "KEY_SEMICOLON"),
        (40, "KEY_APOSTROPHE"),
        (41, "KEY_GRAVE"),
        (43, "KEY_BACKSLASH"),
        (51, "KEY_COMMA"),
        (52, "KEY_DOT"),
        (53, "KEY_SLASH"),
    ] {
        add(&mut names, EV_KEY, code, name);
    }
    names
}

/// Kernel evdev type → the normalised type the monitor loop compares against.
///
/// Windows and macOS backends already emit `EV_KEY` (2) and `EV_REL` (3).
/// Linux `/dev/input` uses the kernel values (KEY=1, REL=2). Translating here
/// is what makes dial events fire on Linux; the Python reader returned
/// the raw kernel type, which the monitor never matched.
pub fn normalize_kernel_type(kernel_type: u16) -> Option<u16> {
    match kernel_type {
        KERNEL_EV_KEY => Some(EV_KEY),
        KERNEL_EV_REL => Some(EV_REL),
        KERNEL_EV_SYN => Some(EV_SYN),
        other => Some(other),
    }
}

pub fn input_event_size() -> usize {
    // struct input_event: two long + u16 + u16 + i32, aligned like Python's "llHHi".
    std::mem::size_of::<i64>() * 2 + 8
}

pub fn parse_input_events(blob: &[u8]) -> Vec<(u16, u16, i32)> {
    let size = if cfg!(target_pointer_width = "64") {
        24
    } else {
        16
    };
    let mut events = Vec::new();
    let mut offset = 0;
    while offset + size <= blob.len() {
        let chunk = &blob[offset..offset + size];
        let type_at = if cfg!(target_pointer_width = "64") {
            16
        } else {
            8
        };
        let ev_type = u16::from_le_bytes([chunk[type_at], chunk[type_at + 1]]);
        let code = u16::from_le_bytes([chunk[type_at + 2], chunk[type_at + 3]]);
        let value = i32::from_le_bytes([
            chunk[type_at + 4],
            chunk[type_at + 5],
            chunk[type_at + 6],
            chunk[type_at + 7],
        ]);
        events.push((ev_type, code, value));
        offset += size;
    }
    events
}

pub fn looks_like_keyboard(buf: &[u8]) -> bool {
    bits_include(buf, KEY_F1) || bits_include(buf, 42) || bits_include(buf, 54)
}

pub fn bits_include(buf: &[u8], code: u16) -> bool {
    let index = (code / 8) as usize;
    if index >= buf.len() {
        return false;
    }
    buf[index] & (1 << (code % 8)) != 0
}

/// macOS virtual keycodes used by a listen-only CGEvent tap, mapped to evdev.
pub fn mac_keycode_to_evdev(code: u16) -> Option<u16> {
    const F: [u16; 12] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111];
    if code == 56 {
        return Some(42);
    }
    if code == 60 {
        return Some(54);
    }
    F.iter().position(|item| *item == code).map(|index| {
        let index = index as u16;
        if index < 10 {
            59 + index
        } else if index == 10 {
            87
        } else {
            88
        }
    })
}
