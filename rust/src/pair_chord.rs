//! Key-pair chords: two T1 keys pressed together fire their own combo,
//! with mis-input prevention. This file is IDENTICAL in T1-Customizer and
//! T1-VREmoKey (kept in parity like `app_presets.rs`); change both copies
//! together.
//!
//! Why sentinels: the T1 sends its burned combo the instant a key goes
//! down, so two ordinary singles would both fire before any software
//! could decide they were meant as a pair. While pair mode is on, every
//! participating KEY1..KEY6 therefore carries a SENTINEL combo instead
//! (Ctrl+Shift+F1..F6, one per key). The resident listener intercepts the
//! sentinels, holds a lone press for up to `interval_ms`, and only then
//! decides: partner down inside the interval injects the pair combo and
//! swallows both singles; otherwise the stored single combo is injected.
//! The trade-off is that a participating single press can be delayed by
//! up to the interval.
//!
//! This module is the platform-independent half: settings shape, the
//! hold/decide state machine, and combo → evdev / virtual-key code maps.
//! Grabbing, forwarding and injection live in `input.rs` / `resident.rs`.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::settings::Settings;
use crate::tables;

/// Modifier byte of every pair sentinel: left Ctrl + left Shift. In
/// T1-VREmoKey the plain Shift+F keys are the expression gestures and
/// Shift+Ctrl+F from a PC keyboard is an expression hotkey; the pair
/// sentinels below are the same Ctrl+Shift+F1..F6 shape, but they are
/// only ever burned onto the T1's own KEY1..KEY6 and, while pair mode is
/// on, pair interception takes precedence for them.
pub const SENTINEL_MODS: u8 = 0x01 | 0x02;

/// Default hold window for deciding single vs. pair, in milliseconds.
pub const DEFAULT_INTERVAL_MS: u64 = 100;

/// The seven assignable pairs as (first, second) key numbers, in the
/// order their settings names are written: across the top row, across
/// the bottom row, then the three columns (bottom key first).
pub const PAIRS: [(u8, u8); 7] = [(1, 2), (2, 3), (4, 5), (5, 6), (4, 1), (5, 2), (6, 3)];

/// The pair containing both keys, in canonical (settings-name) order.
pub fn canonical_pair(a: u8, b: u8) -> Option<(u8, u8)> {
    PAIRS
        .iter()
        .copied()
        .find(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

/// Settings key for a pair, e.g. `"4+1"` for keys 1 and 4.
pub fn pair_name(a: u8, b: u8) -> Option<String> {
    canonical_pair(a, b).map(|(x, y)| format!("{x}+{y}"))
}

/// HID usage of the sentinel for a key: F1..F6 for KEY1..KEY6.
pub fn sentinel_usage(keynum: u8) -> u8 {
    57 + keynum
}

/// The sentinel combo burned onto a key while pair mode is on.
pub fn sentinel_combo(keynum: u8) -> Combo {
    Combo {
        mods: SENTINEL_MODS,
        usage: sentinel_usage(keynum),
    }
}

/// Inverse of [`sentinel_usage`]: the key number for an F1..F6 usage.
pub fn keynum_for_sentinel_usage(usage: u8) -> Option<u8> {
    (58..=63).contains(&usage).then(|| usage - 57)
}

/// One key assignment: a modifier bitmask (the `keys::MODIFIERS` bits)
/// plus one HID usage. `usage == 0` with mods set is a modifiers-only
/// press; both zero is "nothing assigned".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Combo {
    pub mods: u8,
    pub usage: u8,
}

impl Combo {
    pub fn is_empty(self) -> bool {
        self.mods == 0 && self.usage == 0
    }
}

/// Everything pair mode needs, parsed from `settings["chords"]`.
#[derive(Clone, Debug, PartialEq)]
pub struct PairChordConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    /// Configured pair combos, keyed by canonical pair order.
    pub pairs: HashMap<(u8, u8), Combo>,
    /// The real single-key combos, keyed by key number 1..=6. While pair
    /// mode is on the device carries sentinels, so this is the only
    /// record of what a lone press should inject (and what turning pair
    /// mode off burns back).
    pub singles: HashMap<u8, Combo>,
}

impl Default for PairChordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: DEFAULT_INTERVAL_MS,
            pairs: HashMap::new(),
            singles: HashMap::new(),
        }
    }
}

fn combo_from_value(value: &Value) -> Option<Combo> {
    let obj = value.as_object()?;
    let byte = |name: &str| {
        obj.get(name)
            .and_then(|v| v.as_u64())
            .map(|n| (n & 0xFF) as u8)
            .unwrap_or(0)
    };
    let combo = Combo {
        mods: byte("mods"),
        usage: byte("usage"),
    };
    if combo.is_empty() { None } else { Some(combo) }
}

fn combo_to_value(combo: Combo) -> Value {
    json!({ "mods": combo.mods, "usage": combo.usage })
}

impl PairChordConfig {
    pub fn from_settings(settings: &Settings) -> Self {
        let mut cfg = Self::default();
        let Some(chords) = settings.get("chords").and_then(|v| v.as_object()) else {
            return cfg;
        };
        cfg.enabled = matches!(chords.get("enabled"), Some(Value::Bool(true)));
        if let Some(ms) = chords.get("interval_ms").and_then(|v| v.as_u64()) {
            cfg.interval_ms = ms.min(1000);
        }
        if let Some(pairs) = chords.get("pairs").and_then(|v| v.as_object()) {
            for (name, value) in pairs {
                let Some((a, b)) = name
                    .split_once('+')
                    .and_then(|(a, b)| Some((a.parse::<u8>().ok()?, b.parse::<u8>().ok()?)))
                else {
                    continue;
                };
                if let (Some(pair), Some(combo)) = (canonical_pair(a, b), combo_from_value(value)) {
                    cfg.pairs.insert(pair, combo);
                }
            }
        }
        if let Some(singles) = chords.get("singles").and_then(|v| v.as_object()) {
            for (name, value) in singles {
                let Ok(key) = name.parse::<u8>() else {
                    continue;
                };
                if (1..=6).contains(&key)
                    && let Some(combo) = combo_from_value(value)
                {
                    cfg.singles.insert(key, combo);
                }
            }
        }
        cfg
    }

    /// The combo configured for the pair containing both keys, if any.
    pub fn pair_combo(&self, a: u8, b: u8) -> Option<Combo> {
        canonical_pair(a, b)
            .and_then(|pair| self.pairs.get(&pair))
            .copied()
    }

    /// The stored single combo for a key, if any.
    pub fn single(&self, key: u8) -> Option<Combo> {
        self.singles.get(&key).copied()
    }

    /// True when the key takes part in at least one configured pair, so
    /// its presses must go through the sentinel path.
    pub fn participates(&self, key: u8) -> bool {
        self.pairs.keys().any(|&(a, b)| a == key || b == key)
    }
}

fn chords_obj_mut(settings: &mut Settings) -> &mut Map<String, Value> {
    if !settings.get("chords").is_some_and(|v| v.is_object()) {
        settings.insert("chords".into(), Value::Object(Map::new()));
    }
    settings
        .get_mut("chords")
        .and_then(|v| v.as_object_mut())
        .expect("chords object")
}

fn named_obj_mut<'a>(parent: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !parent.get(key).is_some_and(|v| v.is_object()) {
        parent.insert(key.into(), Value::Object(Map::new()));
    }
    parent
        .get_mut(key)
        .and_then(|v| v.as_object_mut())
        .expect("object")
}

/// True when pair mode is switched on in settings.
pub fn chords_enabled(settings: &Settings) -> bool {
    PairChordConfig::from_settings(settings).enabled
}

/// Store the master toggle. Burning/restoring the device is the caller's
/// job (see the GUI toggle handlers).
pub fn set_enabled(settings: &mut Settings, on: bool) {
    chords_obj_mut(settings).insert("enabled".into(), Value::Bool(on));
}

/// Store or clear one pair assignment.
pub fn set_pair(settings: &mut Settings, a: u8, b: u8, combo: Option<Combo>) {
    let Some(name) = pair_name(a, b) else {
        return;
    };
    let chords = chords_obj_mut(settings);
    let pairs = named_obj_mut(chords, "pairs");
    match combo.filter(|c| !c.is_empty()) {
        Some(combo) => {
            pairs.insert(name, combo_to_value(combo));
        }
        None => {
            pairs.remove(&name);
        }
    }
}

/// Store or clear the real single assignment for a key.
pub fn set_single(settings: &mut Settings, key: u8, combo: Option<Combo>) {
    if !(1..=6).contains(&key) {
        return;
    }
    let chords = chords_obj_mut(settings);
    let singles = named_obj_mut(chords, "singles");
    match combo.filter(|c| !c.is_empty()) {
        Some(combo) => {
            singles.insert(key.to_string(), combo_to_value(combo));
        }
        None => {
            singles.remove(&key.to_string());
        }
    }
}

/// What the resident should inject on behalf of a swallowed sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    InjectDown(Combo),
    InjectUp(Combo),
}

struct Held {
    key: u8,
    deadline: u64,
}

struct Active {
    a: u8,
    b: Option<u8>,
    /// What was injected on behalf of these keys; `None` when the press
    /// produced nothing (unassigned single) and the ups are swallowed.
    combo: Option<Combo>,
    up_sent: bool,
    a_up: bool,
    b_up: bool,
}

impl Active {
    fn single(key: u8, combo: Option<Combo>) -> Self {
        Self {
            a: key,
            b: None,
            combo,
            up_sent: false,
            a_up: false,
            b_up: true,
        }
    }

    fn contains(&self, key: u8) -> bool {
        self.a == key || self.b == Some(key)
    }
}

/// The hold/decide state machine. Feed it sentinel key edges with
/// millisecond timestamps from one clock; it answers with the injections
/// that replace them. Key repeats never reach it (callers drop them, and
/// a repeated down for a key already held or active is ignored anyway).
pub struct PairChordEngine {
    cfg: PairChordConfig,
    held: Option<Held>,
    active: Vec<Active>,
}

impl PairChordEngine {
    pub fn new(cfg: PairChordConfig) -> Self {
        Self {
            cfg,
            held: None,
            active: Vec::new(),
        }
    }

    pub fn config(&self) -> &PairChordConfig {
        &self.cfg
    }

    pub fn is_idle(&self) -> bool {
        self.held.is_none() && self.active.is_empty()
    }

    /// When the pending hold expires, if it has not by the last tick.
    pub fn next_deadline(&self) -> Option<u64> {
        self.held.as_ref().map(|held| held.deadline)
    }

    /// Release everything that was injected, for shutdown or a config
    /// swap, so no injected key is left down.
    pub fn shutdown(&mut self) -> Vec<Action> {
        let mut out = Vec::new();
        for act in self.active.drain(..) {
            if !act.up_sent
                && let Some(combo) = act.combo
            {
                out.push(Action::InjectUp(combo));
            }
        }
        self.held = None;
        out
    }

    fn resolve_held_as_single(&mut self, held: Held, out: &mut Vec<Action>) {
        let combo = self.cfg.single(held.key);
        if let Some(combo) = combo {
            out.push(Action::InjectDown(combo));
        }
        self.active.push(Active::single(held.key, combo));
    }

    pub fn key_down(&mut self, key: u8, now: u64) -> Vec<Action> {
        if !(1..=6).contains(&key) {
            return Vec::new();
        }
        if !self.cfg.enabled {
            // Passthrough: the resident normally never routes events
            // here while disabled, but if it does, the single fires at
            // once with no hold and no delay.
            if self.held.as_ref().is_some_and(|h| h.key == key)
                || self.active.iter().any(|act| act.contains(key))
            {
                return Vec::new();
            }
            let combo = self.cfg.single(key);
            self.active.push(Active::single(key, combo));
            return combo.map(Action::InjectDown).into_iter().collect();
        }
        if self.held.as_ref().is_some_and(|h| h.key == key)
            || self.active.iter().any(|act| act.contains(key))
        {
            return Vec::new(); // repeat, or a key already counted as down
        }
        if let Some(held) = self.held.take() {
            if let Some(pair) = self.cfg.pair_combo(held.key, key) {
                self.active.push(Active {
                    a: held.key,
                    b: Some(key),
                    combo: Some(pair),
                    up_sent: false,
                    a_up: false,
                    b_up: false,
                });
                return vec![Action::InjectDown(pair)];
            }
            // Not the configured partner: the first key becomes a single
            // in press order, and the new key starts its own hold.
            let mut out = Vec::new();
            self.resolve_held_as_single(held, &mut out);
            self.held = Some(Held {
                key,
                deadline: now + self.cfg.interval_ms,
            });
            return out;
        }
        self.held = Some(Held {
            key,
            deadline: now + self.cfg.interval_ms,
        });
        Vec::new()
    }

    pub fn key_up(&mut self, key: u8, _now: u64) -> Vec<Action> {
        if self.held.as_ref().is_some_and(|h| h.key == key) {
            self.held = None;
            // Released before the interval expired: a tap fires at once.
            return match self.cfg.single(key) {
                Some(combo) => vec![Action::InjectDown(combo), Action::InjectUp(combo)],
                None => Vec::new(),
            };
        }
        let Some(pos) = self.active.iter().position(|act| act.contains(key)) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let act = &mut self.active[pos];
        if act.a == key {
            act.a_up = true;
        } else {
            act.b_up = true;
        }
        if !act.up_sent
            && let Some(combo) = act.combo
        {
            out.push(Action::InjectUp(combo));
            act.up_sent = true;
        }
        if act.a_up && act.b_up {
            self.active.remove(pos);
        }
        out
    }

    /// Advance time; a hold that expires becomes its single.
    pub fn tick(&mut self, now: u64) -> Vec<Action> {
        let Some(held) = self.held.take_if(|h| now >= h.deadline) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        self.resolve_held_as_single(held, &mut out);
        out
    }
}

/// Modifier bit → Linux evdev key code, in press order.
const MOD_EVDEV: [(u8, u16); 8] = [
    (0x01, 29),  // LCtrl
    (0x02, 42),  // LShift
    (0x04, 56),  // LAlt
    (0x08, 125), // LWin
    (0x10, 97),  // RCtrl
    (0x20, 54),  // RShift
    (0x40, 100), // RAlt
    (0x80, 126), // RWin
];

/// Modifier bit → Windows virtual-key code, in press order.
const MOD_VK: [(u8, u16); 8] = [
    (0x01, 0xA2),
    (0x02, 0xA0),
    (0x04, 0xA4),
    (0x08, 0x5B),
    (0x10, 0xA3),
    (0x20, 0xA1),
    (0x40, 0xA5),
    (0x80, 0x5C),
];

/// Evdev key codes that press `combo`, modifiers first, main key last.
/// Empty when the combo is empty or the usage has no evdev mapping.
pub fn evdev_codes(combo: Combo) -> Vec<u16> {
    let mut out = Vec::new();
    for (bit, code) in MOD_EVDEV {
        if combo.mods & bit != 0 {
            out.push(code);
        }
    }
    if combo.usage != 0
        && let Some(code) = tables::hid_usage_to_evdev().get(&u16::from(combo.usage))
    {
        out.push(*code);
    }
    out
}

/// Windows virtual-key codes that press `combo`, modifiers first, main
/// key last. Derived from the shared tables so both platforms agree.
pub fn vk_codes(combo: Combo) -> Vec<u16> {
    let mut out = Vec::new();
    for (bit, vk) in MOD_VK {
        if combo.mods & bit != 0 {
            out.push(vk);
        }
    }
    if combo.usage != 0
        && let Some(evdev) = tables::hid_usage_to_evdev().get(&u16::from(combo.usage))
        && let Some(vk) = tables::vk_to_evdev()
            .iter()
            .find(|(_, code)| *code == evdev)
            .map(|(vk, _)| *vk)
    {
        out.push(vk);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SINGLE_A: Combo = Combo {
        mods: 0x01,
        usage: 4, // Ctrl+A
    };
    const SINGLE_B: Combo = Combo {
        mods: 0,
        usage: 5, // B
    };
    const PAIR_AB: Combo = Combo {
        mods: 0x01,
        usage: 6, // Ctrl+C
    };

    fn cfg(enabled: bool) -> PairChordConfig {
        let mut cfg = PairChordConfig {
            enabled,
            interval_ms: 100,
            ..Default::default()
        };
        cfg.singles.insert(1, SINGLE_A);
        cfg.singles.insert(2, SINGLE_B);
        cfg.pairs.insert((1, 2), PAIR_AB);
        cfg
    }

    #[test]
    fn sentinel_shape() {
        assert_eq!(SENTINEL_MODS, 0x03);
        assert_eq!(
            sentinel_combo(1),
            Combo {
                mods: 0x03,
                usage: 58
            }
        );
        assert_eq!(
            sentinel_combo(6),
            Combo {
                mods: 0x03,
                usage: 63
            }
        );
        assert_eq!(keynum_for_sentinel_usage(58), Some(1));
        assert_eq!(keynum_for_sentinel_usage(63), Some(6));
        assert_eq!(keynum_for_sentinel_usage(64), None);
    }

    #[test]
    fn pair_names_are_canonical() {
        assert_eq!(pair_name(1, 2).as_deref(), Some("1+2"));
        assert_eq!(pair_name(2, 1).as_deref(), Some("1+2"));
        assert_eq!(pair_name(1, 4).as_deref(), Some("4+1"));
        assert_eq!(pair_name(4, 1).as_deref(), Some("4+1"));
        assert_eq!(pair_name(3, 6).as_deref(), Some("6+3"));
        assert_eq!(pair_name(1, 3), None);
        assert_eq!(PAIRS.len(), 7);
    }

    #[test]
    fn settings_round_trip() {
        let mut settings = Settings::new();
        set_enabled(&mut settings, true);
        set_pair(&mut settings, 4, 1, Some(PAIR_AB));
        set_single(&mut settings, 3, Some(SINGLE_B));
        assert_eq!(
            settings["chords"]["pairs"]["4+1"],
            json!({ "mods": 1, "usage": 6 })
        );
        let parsed = PairChordConfig::from_settings(&settings);
        assert!(parsed.enabled);
        assert_eq!(parsed.interval_ms, DEFAULT_INTERVAL_MS);
        assert_eq!(parsed.pair_combo(1, 4), Some(PAIR_AB));
        assert_eq!(parsed.single(3), Some(SINGLE_B));
        assert!(parsed.participates(4));
        assert!(!parsed.participates(2));
        set_pair(&mut settings, 1, 4, None);
        set_single(&mut settings, 3, None);
        let parsed = PairChordConfig::from_settings(&settings);
        assert_eq!(parsed.pair_combo(1, 4), None);
        assert_eq!(parsed.single(3), None);
    }

    #[test]
    fn pair_within_interval_fires_chord_only() {
        let mut engine = PairChordEngine::new(cfg(true));
        assert_eq!(engine.key_down(1, 0), vec![]);
        assert_eq!(
            engine.key_down(2, 40),
            vec![Action::InjectDown(PAIR_AB)],
            "the pair fires, neither single does"
        );
        assert_eq!(engine.tick(500), vec![]);
        assert_eq!(engine.key_up(1, 600), vec![Action::InjectUp(PAIR_AB)]);
        assert_eq!(engine.key_up(2, 610), vec![]);
        assert!(engine.is_idle());
    }

    #[test]
    fn timeout_fires_singles_in_press_order() {
        let mut engine = PairChordEngine::new(cfg(true));
        assert_eq!(engine.key_down(1, 0), vec![]);
        assert_eq!(engine.tick(99), vec![]);
        assert_eq!(
            engine.tick(100),
            vec![Action::InjectDown(SINGLE_A)],
            "the held key resolves to its single at the deadline"
        );
        assert_eq!(engine.key_down(2, 150), vec![]);
        assert_eq!(engine.tick(250), vec![Action::InjectDown(SINGLE_B)]);
        assert_eq!(engine.key_up(1, 300), vec![Action::InjectUp(SINGLE_A)]);
        assert_eq!(engine.key_up(2, 310), vec![Action::InjectUp(SINGLE_B)]);
        assert!(engine.is_idle());
    }

    #[test]
    fn early_release_is_a_tap() {
        let mut engine = PairChordEngine::new(cfg(true));
        assert_eq!(engine.key_down(1, 0), vec![]);
        assert_eq!(
            engine.key_up(1, 30),
            vec![Action::InjectDown(SINGLE_A), Action::InjectUp(SINGLE_A)]
        );
        assert_eq!(engine.tick(200), vec![]);
        assert!(engine.is_idle());
    }

    #[test]
    fn non_partner_second_key_gives_both_singles() {
        let mut engine = PairChordEngine::new(cfg(true));
        assert_eq!(engine.key_down(1, 0), vec![]);
        // Key 3 is not key 1's configured partner (and has no single).
        assert_eq!(
            engine.key_down(3, 40),
            vec![Action::InjectDown(SINGLE_A)],
            "the first key flushes as its single, in press order"
        );
        assert_eq!(engine.tick(140), vec![], "key 3 has no single to fire");
        assert_eq!(engine.key_up(3, 150), vec![]);
        assert_eq!(engine.key_up(1, 160), vec![Action::InjectUp(SINGLE_A)]);
        assert!(engine.is_idle());
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut engine = PairChordEngine::new(cfg(false));
        assert_eq!(
            engine.key_down(1, 0),
            vec![Action::InjectDown(SINGLE_A)],
            "no hold while disabled"
        );
        assert_eq!(engine.key_up(1, 10), vec![Action::InjectUp(SINGLE_A)]);
        assert!(engine.is_idle());
    }

    #[test]
    fn repeats_and_stray_ups_are_ignored() {
        let mut engine = PairChordEngine::new(cfg(true));
        assert_eq!(engine.key_up(4, 0), vec![]);
        assert_eq!(engine.key_down(1, 0), vec![]);
        assert_eq!(engine.key_down(1, 10), vec![], "repeat while held");
        assert_eq!(engine.key_down(2, 20), vec![Action::InjectDown(PAIR_AB)]);
        assert_eq!(engine.key_down(2, 30), vec![], "repeat while active");
        assert_eq!(engine.key_down(1, 30), vec![], "repeat while active");
        assert_eq!(engine.key_up(2, 40), vec![Action::InjectUp(PAIR_AB)]);
        assert_eq!(engine.key_up(1, 50), vec![]);
        assert!(engine.is_idle());
    }

    #[test]
    fn pair_fires_without_singles() {
        let mut bare = PairChordConfig {
            enabled: true,
            interval_ms: 100,
            ..Default::default()
        };
        bare.pairs.insert((4, 1), PAIR_AB);
        let mut engine = PairChordEngine::new(bare);
        assert_eq!(engine.key_down(4, 0), vec![]);
        assert_eq!(engine.key_down(1, 10), vec![Action::InjectDown(PAIR_AB)]);
        assert_eq!(engine.key_up(4, 20), vec![Action::InjectUp(PAIR_AB)]);
        assert_eq!(engine.key_up(1, 30), vec![]);
        // A lone press of an unassigned participating key fires nothing.
        assert_eq!(engine.key_down(1, 100), vec![]);
        assert_eq!(engine.tick(200), vec![]);
        assert_eq!(engine.key_up(1, 210), vec![]);
        assert!(engine.is_idle());
    }

    #[test]
    fn shutdown_releases_injected_keys() {
        let mut engine = PairChordEngine::new(cfg(true));
        engine.key_down(1, 0);
        engine.tick(100);
        assert_eq!(engine.shutdown(), vec![Action::InjectUp(SINGLE_A)]);
        assert!(engine.is_idle());
        assert_eq!(engine.key_up(1, 200), vec![]);
    }

    #[test]
    fn code_maps_agree_with_tables() {
        // The sentinel Ctrl+Shift+F1 as evdev codes: LCtrl, LShift, F1.
        assert_eq!(evdev_codes(sentinel_combo(1)), vec![29, 42, 59]);
        assert_eq!(vk_codes(sentinel_combo(1)), vec![0xA2, 0xA0, 0x70]);
        assert_eq!(evdev_codes(SINGLE_A), vec![29, 30]); // Ctrl + A
        assert_eq!(vk_codes(SINGLE_A), vec![0xA2, 0x41]);
        assert_eq!(evdev_codes(Combo::default()), Vec::<u16>::new());
    }
}
