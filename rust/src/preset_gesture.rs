//! Hold-and-twist preset switching for the dial, and the preset →
//! LED-effect mapping.
//!
//! The gesture decision lives here, away from the resident's event
//! loop, so it can be unit-tested: the resident feeds knob press and
//! rotation events in and gets preset steps and tap notifications
//! out. One rotation event is one step, matching the granularity the
//! volume handling already uses (direction only, magnitude ignored).
//!
//! Why LED *effects* and not the colours this feature was requested
//! with (preset 1 red, 2 blue, 3 green): the vendor protocol has no
//! colour command at all. The LED report (`protocol::led_report`) is
//! `[0xB0, (layer << 4) | 8, mode]` — a firmware effect preset is the
//! only thing it carries, and the effect is stored per layer on the
//! device. The byte-verified reconstruction of the vendor app
//! (jgt87/Macropad `PROTOCOL.md`, "LED backlight") states it
//! outright: "There is no colour/brightness/per-LED command." The
//! firmware effects are 0 = off, 1 = one LED steady (fixed white),
//! 2 = colour-cycle chase across all LEDs, 3 = pulse/breathing
//! frozen on the cycle's current colour. Arbitrary or per-key RGB
//! would require custom firmware. So each preset gets a distinct
//! effect instead of a colour: preset N → effect N (1 steady,
//! 2 colour-cycle chase, 3 breathing).

use serde_json::Value;

use crate::settings::Settings;

pub const PRESET_COUNT: u8 = 3;

/// The app's current preset (1..=3): the persisted `layer` setting
/// the GUI's preset tabs also read and write, so the gesture, the
/// GUI and the device all agree on one value.
pub fn active_preset(settings: &Settings) -> u8 {
    settings
        .get("layer")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(1, i64::from(PRESET_COUNT)) as u8
}

/// LED effect mode written for a preset: preset N → firmware effect
/// N. See the module docs for why these are effects, not the
/// requested red/blue/green.
pub fn preset_led_mode(preset: u8) -> u8 {
    preset.clamp(1, PRESET_COUNT)
}

/// Short name of a preset's LED effect, for logs.
pub fn preset_led_effect(preset: u8) -> &'static str {
    match preset_led_mode(preset) {
        1 => "steady",
        2 => "colour-cycle chase",
        _ => "breathing",
    }
}

/// Outcome of releasing the knob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// Not held (no matching press was seen); nothing to do.
    NotHeld,
    /// Held and twisted: the press was consumed by preset steps and
    /// must not also cycle the volume mode.
    Twisted,
    /// Held and released without any twist: a plain tap, which keeps
    /// its historic meaning (cycle the volume mode).
    Tap,
}

/// Hold-and-twist state for one dial. The resident's input model has
/// a single dial (one learned push code, one set of rotate codes), so
/// a single tracker covers every knob the OS reports that way.
#[derive(Clone, Debug)]
pub struct PresetGesture {
    preset: u8,
    held: bool,
    twisted: bool,
}

impl PresetGesture {
    pub fn new(preset: u8) -> Self {
        Self {
            preset: preset.clamp(1, PRESET_COUNT),
            held: false,
            twisted: false,
        }
    }

    pub fn preset(&self) -> u8 {
        self.preset
    }

    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Re-sync the tracked preset from settings (the GUI preset tabs
    /// write the same `layer` value). Held/twisted state survives, so
    /// a settings reload mid-hold does not lose the gesture.
    pub fn sync_preset(&mut self, preset: u8) {
        self.preset = preset.clamp(1, PRESET_COUNT);
    }

    pub fn press_down(&mut self) {
        if !self.held {
            self.held = true;
            self.twisted = false;
        }
    }

    /// One rotation event while held: step the preset (positive →
    /// next, negative → previous, wrapping within 1..=3) and return
    /// the new preset. When not held (or the event carries no
    /// movement), returns None and changes nothing, so plain rotation
    /// keeps driving the volume.
    pub fn rotate(&mut self, value: i32) -> Option<u8> {
        if !self.held || value == 0 {
            return None;
        }
        self.twisted = true;
        self.preset = if value > 0 {
            self.preset % PRESET_COUNT + 1
        } else if self.preset <= 1 {
            PRESET_COUNT
        } else {
            self.preset - 1
        };
        Some(self.preset)
    }

    pub fn press_up(&mut self) -> Release {
        if !self.held {
            return Release::NotHeld;
        }
        self.held = false;
        if self.twisted {
            self.twisted = false;
            Release::Twisted
        } else {
            Release::Tap
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_tap_is_a_tap() {
        let mut gesture = PresetGesture::new(1);
        gesture.press_down();
        assert_eq!(gesture.press_up(), Release::Tap);
        assert_eq!(gesture.preset(), 1);
    }

    #[test]
    fn release_without_press_is_not_held() {
        let mut gesture = PresetGesture::new(2);
        assert_eq!(gesture.press_up(), Release::NotHeld);
    }

    #[test]
    fn twist_steps_and_consumes_the_press() {
        let mut gesture = PresetGesture::new(1);
        gesture.press_down();
        assert_eq!(gesture.rotate(1), Some(2));
        // The release after a twist must not read as a tap (which
        // would also cycle the volume mode).
        assert_eq!(gesture.press_up(), Release::Twisted);
        assert_eq!(gesture.preset(), 2);
    }

    #[test]
    fn rotation_without_hold_does_not_step() {
        let mut gesture = PresetGesture::new(1);
        assert_eq!(gesture.rotate(1), None);
        assert_eq!(gesture.rotate(-1), None);
        assert_eq!(gesture.preset(), 1);
        // And a later plain press is still a tap.
        gesture.press_down();
        assert_eq!(gesture.press_up(), Release::Tap);
    }

    #[test]
    fn steps_wrap_around_both_ends() {
        let mut gesture = PresetGesture::new(3);
        gesture.press_down();
        assert_eq!(gesture.rotate(1), Some(1), "3 + right wraps to 1");
        assert_eq!(gesture.rotate(-1), Some(3), "1 + left wraps to 3");
        assert_eq!(gesture.rotate(-1), Some(2));
        assert_eq!(gesture.press_up(), Release::Twisted);
    }

    #[test]
    fn each_detent_event_is_one_step() {
        let mut gesture = PresetGesture::new(1);
        gesture.press_down();
        assert_eq!(gesture.rotate(1), Some(2));
        assert_eq!(gesture.rotate(1), Some(3));
        assert_eq!(gesture.rotate(1), Some(1));
        assert_eq!(gesture.rotate(-1), Some(3));
        gesture.press_up();
        // A fresh hold starts un-twisted again.
        gesture.press_down();
        assert_eq!(gesture.press_up(), Release::Tap);
    }

    #[test]
    fn sync_preset_keeps_hold_state() {
        let mut gesture = PresetGesture::new(1);
        gesture.press_down();
        gesture.sync_preset(2);
        assert!(gesture.is_held());
        assert_eq!(gesture.rotate(1), Some(3));
        assert_eq!(gesture.press_up(), Release::Twisted);
    }

    #[test]
    fn active_preset_reads_the_layer_setting() {
        let mut settings = Settings::new();
        assert_eq!(active_preset(&settings), 1, "missing layer means 1");
        settings.insert("layer".into(), json!(2));
        assert_eq!(active_preset(&settings), 2);
        settings.insert("layer".into(), json!(9));
        assert_eq!(active_preset(&settings), 3, "clamped to the top preset");
        settings.insert("layer".into(), json!(0));
        assert_eq!(active_preset(&settings), 1, "clamped to the first preset");
    }

    #[test]
    fn preset_led_modes_are_the_firmware_effects() {
        assert_eq!(preset_led_mode(1), 1);
        assert_eq!(preset_led_mode(2), 2);
        assert_eq!(preset_led_mode(3), 3);
        assert_eq!(preset_led_effect(1), "steady");
        assert_eq!(preset_led_effect(2), "colour-cycle chase");
        assert_eq!(preset_led_effect(3), "breathing");
    }
}
