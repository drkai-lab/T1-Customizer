//! Modifier combinations and HID usage labels. The tables match the Python tool.

use serde_json::{Map, Value};

use crate::protocol::MAX_KEY_GROUPS;

pub const MAX_COMBO_NAME: usize = 64;
pub const LEFT_SHIFT_MASK: u8 = 0x02;

pub const MODIFIERS: &[(&str, u8)] = &[
    ("Ctrl", 0x01),
    ("Shift", 0x02),
    ("Alt", 0x04),
    ("Win", 0x08),
    ("R-Ctrl", 0x10),
    ("R-Shift", 0x20),
    ("R-Alt", 0x40),
    ("R-Win", 0x80),
];

pub const USAGE_KEYS: &[(&str, u8)] = &[
    ("A", 4),
    ("B", 5),
    ("C", 6),
    ("D", 7),
    ("E", 8),
    ("F", 9),
    ("G", 10),
    ("H", 11),
    ("I", 12),
    ("J", 13),
    ("K", 14),
    ("L", 15),
    ("M", 16),
    ("N", 17),
    ("O", 18),
    ("P", 19),
    ("Q", 20),
    ("R", 21),
    ("S", 22),
    ("T", 23),
    ("U", 24),
    ("V", 25),
    ("W", 26),
    ("X", 27),
    ("Y", 28),
    ("Z", 29),
    ("1", 30),
    ("2", 31),
    ("3", 32),
    ("4", 33),
    ("5", 34),
    ("6", 35),
    ("7", 36),
    ("8", 37),
    ("9", 38),
    ("0", 39),
    ("Enter", 40),
    ("Esc", 41),
    ("Backspace", 42),
    ("Tab", 43),
    ("Space", 44),
    ("-", 45),
    ("=", 46),
    ("[", 47),
    ("]", 48),
    ("\\", 49),
    ("#", 50),
    (";", 51),
    ("'", 52),
    ("`", 53),
    (",", 54),
    (".", 55),
    ("/", 56),
    ("CapsLock", 57),
    ("F1", 58),
    ("F2", 59),
    ("F3", 60),
    ("F4", 61),
    ("F5", 62),
    ("F6", 63),
    ("F7", 64),
    ("F8", 65),
    ("F9", 66),
    ("F10", 67),
    ("F11", 68),
    ("F12", 69),
    ("PrtSc", 70),
    ("ScrollLock", 71),
    ("Pause", 72),
    ("Insert", 73),
    ("Home", 74),
    ("PageUp", 75),
    ("Delete", 76),
    ("End", 77),
    ("PageDown", 78),
    ("Right", 79),
    ("Left", 80),
    ("Down", 81),
    ("Up", 82),
    ("NumLock", 83),
    ("Num/", 84),
    ("Num*", 85),
    ("Num-", 86),
    ("Num+", 87),
    ("NumEnter", 88),
    ("Num1", 89),
    ("Num2", 90),
    ("Num3", 91),
    ("Num4", 92),
    ("Num5", 93),
    ("Num6", 94),
    ("Num7", 95),
    ("Num8", 96),
    ("Num9", 97),
    ("Num0", 98),
    ("Num.", 99),
    ("Menu", 101),
];

pub const MEDIA_ACTIONS: &[(&str, &str)] = &[
    ("play", "Play/Pause"),
    ("prev", "Prev Track"),
    ("next", "Next Track"),
    ("mute", "Mute"),
    ("vol_up", "Volume +"),
    ("vol_down", "Volume -"),
];

pub const MOUSE_ACTIONS: &[(&str, &str)] = &[
    ("left", "Mouse Left"),
    ("right", "Mouse Right"),
    ("centre", "Mouse Centre"),
    ("wheel_up", "Wheel Up"),
    ("wheel_down", "Wheel Down"),
    ("ctrl_wheel_up", "Ctrl + Wheel Up"),
    ("ctrl_wheel_down", "Ctrl + Wheel Down"),
    ("shift_wheel_up", "Shift + Wheel Up"),
    ("shift_wheel_down", "Shift + Wheel Down"),
    ("alt_wheel_up", "Alt + Wheel Up"),
    ("alt_wheel_down", "Alt + Wheel Down"),
];

pub const LED_MODES: &[(u8, &str)] = &[
    (0, "Mode 1 (Off/Static)"),
    (1, "Mode 2 (Breathing)"),
    (2, "Mode 3 (Wave)"),
];

/// Keys on the T1 itself (6 keys and a dial). The protocol still has slots 7–12.
pub const T1_KEY_COUNT: u8 = 6;

pub const PHYS_KEYS: &[(&str, u8)] = &[
    ("KEY1", 1),
    ("KEY2", 2),
    ("KEY3", 3),
    ("KEY4", 4),
    ("KEY5", 5),
    ("KEY6", 6),
    ("KEY7", 7),
    ("KEY8", 8),
    ("KEY9", 9),
    ("KEY10", 10),
    ("KEY11", 11),
    ("KEY12", 12),
    ("K1-L", 13),
    ("K1-C", 14),
    ("K1-R", 15),
    ("K2-L", 16),
    ("K2-C", 17),
    ("K2-R", 18),
];

pub const DIAL_MODES: &[(&str, &str, &str)] = &[
    ("mic", "Mic volume", "マイク音量"),
    ("headphone", "Headphone volume", "イヤーマフ音量"),
    ("user", "Selected user voice", "選択中ユーザーの声"),
    ("expressions", "Expressions", "表情"),
];

pub fn usage_for_label(label: &str) -> u8 {
    USAGE_KEYS
        .iter()
        .find(|(name, _)| *name == label)
        .map(|(_, code)| *code)
        .unwrap_or(0)
}

pub fn label_for_usage(usage: u8) -> Option<&'static str> {
    USAGE_KEYS
        .iter()
        .find(|(_, code)| *code == usage)
        .map(|(name, _)| *name)
}

pub fn clean_combo_name(name: &str) -> String {
    let flat = name.replace(['\r', '\n'], " ");
    let trimmed = flat.trim();
    trimmed.chars().take(MAX_COMBO_NAME).collect()
}

pub fn describe_mod_usage(mods: u8, usage: u8) -> String {
    let mut names = Vec::new();
    for (name, mask) in MODIFIERS {
        if mods & mask != 0 {
            names.push(*name);
        }
    }
    if usage != 0
        && let Some(label) = label_for_usage(usage)
    {
        names.push(label);
    }
    if names.is_empty() {
        "-".to_string()
    } else {
        names.join("+")
    }
}

pub fn normalize_key_groups(groups: &[(u8, u8)]) -> Vec<(u8, u8)> {
    groups
        .iter()
        .copied()
        .filter(|(_, usage)| *usage != 0)
        .take(MAX_KEY_GROUPS)
        .collect()
}

pub fn describe_key_combo(keys: &[(u8, u8)], lone_mod: u8) -> String {
    let keys = normalize_key_groups(keys);
    if keys.is_empty() {
        describe_mod_usage(lone_mod, 0)
    } else {
        keys.iter()
            .map(|(mods, usage)| describe_mod_usage(*mods, *usage))
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

pub fn editor_key_combo(groups: &[(u8, u8)], mods: u8, usage: u8) -> (Vec<(u8, u8)>, u8) {
    let keys = normalize_key_groups(groups);
    if !keys.is_empty() {
        let lone = keys[0].0;
        return (keys, lone);
    }
    if usage != 0 {
        return (vec![(mods, usage)], mods);
    }
    (Vec::new(), mods)
}

pub enum AppendStatus {
    Ok,
    NeedBase,
    Full,
}

pub fn append_key_group(groups: &[(u8, u8)], mods: u8, usage: u8) -> (Vec<(u8, u8)>, AppendStatus) {
    let current = normalize_key_groups(groups);
    if usage == 0 {
        return (current, AppendStatus::NeedBase);
    }
    if current.len() >= MAX_KEY_GROUPS {
        return (current, AppendStatus::Full);
    }
    let mut next = current;
    next.push((mods, usage));
    (next, AppendStatus::Ok)
}

#[derive(Clone, Debug)]
pub struct PendingCombo {
    pub desc: String,
    pub keys: Vec<(u8, u8)>,
    pub lone_mod: u8,
}

pub fn pending_key_combo(keys: &[(u8, u8)], lone_mod: u8) -> Option<PendingCombo> {
    if keys.is_empty() && lone_mod == 0 {
        return None;
    }
    Some(PendingCombo {
        desc: describe_key_combo(keys, lone_mod),
        keys: keys.to_vec(),
        lone_mod,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboRecord {
    pub name: String,
    pub keys: Vec<(u8, u8)>,
    pub lone_mod: u8,
}

fn as_byte(value: &Value) -> Option<u8> {
    if value.is_boolean() {
        return None;
    }
    value.as_u64().map(|n| (n & 0xFF) as u8).or_else(|| {
        value.as_i64().and_then(|n| {
            if (0..=255).contains(&n) {
                Some(n as u8)
            } else if n < 0 {
                None
            } else {
                Some((n as u64 & 0xFF) as u8)
            }
        })
    })
}

fn groups_from_value(value: &Value) -> Vec<(u8, u8)> {
    let Some(list) = value.as_array() else {
        return Vec::new();
    };
    let mut raw = Vec::new();
    for item in list {
        let Some(pair) = item.as_array() else {
            continue;
        };
        if pair.len() < 2 {
            continue;
        }
        let Some(mods) = as_byte(&pair[0]) else {
            continue;
        };
        let Some(usage) = as_byte(&pair[1]) else {
            continue;
        };
        raw.push((mods, usage));
    }
    normalize_key_groups(&raw)
}

fn combo_record(item: &Value) -> Option<ComboRecord> {
    let obj = item.as_object()?;
    let name = clean_combo_name(obj.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    if name.is_empty() {
        return None;
    }
    let keys = groups_from_value(obj.get("keys").unwrap_or(&Value::Null));
    let lone_byte = obj.get("lone_mod").and_then(as_byte).unwrap_or(0);
    let lone = if keys.is_empty() { lone_byte } else { 0 };
    if keys.is_empty() && lone == 0 {
        return None;
    }
    Some(ComboRecord {
        name,
        keys,
        lone_mod: lone,
    })
}

pub fn parse_modifier_combos(settings: &Map<String, Value>) -> Vec<ComboRecord> {
    let Some(raw) = settings.get("modifier_combos").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for item in raw {
        let Some(record) = combo_record(item) else {
            continue;
        };
        if seen.iter().any(|name| name == &record.name) {
            continue;
        }
        seen.push(record.name.clone());
        out.push(record);
    }
    out
}

pub fn upsert_modifier_combo(
    settings: &mut Map<String, Value>,
    name: &str,
    keys: &[(u8, u8)],
    lone_mod: u8,
) -> Result<ComboRecord, &'static str> {
    let name = clean_combo_name(name);
    if name.is_empty() {
        return Err("empty name");
    }
    let keys = normalize_key_groups(keys);
    let lone = if keys.is_empty() { lone_mod } else { 0 };
    if keys.is_empty() && lone == 0 {
        return Err("empty combo");
    }
    let record = ComboRecord {
        name,
        keys,
        lone_mod: lone,
    };
    let mut combos = parse_modifier_combos(settings);
    if let Some(existing) = combos.iter_mut().find(|item| item.name == record.name) {
        *existing = record.clone();
    } else {
        combos.push(record.clone());
    }
    settings.insert("modifier_combos".into(), combos_to_value(&combos));
    Ok(record)
}

pub fn delete_modifier_combo(settings: &mut Map<String, Value>, name: &str) -> Vec<ComboRecord> {
    let name = clean_combo_name(name);
    let combos: Vec<_> = parse_modifier_combos(settings)
        .into_iter()
        .filter(|item| item.name != name)
        .collect();
    settings.insert("modifier_combos".into(), combos_to_value(&combos));
    combos
}

fn combos_to_value(combos: &[ComboRecord]) -> Value {
    Value::Array(
        combos
            .iter()
            .map(|item| {
                let mut obj = Map::new();
                obj.insert("name".into(), Value::String(item.name.clone()));
                obj.insert(
                    "keys".into(),
                    Value::Array(
                        item.keys
                            .iter()
                            .map(|(m, u)| Value::Array(vec![Value::from(*m), Value::from(*u)]))
                            .collect(),
                    ),
                );
                obj.insert("lone_mod".into(), Value::from(item.lone_mod));
                Value::Object(obj)
            })
            .collect(),
    )
}
