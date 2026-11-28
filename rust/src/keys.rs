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

/// Physical keyboard layout the key labels follow. `USAGE_KEYS` holds
/// the US (ANSI) labels; `JIS_LABELS` overrides the usages whose
/// unshifted character differs on a Japanese (JIS) keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyLayout {
    Jis,
    Us,
}

impl KeyLayout {
    /// Settings value (`settings["key_layout"]`); anything but `"us"`
    /// (including a missing value) means JIS.
    pub fn from_setting(value: Option<&str>) -> Self {
        match value {
            Some("us") => KeyLayout::Us,
            _ => KeyLayout::Jis,
        }
    }

    pub fn setting(self) -> &'static str {
        match self {
            KeyLayout::Jis => "jis",
            KeyLayout::Us => "us",
        }
    }
}

/// JIS labels for usages whose printed (unshifted) character differs
/// from the US label in `USAGE_KEYS`, keyed by HID usage. Verified
/// against the standard JIS layout: 0x2F is the @ key, 0x30 the [ key,
/// 0x31 the ] key, 0x34 the : key, and 0x35 the 半角/全角 key. Usages
/// not listed here print the same character on both layouts (e.g.
/// 0x2D "-", 0x33 ";"). Usage 0x32 ("#") has no JIS counterpart and
/// keeps its US label in both layouts.
pub const JIS_LABELS: &[(&str, u8)] = &[
    ("^", 46),
    ("@", 47),
    ("[", 48),
    ("]", 49),
    (":", 52),
    ("半角/全角", 53),
];

/// Keys that exist only on Japanese keyboards (HID usages 0x87–0x8B:
/// the 国際/International 1–5 usages). They are not in `USAGE_KEYS`
/// because they have no US meaning — the pickers list them under the
/// JIS layout only (see [`picker_keys`]). Their labels still resolve
/// under either layout, so a description recorded in JIS mode keeps
/// displaying after a switch to US. LANG1/LANG2 (0x90/0x91) are
/// deliberately absent: on the Windows JIS layout this selector
/// models, the kana keys are the ones below; LANG1/LANG2 are the
/// macOS JIS かな/英数 keys with no physical counterpart here.
pub const JIS_ONLY_KEYS: &[(&str, u8)] = &[
    ("ろ", 0x87),
    ("カタカナ/ひらがな", 0x88),
    ("¥", 0x89),
    ("変換", 0x8A),
    ("無変換", 0x8B),
];

/// True for usages that exist only on Japanese keyboards.
pub fn is_jis_only_usage(usage: u8) -> bool {
    JIS_ONLY_KEYS.iter().any(|(_, code)| *code == usage)
}

/// (label, usage) pairs the key pickers show under `layout`: every
/// `USAGE_KEYS` entry with its layout label, plus — under JIS only —
/// the JIS-only keys appended after them, so an index into the shared
/// part of the list means the same key in both layouts.
pub fn picker_keys(layout: KeyLayout) -> Vec<(&'static str, u8)> {
    let mut out: Vec<(&'static str, u8)> = USAGE_KEYS
        .iter()
        .map(|(_, usage)| (label_for_usage_layout(*usage, layout).unwrap_or(""), *usage))
        .collect();
    if layout == KeyLayout::Jis {
        out.extend_from_slice(JIS_ONLY_KEYS);
    }
    out
}

pub fn usage_for_label(label: &str) -> u8 {
    USAGE_KEYS
        .iter()
        .chain(JIS_ONLY_KEYS.iter())
        .find(|(name, _)| *name == label)
        .map(|(_, code)| *code)
        .unwrap_or(0)
}

/// Resolve a displayed label under `layout`. Falls back to the other
/// layout's labels so a label picked before a layout switch (e.g. the
/// combo editor's staged base key) still resolves to its usage. The
/// JIS-only labels resolve under both layouts.
pub fn usage_for_label_layout(label: &str, layout: KeyLayout) -> u8 {
    let tables: [&[(&str, u8)]; 3] = match layout {
        KeyLayout::Jis => [JIS_LABELS, USAGE_KEYS, JIS_ONLY_KEYS],
        KeyLayout::Us => [USAGE_KEYS, JIS_LABELS, JIS_ONLY_KEYS],
    };
    for table in tables {
        if let Some((_, code)) = table.iter().find(|(name, _)| *name == label) {
            return *code;
        }
    }
    0
}

pub fn label_for_usage(usage: u8) -> Option<&'static str> {
    USAGE_KEYS
        .iter()
        .chain(JIS_ONLY_KEYS.iter())
        .find(|(_, code)| *code == usage)
        .map(|(name, _)| *name)
}

pub fn label_for_usage_layout(usage: u8, layout: KeyLayout) -> Option<&'static str> {
    if layout == KeyLayout::Jis
        && let Some((name, _)) = JIS_LABELS.iter().find(|(_, code)| *code == usage)
    {
        return Some(name);
    }
    label_for_usage(usage)
}

pub fn clean_combo_name(name: &str) -> String {
    let flat = name.replace(['\r', '\n'], " ");
    let trimmed = flat.trim();
    trimmed.chars().take(MAX_COMBO_NAME).collect()
}

pub fn describe_mod_usage(mods: u8, usage: u8) -> String {
    describe_mod_usage_layout(mods, usage, KeyLayout::Us)
}

pub fn describe_mod_usage_layout(mods: u8, usage: u8, layout: KeyLayout) -> String {
    let mut names = Vec::new();
    for (name, mask) in MODIFIERS {
        if mods & mask != 0 {
            names.push(*name);
        }
    }
    if usage != 0
        && let Some(label) = label_for_usage_layout(usage, layout)
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
    describe_key_combo_layout(keys, lone_mod, KeyLayout::Us)
}

pub fn describe_key_combo_layout(keys: &[(u8, u8)], lone_mod: u8, layout: KeyLayout) -> String {
    let keys = normalize_key_groups(keys);
    if keys.is_empty() {
        describe_mod_usage_layout(lone_mod, 0, layout)
    } else {
        keys.iter()
            .map(|(mods, usage)| describe_mod_usage_layout(*mods, *usage, layout))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_labels_for_differing_usages() {
        // (usage, US label, JIS label) for every usage whose printed
        // character differs between the layouts.
        let cases: [(u8, &str, &str); 6] = [
            (46, "=", "^"),
            (47, "[", "@"),
            (48, "]", "["),
            (49, "\\", "]"),
            (52, "'", ":"),
            (53, "`", "半角/全角"),
        ];
        for (usage, us, jis) in cases {
            assert_eq!(label_for_usage_layout(usage, KeyLayout::Us), Some(us));
            assert_eq!(label_for_usage_layout(usage, KeyLayout::Jis), Some(jis));
            assert_eq!(usage_for_label_layout(us, KeyLayout::Us), usage);
            assert_eq!(usage_for_label_layout(jis, KeyLayout::Jis), usage);
        }
    }

    #[test]
    fn layout_labels_shared_usages_match() {
        for (usage, label) in [(45u8, "-"), (51, ";"), (54, ","), (56, "/"), (4, "A")] {
            assert_eq!(label_for_usage_layout(usage, KeyLayout::Jis), Some(label));
            assert_eq!(label_for_usage_layout(usage, KeyLayout::Us), Some(label));
        }
    }

    #[test]
    fn label_resolution_survives_a_layout_switch() {
        // A base label picked under one layout still resolves after
        // switching to the other.
        assert_eq!(usage_for_label_layout("=", KeyLayout::Jis), 46);
        assert_eq!(usage_for_label_layout("^", KeyLayout::Us), 46);
        assert_eq!(usage_for_label_layout("nope", KeyLayout::Jis), 0);
    }

    #[test]
    fn layout_setting_round_trip_and_default() {
        assert_eq!(KeyLayout::from_setting(None), KeyLayout::Jis);
        assert_eq!(KeyLayout::from_setting(Some("jis")), KeyLayout::Jis);
        assert_eq!(KeyLayout::from_setting(Some("us")), KeyLayout::Us);
        assert_eq!(KeyLayout::from_setting(Some("bogus")), KeyLayout::Jis);
        for layout in [KeyLayout::Jis, KeyLayout::Us] {
            assert_eq!(KeyLayout::from_setting(Some(layout.setting())), layout);
        }
    }

    #[test]
    fn jis_only_keys_resolve_in_both_layouts() {
        let cases: [(&str, u8); 5] = [
            ("ろ", 0x87),
            ("カタカナ/ひらがな", 0x88),
            ("¥", 0x89),
            ("変換", 0x8A),
            ("無変換", 0x8B),
        ];
        for (label, usage) in cases {
            assert!(is_jis_only_usage(usage));
            assert_eq!(label_for_usage(usage), Some(label));
            assert_eq!(usage_for_label(label), usage);
            // Labels resolve under either layout, so a description
            // recorded in JIS mode still displays in US mode.
            for layout in [KeyLayout::Jis, KeyLayout::Us] {
                assert_eq!(label_for_usage_layout(usage, layout), Some(label));
                assert_eq!(usage_for_label_layout(label, layout), usage);
            }
        }
        assert!(!is_jis_only_usage(0x04));
        assert!(!is_jis_only_usage(0x90));
    }

    #[test]
    fn picker_keys_visibility_follows_the_layout() {
        let jis = picker_keys(KeyLayout::Jis);
        let us = picker_keys(KeyLayout::Us);
        assert_eq!(jis.len(), USAGE_KEYS.len() + JIS_ONLY_KEYS.len());
        assert_eq!(us.len(), USAGE_KEYS.len());
        // The shared part is index-aligned across layouts.
        for (index, (_, usage)) in USAGE_KEYS.iter().enumerate() {
            assert_eq!(jis[index].1, *usage);
            assert_eq!(us[index].1, *usage);
        }
        // JIS-only keys are appended under JIS and absent under US.
        for (label, usage) in JIS_ONLY_KEYS {
            assert!(jis.contains(&(*label, *usage)));
            assert!(us.iter().all(|(_, code)| code != usage));
        }
        // Labels in the picker are the layout labels.
        assert_eq!(jis[25].0, "Z");
        let caret = USAGE_KEYS.iter().position(|(_, u)| *u == 46).unwrap();
        assert_eq!(jis[caret].0, "^");
        assert_eq!(us[caret].0, "=");
    }

    #[test]
    fn describe_jis_only_plain_and_in_combo() {
        // 変換 as a plain key and inside a modifier combo, in both
        // layouts (the US side resolves through the fallback).
        for layout in [KeyLayout::Jis, KeyLayout::Us] {
            assert_eq!(describe_key_combo_layout(&[(0, 0x8A)], 0, layout), "変換");
            assert_eq!(describe_mod_usage_layout(0x01, 0x8A, layout), "Ctrl+変換");
            assert_eq!(
                describe_key_combo_layout(&[(0x01, 0x8A), (0, 0x87)], 0, layout),
                "Ctrl+変換 / ろ"
            );
        }
        assert_eq!(describe_key_combo(&[(0, 0x8B)], 0), "無変換");
    }

    #[test]
    fn describe_uses_the_layout() {
        assert_eq!(
            describe_mod_usage_layout(0x01, 46, KeyLayout::Jis),
            "Ctrl+^"
        );
        assert_eq!(describe_mod_usage_layout(0x01, 46, KeyLayout::Us), "Ctrl+=");
        assert_eq!(
            describe_key_combo_layout(&[(0, 46)], 0, KeyLayout::Jis),
            "^"
        );
        assert_eq!(describe_key_combo(&[(0, 46)], 0), "=");
    }
}
