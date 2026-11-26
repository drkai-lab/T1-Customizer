//! One-click application presets for well-known desktop apps.
//!
//! Each preset writes six plain key combinations (modifier mask + HID
//! usage, the same bytes the Modifiers page writes) to KEY1..KEY6 of the
//! current layer. The combinations are the apps' default Windows
//! shortcuts; see `docs/app-presets.md` for the full tables, the macOS
//! equivalents and the sources.

use crate::keys::{USAGE_KEYS, describe_key_combo};

/// Modifier masks, matching `keys::MODIFIERS`.
const CTRL: u8 = 0x01;
const SHIFT: u8 = 0x02;

/// HID usage for a label in `USAGE_KEYS`, resolved at compile time.
const fn usage_of(label: &str) -> u8 {
    let want = label.as_bytes();
    let mut index = 0;
    while index < USAGE_KEYS.len() {
        let (name, usage) = USAGE_KEYS[index];
        let have = name.as_bytes();
        if have.len() == want.len() {
            let mut at = 0;
            let mut same = true;
            while at < have.len() {
                if have[at] != want[at] {
                    same = false;
                    break;
                }
                at += 1;
            }
            if same {
                return usage;
            }
        }
        index += 1;
    }
    0
}

/// One key of a preset: a single modifier + key combination and what it
/// does in the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetKey {
    pub action_en: &'static str,
    pub action_ja: &'static str,
    pub mods: u8,
    pub usage: u8,
}

/// A whole app preset: six keys for KEY1..KEY6 plus a hint for what the
/// dial usually does in that app (the dial itself is not written).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub keys: [PresetKey; 6],
    pub dial_en: &'static str,
    pub dial_ja: &'static str,
}

const fn key(action_en: &'static str, action_ja: &'static str, mods: u8, label: &str) -> PresetKey {
    PresetKey {
        action_en,
        action_ja,
        mods,
        usage: usage_of(label),
    }
}

/// The built-in presets, in display order.
pub const APP_PRESETS: [AppPreset; 3] = [
    AppPreset {
        id: "premiere",
        name: "Adobe Premiere 2026",
        keys: [
            key("Add Edit", "編集点を追加", CTRL, "K"),
            key("Ripple Delete", "リップル削除", SHIFT, "Delete"),
            key(
                "Ripple Trim Previous",
                "前の編集点をリップルトリミング",
                0,
                "Q",
            ),
            key("Ripple Trim Next", "次の編集点をリップルトリミング", 0, "W"),
            key("Undo", "取り消し", CTRL, "Z"),
            key("Save", "保存", CTRL, "S"),
        ],
        dial_en: "= / - : timeline zoom",
        dial_ja: "= / -：タイムラインのズーム",
    },
    AppPreset {
        id: "illustrator",
        name: "Adobe Illustrator 2026",
        keys: [
            key("Selection Tool", "選択ツール", 0, "V"),
            key("Direct Selection Tool", "ダイレクト選択ツール", 0, "A"),
            key("Pen Tool", "ペンツール", 0, "P"),
            key("Type Tool", "文字ツール", 0, "T"),
            key("Undo", "元に戻す", CTRL, "Z"),
            key("Save", "保存", CTRL, "S"),
        ],
        dial_en: "Ctrl+= / Ctrl+- : zoom",
        dial_ja: "Ctrl+= / Ctrl+-：ズーム",
    },
    AppPreset {
        id: "photoshop",
        name: "Adobe Photoshop 2026",
        keys: [
            key("Move Tool", "移動ツール", 0, "V"),
            key("Brush Tool", "ブラシツール", 0, "B"),
            key("Eraser Tool", "消しゴムツール", 0, "E"),
            key("Free Transform", "自由変形", CTRL, "T"),
            key("Undo", "取り消し", CTRL, "Z"),
            key("Save", "保存", CTRL, "S"),
        ],
        dial_en: "[ / ] : brush size",
        dial_ja: "[ / ]：ブラシサイズ",
    },
];

/// The settings description recorded for a preset key, in the same form
/// the Modifiers page records (e.g. `Ctrl+K`, `Shift+Delete`).
pub fn description(key: &PresetKey) -> String {
    describe_key_combo(&[(key.mods, key.usage)], 0)
}

/// Look up a preset by its `id`.
pub fn preset(id: &str) -> Option<&'static AppPreset> {
    APP_PRESETS.iter().find(|preset| preset.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::label_for_usage;

    #[test]
    fn three_presets_with_six_keys_each() {
        assert_eq!(APP_PRESETS.len(), 3);
        assert_eq!(
            APP_PRESETS
                .iter()
                .map(|preset| preset.id)
                .collect::<Vec<_>>(),
            ["premiere", "illustrator", "photoshop"]
        );
        for preset in &APP_PRESETS {
            assert_eq!(preset.keys.len(), 6, "{}", preset.id);
            assert!(!preset.name.is_empty());
            assert!(!preset.dial_en.is_empty() && !preset.dial_ja.is_empty());
        }
    }

    #[test]
    fn every_usage_resolves_and_descriptions_are_non_empty() {
        for preset in &APP_PRESETS {
            for key in &preset.keys {
                assert_ne!(key.usage, 0, "{}: {}", preset.id, key.action_en);
                assert!(
                    label_for_usage(key.usage).is_some(),
                    "{}: usage {} has no label",
                    preset.id,
                    key.usage
                );
                let desc = description(key);
                assert!(
                    !desc.is_empty() && desc != "-",
                    "{}: {}",
                    preset.id,
                    key.action_en
                );
                assert!(!key.action_en.is_empty() && !key.action_ja.is_empty());
            }
        }
    }

    #[test]
    fn no_duplicate_combo_inside_one_preset() {
        for preset in &APP_PRESETS {
            for (index, key) in preset.keys.iter().enumerate() {
                for other in &preset.keys[index + 1..] {
                    assert_ne!(
                        (key.mods, key.usage),
                        (other.mods, other.usage),
                        "{}: duplicate combo",
                        preset.id
                    );
                }
            }
        }
    }

    #[test]
    fn spot_checks_match_the_adobe_defaults() {
        let premiere = preset("premiere").unwrap();
        assert_eq!(description(&premiere.keys[0]), "Ctrl+K");
        assert_eq!(description(&premiere.keys[1]), "Shift+Delete");
        assert_eq!(description(&premiere.keys[2]), "Q");
        let illustrator = preset("illustrator").unwrap();
        assert_eq!(description(&illustrator.keys[0]), "V");
        assert_eq!(description(&illustrator.keys[3]), "T");
        let photoshop = preset("photoshop").unwrap();
        assert_eq!(description(&photoshop.keys[3]), "Ctrl+T");
        assert_eq!(description(&photoshop.keys[5]), "Ctrl+S");
    }
}
