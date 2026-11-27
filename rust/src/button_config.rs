//! Save / load of the whole button configuration as one JSON file.
//! This file is IDENTICAL in T1-Customizer and T1-VREmoKey (kept in
//! parity like `pair_chord.rs`); change both copies together.
//!
//! Until now the app burned assignments straight to the device and
//! `settings.json` kept only the description strings
//! (`settings["assignments"]`), so a configuration could not be moved
//! to another PC or restored after a reset. Every successful GUI write
//! now also records the full assignment under `settings["buttons"]`:
//!
//! ```json
//! "buttons": {
//!   "1": {                       // layer (preset) 1..=3
//!     "1": {"kind": "key", "groups": [[0, 44]], "lone": 0},
//!     "2": {"kind": "media", "name": "play"},
//!     "13": {"kind": "mouse", "name": "wheel_up"}
//!   }
//! }
//! ```
//!
//! A `key` entry stores the exact arguments of `device::write_key`:
//! up to [`MAX_GROUPS`] (modifier, usage) groups plus a lone modifier
//! byte (both empty is a cleared key). `media` / `mouse` store the
//! action name from the `keys::MEDIA_ACTIONS` / `keys::MOUSE_ACTIONS`
//! tables. Pair-chord assignments and dial settings already live in
//! settings (`chords`, `dial`), so the export simply carries those
//! sections along with the descriptions.
//!
//! The export document is the settings sections wrapped in a small
//! envelope (`format` + `version`) so a wrong file is rejected before
//! anything is applied. Import validates every button entry strictly
//! (it drives device writes); entries that fail validation are skipped
//! and counted, never half-applied. The `assignments` / `chords` /
//! `dial` sections are shape-checked (must be JSON objects) and then
//! carried as-is, matching how the rest of the app parses them
//! defensively.

use serde_json::{Map, Value, json};

use crate::keys;
use crate::settings::{self, Settings};

/// Envelope tag of an exported button configuration file.
pub const FORMAT: &str = "t1-button-config";

/// Envelope version this build writes and can read.
pub const FORMAT_VERSION: u64 = 1;

/// Most (modifier, usage) groups one key entry may hold. The editor
/// itself stages at most five; eight leaves headroom for hand edits.
pub const MAX_GROUPS: usize = 8;

/// One recorded assignment for one slot on one layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ButtonAssignment {
    /// The exact `device::write_key` arguments: (mods, usage) groups
    /// plus a lone modifier byte. Both empty clears the key.
    Key { groups: Vec<(u8, u8)>, lone: u8 },
    /// A `keys::MEDIA_ACTIONS` action name.
    Media(String),
    /// A `keys::MOUSE_ACTIONS` action name.
    Mouse(String),
}

fn assignment_to_value(assignment: &ButtonAssignment) -> Value {
    match assignment {
        ButtonAssignment::Key { groups, lone } => {
            let groups: Vec<Value> = groups
                .iter()
                .map(|(mods, usage)| json!([mods, usage]))
                .collect();
            json!({ "kind": "key", "groups": groups, "lone": lone })
        }
        ButtonAssignment::Media(name) => json!({ "kind": "media", "name": name }),
        ButtonAssignment::Mouse(name) => json!({ "kind": "mouse", "name": name }),
    }
}

fn byte_of(value: &Value) -> Option<u8> {
    value.as_u64().and_then(|n| u8::try_from(n).ok())
}

/// Parse one button entry, strictly: unknown kinds, out-of-range
/// slots and unknown media / mouse action names are all rejected so a
/// bad file can never reach the device.
pub fn assignment_from_value(value: &Value) -> Option<ButtonAssignment> {
    let obj = value.as_object()?;
    match obj.get("kind")?.as_str()? {
        "key" => {
            let raw = obj.get("groups")?.as_array()?;
            if raw.len() > MAX_GROUPS {
                return None;
            }
            let mut groups = Vec::with_capacity(raw.len());
            for pair in raw {
                let pair = pair.as_array()?;
                if pair.len() != 2 {
                    return None;
                }
                groups.push((byte_of(&pair[0])?, byte_of(&pair[1])?));
            }
            let lone = match obj.get("lone") {
                None => 0,
                Some(v) => byte_of(v)?,
            };
            Some(ButtonAssignment::Key { groups, lone })
        }
        "media" => {
            let name = obj.get("name")?.as_str()?;
            canonical_action_name(keys::MEDIA_ACTIONS, name)
                .map(|name| ButtonAssignment::Media(name.to_string()))
        }
        "mouse" => {
            let name = obj.get("name")?.as_str()?;
            canonical_action_name(keys::MOUSE_ACTIONS, name)
                .map(|name| ButtonAssignment::Mouse(name.to_string()))
        }
        _ => None,
    }
}

/// Resolve a stored media/mouse action name to the canonical
/// internal name the encoder accepts. Files written by builds with
/// the GUI display-name leak hold the display label ("Wheel Up")
/// instead of the internal name ("wheel_up"); accept those too and
/// normalize, so a config saved by such a build still restores.
fn canonical_action_name<'a>(table: &[(&'a str, &'a str)], name: &str) -> Option<&'a str> {
    table
        .iter()
        .find(|(n, _)| *n == name)
        .or_else(|| table.iter().find(|(_, label)| *label == name))
        .map(|(n, _)| *n)
}

/// Slots a button entry may target: KEY1..KEY6 plus the six knob
/// slots. Slots 7..=12 exist in the protocol but the GUI never assigns
/// them, so files carrying them are treated as invalid.
fn valid_slot(key: u8) -> bool {
    (1..=6).contains(&key) || (13..=18).contains(&key)
}

fn parse_slot(text: &str) -> Option<u8> {
    text.parse::<u8>().ok().filter(|k| valid_slot(*k))
}

fn parse_layer(text: &str) -> Option<u8> {
    text.parse::<u8>().ok().filter(|l| (1..=3).contains(l))
}

/// Record one assignment into `settings["buttons"][layer][key]`.
/// Called on every successful GUI write so the section always mirrors
/// what was last burned.
pub fn record(settings: &mut Settings, layer: u8, key: u8, assignment: &ButtonAssignment) {
    let buttons = settings::ensure_object(settings, "buttons");
    if !buttons
        .get(&layer.to_string())
        .is_some_and(|v| v.is_object())
    {
        buttons.insert(layer.to_string(), Value::Object(Map::new()));
    }
    if let Some(layer_map) = buttons
        .get_mut(&layer.to_string())
        .and_then(|v| v.as_object_mut())
    {
        layer_map.insert(key.to_string(), assignment_to_value(assignment));
    }
}

/// Every recorded assignment in a buttons map, sorted by layer then
/// key, with invalid entries skipped. This is the single parse the
/// write plan and the tests share.
pub fn recorded_in(buttons: &Map<String, Value>) -> Vec<(u8, u8, ButtonAssignment)> {
    let mut out = Vec::new();
    for (layer_text, layer_value) in buttons {
        let (Some(layer), Some(layer_map)) = (parse_layer(layer_text), layer_value.as_object())
        else {
            continue;
        };
        for (key_text, entry) in layer_map {
            let (Some(key), Some(assignment)) =
                (parse_slot(key_text), assignment_from_value(entry))
            else {
                continue;
            };
            out.push((layer, key, assignment));
        }
    }
    out.sort_by_key(|(layer, key, _)| (*layer, *key));
    out
}

/// Every recorded assignment in the live settings.
pub fn recorded(settings: &Settings) -> Vec<(u8, u8, ButtonAssignment)> {
    match settings.get("buttons").and_then(|v| v.as_object()) {
        Some(buttons) => recorded_in(buttons),
        None => Vec::new(),
    }
}

/// True when there is anything worth exporting: at least one recorded
/// button, or at least one configured pair (a pair-only configuration
/// is still a configuration).
pub fn has_exportable(settings: &Settings) -> bool {
    if !recorded(settings).is_empty() {
        return true;
    }
    settings
        .get("chords")
        .and_then(|v| v.as_object())
        .and_then(|c| c.get("pairs"))
        .and_then(|v| v.as_object())
        .is_some_and(|pairs| !pairs.is_empty())
}

/// Build the export document from the live settings.
pub fn build_export(settings: &Settings) -> Value {
    let mut doc = Map::new();
    doc.insert("format".into(), json!(FORMAT));
    doc.insert("version".into(), json!(FORMAT_VERSION));
    for section in ["buttons", "assignments", "chords", "dial"] {
        if let Some(value) = settings.get(section).filter(|v| v.is_object()) {
            doc.insert(section.into(), value.clone());
        }
    }
    if !doc.contains_key("buttons") {
        doc.insert("buttons".into(), Value::Object(Map::new()));
    }
    Value::Object(doc)
}

/// A validated import, ready to be applied to settings.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedConfig {
    /// Validated button entries, re-serialized in canonical shape.
    pub buttons: Map<String, Value>,
    pub assignments: Option<Value>,
    pub chords: Option<Value>,
    pub dial: Option<Value>,
    /// Button entries that failed validation and were skipped.
    pub skipped_entries: usize,
}

/// Parse and validate an export document. Errors are short stable
/// codes (shown to the user next to the localized message), never
/// panics, and nothing is applied on error.
pub fn parse_import(text: &str) -> Result<ImportedConfig, String> {
    let doc: Value = serde_json::from_str(text).map_err(|_| "not-json".to_string())?;
    let doc = doc.as_object().ok_or("not-object")?;
    if let Some(format) = doc.get("format")
        && format.as_str() != Some(FORMAT)
    {
        return Err("bad-format".into());
    }
    if let Some(version) = doc.get("version") {
        let version = version.as_u64().ok_or("bad-version")?;
        if version > FORMAT_VERSION {
            return Err("newer-version".into());
        }
    }
    let raw_buttons = doc
        .get("buttons")
        .and_then(|v| v.as_object())
        .ok_or("no-buttons")?;
    let mut buttons = Map::new();
    let mut skipped = 0usize;
    let mut valid = 0usize;
    for (layer_text, layer_value) in raw_buttons {
        let Some(layer_map) = layer_value.as_object() else {
            skipped += 1;
            continue;
        };
        for (key_text, entry) in layer_map {
            match (
                parse_layer(layer_text),
                parse_slot(key_text),
                assignment_from_value(entry),
            ) {
                (Some(layer), Some(key), Some(assignment)) => {
                    valid += 1;
                    let layer_key = layer.to_string();
                    if !buttons.get(&layer_key).is_some_and(|v| v.is_object()) {
                        buttons.insert(layer_key.clone(), Value::Object(Map::new()));
                    }
                    if let Some(map) = buttons.get_mut(&layer_key).and_then(|v| v.as_object_mut()) {
                        map.insert(key.to_string(), assignment_to_value(&assignment));
                    }
                }
                _ => skipped += 1,
            }
        }
    }
    if valid == 0 {
        return Err("empty-buttons".into());
    }
    let section = |name: &str| doc.get(name).filter(|v| v.is_object()).cloned();
    Ok(ImportedConfig {
        buttons,
        assignments: section("assignments"),
        chords: section("chords"),
        dial: section("dial"),
        skipped_entries: skipped,
    })
}

/// Replace the configuration sections in settings with the imported
/// ones. Sections the file does not carry are left untouched.
pub fn apply_import(settings: &mut Settings, imported: &ImportedConfig) {
    settings.insert("buttons".into(), Value::Object(imported.buttons.clone()));
    if let Some(assignments) = &imported.assignments {
        settings.insert("assignments".into(), assignments.clone());
    }
    if let Some(chords) = &imported.chords {
        settings.insert("chords".into(), chords.clone());
    }
    if let Some(dial) = &imported.dial {
        settings.insert("dial".into(), dial.clone());
    }
}

/// Why a recorded assignment is not written back on load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// Knob slot while the knob is not in key-assignment mode; the
    /// resident listener owns the knob then.
    KnobNeedsKeyMode,
    /// Media / mouse on KEY1..KEY6 while pair mode is on: the burn
    /// would replace the pair sentinel and fire immediately, the same
    /// refusal the Media / Mouse pages make.
    PairModeKeysOnly,
}

/// One device write a loaded configuration produces.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedWrite {
    pub layer: u8,
    pub key: u8,
    pub assignment: ButtonAssignment,
}

/// The writes a loaded configuration produces, in layer/key order,
/// plus the entries held back by the current mode guards. Key writes
/// are always planned: the GUI runs them through the same burn path
/// as a manual write, so pair mode stores the single and burns the
/// sentinel exactly as usual.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WritePlan {
    pub writes: Vec<PlannedWrite>,
    pub skipped: Vec<(u8, u8, SkipReason)>,
}

pub fn write_plan(settings: &Settings, knob_key_mode: bool, chords_enabled: bool) -> WritePlan {
    let mut plan = WritePlan::default();
    for (layer, key, assignment) in recorded(settings) {
        if (13..=18).contains(&key) && !knob_key_mode {
            plan.skipped
                .push((layer, key, SkipReason::KnobNeedsKeyMode));
            continue;
        }
        if chords_enabled
            && (1..=6).contains(&key)
            && !matches!(assignment, ButtonAssignment::Key { .. })
        {
            plan.skipped
                .push((layer, key, SkipReason::PairModeKeysOnly));
            continue;
        }
        plan.writes.push(PlannedWrite {
            layer,
            key,
            assignment,
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> Settings {
        let mut settings = Settings::new();
        record(
            &mut settings,
            1,
            1,
            &ButtonAssignment::Key {
                groups: vec![(0, 44)],
                lone: 0,
            },
        );
        record(
            &mut settings,
            1,
            2,
            &ButtonAssignment::Key {
                groups: vec![(1, 4), (2, 5)],
                lone: 3,
            },
        );
        record(&mut settings, 1, 3, &ButtonAssignment::Media("play".into()));
        record(
            &mut settings,
            2,
            13,
            &ButtonAssignment::Mouse("wheel_up".into()),
        );
        record(
            &mut settings,
            3,
            6,
            &ButtonAssignment::Key {
                groups: Vec::new(),
                lone: 0,
            },
        );
        settings
    }

    #[test]
    fn export_import_round_trip() {
        let mut settings = sample_settings();
        settings.insert("assignments".into(), json!({ "1": { "1": "Space" } }));
        settings.insert(
            "chords".into(),
            json!({ "enabled": false, "pairs": { "1+2": { "mods": 1, "usage": 44 } } }),
        );
        settings.insert("dial".into(), json!({ "press_action": "key" }));
        let text = serde_json::to_string_pretty(&build_export(&settings)).unwrap();
        let imported = parse_import(&text).expect("import");
        assert_eq!(imported.skipped_entries, 0);
        let mut restored = Settings::new();
        apply_import(&mut restored, &imported);
        assert_eq!(recorded(&restored), recorded(&settings));
        assert_eq!(restored.get("assignments"), settings.get("assignments"));
        assert_eq!(restored.get("chords"), settings.get("chords"));
        assert_eq!(restored.get("dial"), settings.get("dial"));
        // Multi-group combos and cleared keys survive the round trip.
        let entries = recorded(&restored);
        assert!(entries.contains(&(
            1,
            2,
            ButtonAssignment::Key {
                groups: vec![(1, 4), (2, 5)],
                lone: 3,
            },
        )));
        assert!(entries.contains(&(
            3,
            6,
            ButtonAssignment::Key {
                groups: Vec::new(),
                lone: 0,
            },
        )));
    }

    #[test]
    fn import_rejects_bad_files() {
        assert_eq!(parse_import("not json").unwrap_err(), "not-json");
        assert_eq!(parse_import("[1, 2]").unwrap_err(), "not-object");
        assert_eq!(
            parse_import(r#"{"format": "other", "buttons": {}}"#).unwrap_err(),
            "bad-format"
        );
        assert_eq!(
            parse_import(r#"{"version": 99, "buttons": {}}"#).unwrap_err(),
            "newer-version"
        );
        assert_eq!(parse_import(r#"{"dial": {}}"#).unwrap_err(), "no-buttons");
        assert_eq!(
            parse_import(r#"{"buttons": {"1": {"1": {"kind": "mystery"}}}}"#).unwrap_err(),
            "empty-buttons"
        );
    }

    #[test]
    fn import_skips_invalid_entries_and_counts_them() {
        let text = r#"{
            "format": "t1-button-config",
            "version": 1,
            "buttons": {
                "1": {
                    "1": {"kind": "key", "groups": [[0, 44]], "lone": 0},
                    "2": {"kind": "media", "name": "bogus"},
                    "7": {"kind": "key", "groups": [[0, 4]], "lone": 0},
                    "19": {"kind": "mouse", "name": "left"}
                },
                "4": {
                    "1": {"kind": "key", "groups": [[0, 4]], "lone": 0}
                },
                "2": {
                    "15": {"kind": "mouse", "name": "wheel_down"},
                    "16": {"kind": "key", "groups": [[0, 4], [0, 5], [0, 6], [0, 7], [0, 8], [0, 9], [0, 10], [0, 11], [0, 12]], "lone": 0}
                }
            }
        }"#;
        let imported = parse_import(text).expect("import");
        // bogus media name, slot 7, slot 19, layer 4, oversized groups.
        assert_eq!(imported.skipped_entries, 5);
        let mut settings = Settings::new();
        apply_import(&mut settings, &imported);
        assert_eq!(
            recorded(&settings),
            vec![
                (
                    1,
                    1,
                    ButtonAssignment::Key {
                        groups: vec![(0, 44)],
                        lone: 0,
                    },
                ),
                (2, 15, ButtonAssignment::Mouse("wheel_down".into())),
            ]
        );
    }

    #[test]
    fn import_normalizes_display_labels_from_buggy_builds() {
        // Configs recorded by builds with the GUI display-name leak
        // hold labels like "Wheel Up" / "Play/Pause" in place of the
        // internal action names. Import must accept and normalize
        // them instead of skipping the entries as invalid.
        let text = r#"{
            "format": "t1-button-config",
            "version": 1,
            "buttons": {
                "1": {
                    "13": {"kind": "mouse", "name": "Wheel Up"},
                    "3": {"kind": "media", "name": "Play/Pause"}
                }
            }
        }"#;
        let imported = parse_import(text).expect("import");
        assert_eq!(imported.skipped_entries, 0);
        let mut settings = Settings::new();
        apply_import(&mut settings, &imported);
        assert_eq!(
            recorded(&settings),
            vec![
                (1, 3, ButtonAssignment::Media("play".into())),
                (1, 13, ButtonAssignment::Mouse("wheel_up".into())),
            ]
        );
        // Canonical names and unknown names behave as before.
        assert_eq!(
            canonical_action_name(keys::MOUSE_ACTIONS, "wheel_up"),
            Some("wheel_up")
        );
        assert_eq!(canonical_action_name(keys::MOUSE_ACTIONS, "bogus"), None);
    }

    #[test]
    fn write_plan_respects_mode_guards() {
        let settings = sample_settings();
        // Cycle mode: the knob slot (layer 2, key 13) is held back.
        let plan = write_plan(&settings, false, false);
        assert_eq!(plan.writes.len(), 4);
        assert_eq!(plan.skipped, vec![(2, 13, SkipReason::KnobNeedsKeyMode)]);
        // Key mode: everything is planned, in layer/key order.
        let plan = write_plan(&settings, true, false);
        assert_eq!(plan.writes.len(), 5);
        assert!(plan.skipped.is_empty());
        let order: Vec<(u8, u8)> = plan.writes.iter().map(|w| (w.layer, w.key)).collect();
        assert_eq!(order, vec![(1, 1), (1, 2), (1, 3), (2, 13), (3, 6)]);
        // Pair mode: the media entry on KEY3 is held back, key
        // entries still go through the burn path.
        let plan = write_plan(&settings, true, true);
        assert_eq!(plan.writes.len(), 4);
        assert_eq!(plan.skipped, vec![(1, 3, SkipReason::PairModeKeysOnly)]);
    }

    #[test]
    fn has_exportable_covers_pair_only_configs() {
        let mut settings = Settings::new();
        assert!(!has_exportable(&settings));
        settings.insert(
            "chords".into(),
            json!({ "pairs": { "1+2": { "mods": 1, "usage": 44 } } }),
        );
        assert!(has_exportable(&settings));
        let settings = sample_settings();
        assert!(has_exportable(&settings));
    }
}
