//! egui window. Tabs: Keys, Modifiers, Media, Mouse, LED, Dial, Help.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::VERSION;
use crate::app_presets;
use crate::button_config::{self, ButtonAssignment};
use crate::device::{DeviceError, T1Device};
use crate::edition;
use crate::gui::fonts;
use crate::gui::i18n::{self, tr};
use crate::keys::{self, AppendStatus, MODIFIERS, PHYS_KEYS, T1_KEY_COUNT, USAGE_KEYS};
use crate::pair_chord;
use crate::paths;
use crate::resident;
use crate::settings::{self, Settings};
use crate::state;

pub fn run_gui() -> i32 {
    let instance = match super::instance::claim(&paths::state_dir()) {
        super::instance::Claim::AlreadyRunning => {
            if !super::instance::focus_existing(&paths::state_dir()) {
                eprintln!("{} is already running", edition::PRODUCT_NAME_EN);
            }
            return 0;
        }
        super::instance::Claim::Primary(instance) => instance,
    };
    let window_height = if std::env::var_os("T1_SHOT_DIR").is_some() {
        1280.0
    } else {
        700.0
    };
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([980.0, window_height])
        .with_min_inner_size([760.0, 520.0])
        .with_title(edition::PRODUCT_NAME_JA);
    if let Some(icon) = load_rgba(&icon_file("app")) {
        viewport = viewport.with_icon(egui::IconData {
            rgba: icon.0,
            width: icon.1,
            height: icon.2,
        });
    }
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if std::env::var("T1_RENDERER").ok().as_deref() == Some("glow") {
        options.renderer = eframe::Renderer::Glow;
    }
    // eframe app id (window class / storage key), not shown as a title.
    match eframe::run_native(
        "T1-Customizer",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, instance)))),
    ) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("GUI failed: {err}");
            if let Some(dir) = std::env::var_os("T1_SHOT_DIR") {
                let path = PathBuf::from(dir).join("gui-error.txt");
                let _ = std::fs::write(path, format!("{err}\n"));
            }
            1
        }
    }
}

struct Learn {
    rx: Receiver<LearnHit>,
    until: Instant,
}

enum LearnHit {
    Code(u16, u16),
    NoDevice,
    Timeout,
}

#[derive(Clone)]
enum Written {
    Keys { keys: Vec<(u8, u8)>, lone: u8 },
    Media(String),
    Mouse(String),
}

#[derive(Clone)]
struct KeySnap {
    written: Written,
    desc: String,
}

#[derive(Clone)]
struct UndoWrite {
    key: u8,
    layer: u8,
    snap: KeySnap,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirm {
    DisableKey,
}

struct App {
    settings: Settings,
    device: T1Device,
    lang: String,
    layer: u8,
    page: usize,
    selected: Option<u8>,
    toast: String,
    key_sel: Option<usize>,
    media_sel: Option<usize>,
    mouse_sel: Option<usize>,
    led_sel: u8,
    mod_on: [bool; 8],
    base_label: String,
    groups: Vec<(u8, u8)>,
    combo_name: String,
    combo_sel: Option<usize>,
    pair_sel: Option<usize>,
    learn: Option<Learn>,
    tray_note: bool,
    allow_close: bool,
    shot_dir: Option<PathBuf>,
    shot_single: Option<PathBuf>,
    shot_phase: usize,
    shot_wait: u32,
    connected: bool,
    connect_tried: bool,
    unlocked: bool,
    double_click_write: bool,
    zoom: f32,
    confirm: Option<Confirm>,
    status: String,
    undo: Option<UndoWrite>,
    last_written: HashMap<(u8, u8), KeySnap>,
    tray: Option<tray_icon::TrayIcon>,
    menu_open: tray_icon::menu::MenuItem,
    menu_toggle: tray_icon::menu::MenuItem,
    menu_quit: tray_icon::menu::MenuItem,
    instance: super::instance::GuiInstance,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, instance: super::instance::GuiInstance) -> Self {
        fonts::install(&cc.egui_ctx);
        install_visuals(&cc.egui_ctx);
        let settings = settings::load_settings();
        let lang = i18n::ui_lang(settings.get("lang").and_then(|v| v.as_str())).to_string();
        let layer = settings
            .get("layer")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .clamp(1, 3) as u8;
        let unlocked = true;
        let zoom = settings
            .get("ui_zoom")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(0.8, 2.0) as f32;
        let double_click_write = settings
            .get("double_click_write")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let capture = std::env::var_os("T1_SHOT_DIR").is_some();
        let (tray, menu_open, menu_toggle, menu_quit) = build_tray(unlocked || capture, &lang);
        let mut app = Self {
            settings,
            device: T1Device::new(),
            lang,
            layer,
            page: std::env::var("T1_GUI_TAB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            selected: None,
            toast: String::new(),
            key_sel: None,
            media_sel: None,
            mouse_sel: None,
            led_sel: 0,
            mod_on: [false; 8],
            base_label: String::new(),
            groups: Vec::new(),
            combo_name: String::new(),
            combo_sel: None,
            pair_sel: None,
            learn: None,
            tray_note: std::env::var_os("T1_SHOT_DIR").is_some()
                || std::env::var_os("T1_SHOT_TRAY").is_some(),
            allow_close: false,
            shot_dir: std::env::var_os("T1_SHOT_DIR").map(PathBuf::from),
            shot_single: std::env::var_os("T1_GUI_SHOT").map(PathBuf::from),
            shot_phase: 0,
            shot_wait: 0,
            connected: false,
            connect_tried: false,
            unlocked,
            double_click_write,
            zoom,
            confirm: None,
            status: String::new(),
            undo: None,
            last_written: HashMap::new(),
            tray,
            menu_open,
            menu_toggle,
            menu_quit,
            instance,
        };
        let ctx = cc.egui_ctx.clone();
        app.instance
            .set_waker(std::sync::Arc::new(move || ctx.request_repaint()));
        app.base_label = tr(&app.lang, "mod_only").to_string();
        if capture {
            app.selected = Some(1);
        }
        app
    }

    fn t(&self, key: &str) -> &'static str {
        tr(&self.lang, key)
    }

    /// JA / EN / KO at the top right. Switches at once and saves `lang`.
    /// Call inside a right-to-left layout; the buttons read left to right.
    fn lang_switch(&mut self, ui: &mut egui::Ui) {
        for lang in i18n::LANGS.iter().rev() {
            let selected = self.lang == *lang;
            if ui
                .add(egui::Button::selectable(selected, i18n::lang_button(lang)))
                .clicked()
                && !selected
            {
                self.lang = (*lang).to_string();
                self.settings.insert("lang".into(), json!(self.lang));
                self.base_label = self.t("mod_only").to_string();
                self.save();
            }
        }
    }

    fn save(&mut self) {
        let _ = settings::save_settings(&self.settings);
    }

    fn remember(&mut self, description: &str) {
        let Some(key) = self.selected else {
            return;
        };
        let assignments = settings::ensure_object(&mut self.settings, "assignments");
        if !assignments
            .get(&self.layer.to_string())
            .is_some_and(|v| v.is_object())
        {
            assignments.insert(self.layer.to_string(), Value::Object(Default::default()));
        }
        if let Some(layer) = assignments
            .get_mut(&self.layer.to_string())
            .and_then(|v| v.as_object_mut())
        {
            layer.insert(key.to_string(), Value::String(description.to_string()));
        }
        self.save();
    }

    /// True when the knob press is a key assignment (`dial.press_action`).
    fn press_is_key(&self) -> bool {
        crate::resident::press_action(&self.settings) == crate::resident::PressAction::Key
    }

    /// Slots the user may write: KEY1..KEY6 always, plus all six knob
    /// slots K1-L/C/R (13..15) and K2-L/C/R (16..18) while the knob is a
    /// key assignment. In cycle mode the resident listener owns the knob,
    /// so those slots stay read-only.
    fn key_slot_selectable(&self, num: u8) -> bool {
        num <= T1_KEY_COUNT || (self.press_is_key() && (13..=18).contains(&num))
    }

    fn require_key(&mut self) -> bool {
        if self.selected.is_none() {
            self.toast = self.t("pick_first").to_string();
            return false;
        }
        if self
            .selected
            .is_some_and(|key| !self.key_slot_selectable(key))
        {
            self.toast = self.t("gray_keys").to_string();
            return false;
        }
        if !self.device.is_open() {
            self.toast = self.t("need_device").to_string();
            return false;
        }
        true
    }

    fn write_current(&mut self) {
        match self.page {
            0 => self.write_basic(),
            1 => self.write_modifier(),
            2 | 3 => {
                let (table, media) = named_write_for_page(self.page);
                self.write_named(table, media);
            }
            4 => self.write_led(),
            _ => self.toast = self.t("pick_function").to_string(),
        }
    }

    fn write_basic(&mut self) {
        let Some(index) = self.key_sel else {
            self.toast = self.t("pick_function").to_string();
            return;
        };
        let (label, usage) = USAGE_KEYS[index];
        if let Some(pair) = self.pair_sel {
            // A pair is selected: the write assigns the pair in
            // settings only; nothing is burned for the pair itself.
            self.assign_pair(pair, pair_chord::Combo { mods: 0, usage });
            return;
        }
        if !self.require_key() {
            return;
        }
        let key = self.selected.unwrap();
        let ok = self.burn_key(key, &[(0, usage)], 0);
        self.finish_key_write(
            key,
            label,
            Written::Keys {
                keys: vec![(0, usage)],
                lone: 0,
            },
            ok,
        );
    }

    fn page_keys(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(self.t("app_presets"));
            ui.horizontal_wrapped(|ui| {
                for preset in &app_presets::APP_PRESETS {
                    if ui.button(preset.name).clicked() {
                        self.apply_app_preset(preset);
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.label(self.t("chords_mode_label"));
            let on = pair_chord::chords_enabled(&self.settings);
            ui.horizontal(|ui| {
                if ui.selectable_label(on, self.t("chords_on")).clicked() && !on {
                    self.set_chords_enabled(true);
                }
                if ui.selectable_label(!on, self.t("chords_off")).clicked() && on {
                    self.set_chords_enabled(false);
                }
            });
            ui.label(self.t("chords_note"));
            if on {
                let cfg = pair_chord::PairChordConfig::from_settings(&self.settings);
                for (index, (a, b)) in pair_chord::PAIRS.iter().enumerate() {
                    let desc = cfg
                        .pair_combo(*a, *b)
                        .map(|c| keys::describe_mod_usage(c.mods, c.usage))
                        .unwrap_or_else(|| "-".to_string());
                    let caption = format!("KEY{a}+KEY{b}  {desc}");
                    if ui
                        .selectable_label(self.pair_sel == Some(index), caption)
                        .clicked()
                    {
                        self.pair_sel = if self.pair_sel == Some(index) {
                            None
                        } else {
                            Some(index)
                        };
                    }
                }
                if self.pair_sel.is_some() {
                    ui.label(self.t("chords_assign_hint"));
                }
            }
        });
        ui.add_space(4.0);
        self.page_list(ui, USAGE_KEYS.iter().map(|(n, _)| *n).collect(), true);
    }

    fn pair_label(index: usize) -> String {
        let (a, b) = pair_chord::PAIRS[index];
        format!("KEY{a}+KEY{b}")
    }

    /// Assign a combo to the selected pair (settings only), then burn
    /// sentinels for both keys whose single is already known. A key
    /// without a stored single is named in the toast: assigning its
    /// single first is what burns its sentinel.
    fn assign_pair(&mut self, index: usize, combo: pair_chord::Combo) {
        let (a, b) = pair_chord::PAIRS[index];
        if combo.is_empty() {
            self.toast = self.t("mod_need_combo").to_string();
            return;
        }
        pair_chord::set_pair(&mut self.settings, a, b, Some(combo));
        self.save();
        let cfg = pair_chord::PairChordConfig::from_settings(&self.settings);
        let mut missing = Vec::new();
        let mut pending = false;
        for key in [a, b] {
            match cfg.single(key) {
                Some(_) => {
                    if self.device.is_open() {
                        self.burn_sentinel(key);
                    } else {
                        pending = true;
                    }
                }
                None => missing.push(key),
            }
        }
        if !missing.is_empty() {
            let names = missing
                .iter()
                .map(|k| format!("KEY{k}"))
                .collect::<Vec<_>>()
                .join(", ");
            self.toast = self.t("chords_single_first").replace("{keys}", &names);
        } else if pending {
            self.toast = self.t("chords_burn_pending").to_string();
        } else {
            let desc = keys::describe_mod_usage(combo.mods, combo.usage);
            let name = pair_chord::pair_name(a, b).unwrap_or_default();
            self.toast = self
                .t("chords_pair_saved")
                .replace("{pair}", &name)
                .replace("{desc}", &desc);
        }
    }

    fn burn_sentinel(&mut self, key: u8) {
        let sentinel = pair_chord::sentinel_combo(key);
        self.device.write_key(
            key,
            self.layer,
            &[(sentinel.mods, sentinel.usage)],
            sentinel.mods,
        );
    }

    /// The master pair-mode toggle. Turning on burns sentinels (current
    /// preset only) for participating keys whose single is stored;
    /// turning off burns the stored singles back. Without a connected
    /// device the burn cannot happen and the toast says so plainly.
    fn set_chords_enabled(&mut self, on: bool) {
        pair_chord::set_enabled(&mut self.settings, on);
        self.save();
        self.pair_sel = None;
        let cfg = pair_chord::PairChordConfig::from_settings(&self.settings);
        if on {
            let mut missing = Vec::new();
            let mut pending = false;
            for key in 1..=6u8 {
                if !cfg.participates(key) {
                    continue;
                }
                match cfg.single(key) {
                    Some(_) => {
                        if self.device.is_open() {
                            self.burn_sentinel(key);
                        } else {
                            pending = true;
                        }
                    }
                    None => missing.push(key),
                }
            }
            if !missing.is_empty() {
                let names = missing
                    .iter()
                    .map(|k| format!("KEY{k}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.toast = self.t("chords_single_first").replace("{keys}", &names);
            } else if pending {
                self.toast = self.t("chords_burn_pending").to_string();
            } else {
                self.toast = self.t("chords_on_done").to_string();
            }
        } else {
            let mut pending = false;
            for key in 1..=6u8 {
                if let Some(single) = cfg.single(key) {
                    if self.device.is_open() {
                        self.burn_raw_single(key, single);
                    } else {
                        pending = true;
                    }
                }
            }
            self.toast = if pending {
                self.t("chords_restore_pending").to_string()
            } else {
                self.t("chords_off_done").to_string()
            };
        }
    }

    fn burn_raw_single(&mut self, key: u8, single: pair_chord::Combo) {
        if single.usage != 0 {
            self.device
                .write_key(key, self.layer, &[(single.mods, single.usage)], 0);
        } else {
            self.device.write_key(key, self.layer, &[], single.mods);
        }
    }

    /// Write a key combo, honouring pair mode. While pair mode is on, a
    /// KEY1..KEY6 write stores the real combo in `chords.singles` and
    /// burns the key's sentinel instead when the key takes part in a
    /// configured pair (the real combo when it does not). A multi-group
    /// combo cannot be a pair-mode single and is refused.
    fn burn_key(&mut self, key: u8, groups: &[(u8, u8)], lone: u8) -> bool {
        if !pair_chord::chords_enabled(&self.settings) || !(1..=6).contains(&key) {
            return self.device.write_key(key, self.layer, groups, lone);
        }
        let single = match (groups, lone) {
            ([(mods, usage)], _) => Some(pair_chord::Combo {
                mods: *mods,
                usage: *usage,
            }),
            ([], 0) => None,
            ([], mods) => Some(pair_chord::Combo { mods, usage: 0 }),
            _ => {
                self.toast = self.t("chords_single_only").to_string();
                return false;
            }
        };
        let cfg = pair_chord::PairChordConfig::from_settings(&self.settings);
        let ok = if cfg.participates(key) {
            let sentinel = pair_chord::sentinel_combo(key);
            self.device.write_key(
                key,
                self.layer,
                &[(sentinel.mods, sentinel.usage)],
                sentinel.mods,
            )
        } else {
            self.device.write_key(key, self.layer, groups, lone)
        };
        if ok {
            pair_chord::set_single(&mut self.settings, key, single);
            self.save();
        }
        ok
    }

    fn apply_app_preset(&mut self, preset: &app_presets::AppPreset) {
        if !self.device.is_open() {
            self.toast = self.t("need_device").to_string();
            return;
        }
        let mut written = 0usize;
        for (index, preset_key) in preset.keys.iter().enumerate() {
            let keynum = (index + 1) as u8;
            let combo = [(preset_key.mods, preset_key.usage)];
            if !self.burn_key(keynum, &combo, 0) {
                continue;
            }
            written += 1;
            self.selected = Some(keynum);
            let desc = app_presets::description(preset_key);
            self.finish_key_write(
                keynum,
                &desc,
                Written::Keys {
                    keys: combo.to_vec(),
                    lone: 0,
                },
                true,
            );
        }
        // A preset may also carry a knob rotation pair (CLIP STUDIO
        // PAINT). It goes to K1-L / K1-R, and only in key assignment
        // mode: in cycle mode the resident owns the knob.
        let mut total = preset.keys.len();
        let mut knob_note: Option<&str> = None;
        if let Some(knob) = &preset.knob {
            if self.press_is_key() {
                total += 2;
                for (keynum, preset_key) in [(13u8, &knob.left), (15, &knob.right)] {
                    let combo = [(preset_key.mods, preset_key.usage)];
                    if !self.device.write_key(keynum, self.layer, &combo, 0) {
                        continue;
                    }
                    written += 1;
                    self.selected = Some(keynum);
                    let desc = app_presets::description(preset_key);
                    self.finish_key_write(
                        keynum,
                        &desc,
                        Written::Keys {
                            keys: combo.to_vec(),
                            lone: 0,
                        },
                        true,
                    );
                }
                if written == total {
                    knob_note = Some("app_preset_knob");
                }
            } else {
                knob_note = Some("app_preset_knob_skip");
            }
        }
        if written == total && total > 0 {
            self.status = self.t("app_preset_applied").to_string();
            if let Some(note) = knob_note {
                self.status = format!("{} {}", self.status, self.t(note));
            }
            self.toast = self.status.clone();
        } else if written > 0 {
            let line = self
                .t("app_preset_partial")
                .replace("{ok}", &written.to_string());
            self.status = line.clone();
            self.toast = line;
        } else {
            let line = self.ng_line();
            self.note_result(line, false);
        }
    }

    fn staged_mods(&self) -> u8 {
        let mut mods = 0u8;
        for (index, (_, mask)) in MODIFIERS.iter().enumerate() {
            if self.mod_on[index] {
                mods |= mask;
            }
        }
        mods
    }

    fn write_modifier(&mut self) {
        let usage = keys::usage_for_label(&self.base_label);
        let (keys, lone) = keys::editor_key_combo(&self.groups, self.staged_mods(), usage);
        if let Some(pair) = self.pair_sel {
            // A pair takes a single (mods, usage): the first staged
            // group, or the modifiers alone.
            let combo = keys
                .first()
                .map(|(mods, usage)| pair_chord::Combo {
                    mods: *mods,
                    usage: *usage,
                })
                .or(if lone != 0 {
                    Some(pair_chord::Combo {
                        mods: lone,
                        usage: 0,
                    })
                } else {
                    None
                });
            match combo {
                Some(combo) => self.assign_pair(pair, combo),
                None => self.toast = self.t("mod_need_combo").to_string(),
            }
            return;
        }
        if !self.require_key() {
            return;
        }
        if keys.is_empty() && lone == 0 {
            self.toast = self.t("mod_need_combo").to_string();
            return;
        }
        let key = self.selected.unwrap();
        let desc = keys::describe_key_combo(&keys, lone);
        let ok = self.burn_key(key, &keys, lone);
        self.finish_key_write(
            key,
            &desc,
            Written::Keys {
                keys: keys.clone(),
                lone,
            },
            ok,
        );
    }

    fn write_named(&mut self, table: &[(&str, &str)], media: bool) {
        if pair_chord::chords_enabled(&self.settings)
            && self.selected.is_some_and(|key| (1..=6).contains(&key))
        {
            // A media/mouse burn would replace the sentinel and break
            // pair detection for that key (and fire immediately, which
            // is exactly the mis-input pair mode exists to prevent).
            self.toast = self.t("chords_keys_only").to_string();
            return;
        }
        let index = if media {
            self.media_sel
        } else {
            self.mouse_sel
        };
        let Some(index) = index else {
            self.toast = self.t("pick_function").to_string();
            return;
        };
        if !self.require_key() {
            return;
        }
        let Some((action, label)) = named_write_parts(table, index) else {
            self.toast = self.t("pick_function").to_string();
            return;
        };
        let key = self.selected.unwrap();
        let ok = if media {
            self.device.write_media(key, self.layer, action)
        } else {
            self.device.write_mouse(key, self.layer, action)
        };
        let written = if media {
            Written::Media(action.to_string())
        } else {
            Written::Mouse(action.to_string())
        };
        self.finish_key_write(key, label, written, ok);
    }

    fn write_led(&mut self) {
        if !self.device.is_open() {
            self.toast = self.t("need_device").to_string();
            return;
        }
        let ok = self.device.write_led(self.layer, self.led_sel);
        let label = keys::LED_MODES
            .iter()
            .find(|(mode, _)| *mode == self.led_sel)
            .map(|(_, name)| *name)
            .unwrap_or("");
        self.note_result(i18n::wrote_line(&self.lang, "LED", label), ok);
    }

    fn disable_key(&mut self) {
        if !self.require_key() {
            return;
        }
        let key = self.selected.unwrap();
        let ok = self.burn_key(key, &[], 0);
        let empty = self.t("assign_empty").to_string();
        self.finish_key_write(
            key,
            &empty,
            Written::Keys {
                keys: Vec::new(),
                lone: 0,
            },
            ok,
        );
        if ok {
            self.undo = None;
        }
    }

    /// Mirror a successful write into `settings["buttons"]` so the
    /// configuration can be saved to a file and restored later. The
    /// recorded value is the real assignment (in pair mode the device
    /// carries the sentinel, but the single is what a restore burns).
    fn record_button(&mut self, layer: u8, key: u8, written: &Written) {
        let assignment = match written {
            Written::Keys { keys, lone } => ButtonAssignment::Key {
                groups: keys.clone(),
                lone: *lone,
            },
            Written::Media(name) => ButtonAssignment::Media(name.clone()),
            Written::Mouse(name) => ButtonAssignment::Mouse(name.clone()),
        };
        button_config::record(&mut self.settings, layer, key, &assignment);
        self.save();
    }

    /// The write-failure line, with the device's recorded failure
    /// detail (operation, stage, transport error) appended when there
    /// is one — a bare "failed" gives the user nothing to report.
    /// The app log's location is appended too: every assignment write
    /// is logged there, and "the log file does not exist" must never
    /// be the end of a bug report again.
    fn ng_line(&mut self) -> String {
        let log = paths::log_file().display().to_string();
        match self.device.last_failure() {
            Some(detail) => format!("{} ({detail}) [log: {log}]", self.t("download_ng")),
            None => format!("{} [log: {log}]", self.t("download_ng")),
        }
    }

    fn finish_key_write(&mut self, key: u8, label: &str, written: Written, ok: bool) {
        let target = format!("KEY{key}");
        if !ok {
            let line = self.ng_line();
            self.note_result(line, false);
            return;
        }
        let prev = self.last_written.get(&(self.layer, key)).cloned();
        self.remember(label);
        self.record_button(self.layer, key, &written);
        self.last_written.insert(
            (self.layer, key),
            KeySnap {
                written,
                desc: label.to_string(),
            },
        );
        self.undo = prev.map(|snap| UndoWrite {
            key,
            layer: self.layer,
            snap,
        });
        self.note_result(i18n::wrote_line(&self.lang, &target, label), true);
    }

    fn note_result(&mut self, line: String, ok: bool) {
        if ok {
            self.status = line.clone();
            self.toast = line;
        } else {
            self.status = line.clone();
            self.toast = line;
            self.undo = None;
        }
    }

    fn restore_undo(&mut self) {
        let Some(undo) = self.undo.clone() else {
            return;
        };
        if !self.device.is_open() {
            self.toast = self.t("need_device").to_string();
            return;
        }
        if pair_chord::chords_enabled(&self.settings)
            && (1..=6).contains(&undo.key)
            && !matches!(undo.snap.written, Written::Keys { .. })
        {
            self.toast = self.t("chords_keys_only").to_string();
            return;
        }
        self.layer = undo.layer;
        let ok = match &undo.snap.written {
            Written::Keys { keys, lone } => self.burn_key(undo.key, keys, *lone),
            Written::Media(action) => self.device.write_media(undo.key, undo.layer, action),
            Written::Mouse(action) => self.device.write_mouse(undo.key, undo.layer, action),
        };
        if !ok {
            self.note_result(self.t("download_ng").to_string(), false);
            return;
        }
        self.layer = undo.layer;
        self.selected = Some(undo.key);
        self.remember(&undo.snap.desc);
        self.record_button(undo.layer, undo.key, &undo.snap.written);
        self.last_written
            .insert((undo.layer, undo.key), undo.snap.clone());
        self.undo = None;
        self.note_result(
            i18n::wrote_line(&self.lang, &format!("KEY{}", undo.key), &undo.snap.desc),
            true,
        );
    }

    /// Save the whole button configuration (recorded buttons, pair
    /// chords, dial settings, descriptions) to a JSON file the user
    /// picks.
    fn export_button_config(&mut self) {
        if !button_config::has_exportable(&self.settings) {
            self.toast = self.t("config_save_empty").to_string();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.t("config_save"))
            .add_filter("JSON", &["json"])
            .set_file_name("t1-button-config.json")
            .save_file()
        else {
            return;
        };
        let doc = button_config::build_export(&self.settings);
        let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
        match std::fs::write(&path, text) {
            Ok(()) => {
                self.toast = self
                    .t("config_saved")
                    .replace("{path}", &path.display().to_string());
            }
            Err(_) => {
                self.toast = self.t("config_save_failed").to_string();
            }
        }
    }

    /// Load a configuration file saved by [`Self::export_button_config`].
    /// The file is validated before anything changes; on success the
    /// imported sections replace the current ones and every recorded
    /// assignment is written back to the device through the usual burn
    /// paths. Without a connected device the settings are loaded and
    /// the toast says the device write is still pending.
    fn import_button_config(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.t("config_load"))
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                self.toast = format!("{} ({err})", self.t("config_load_bad"));
                return;
            }
        };
        let imported = match button_config::parse_import(&text) {
            Ok(imported) => imported,
            Err(code) => {
                self.toast = format!("{} ({code})", self.t("config_load_bad"));
                return;
            }
        };
        let skipped_invalid = imported.skipped_entries;
        button_config::apply_import(&mut self.settings, &imported);
        self.pair_sel = None;
        self.save();
        if !self.device.is_open() {
            self.toast = self.t("config_loaded_pending").to_string();
            return;
        }
        let plan = button_config::write_plan(
            &self.settings,
            self.press_is_key(),
            pair_chord::chords_enabled(&self.settings),
        );
        let saved_layer = self.layer;
        let (mut ok, mut failed) = (0usize, 0usize);
        for write in &plan.writes {
            self.layer = write.layer;
            let good = match &write.assignment {
                ButtonAssignment::Key { groups, lone } => self.burn_key(write.key, groups, *lone),
                ButtonAssignment::Media(name) => {
                    self.device.write_media(write.key, write.layer, name)
                }
                ButtonAssignment::Mouse(name) => {
                    self.device.write_mouse(write.key, write.layer, name)
                }
            };
            if good {
                ok += 1;
            } else {
                failed += 1;
            }
        }
        self.layer = saved_layer;
        let skipped = plan.skipped.len() + skipped_invalid;
        let line = self
            .t("config_loaded_counts")
            .replace("{ok}", &ok.to_string())
            .replace("{failed}", &failed.to_string())
            .replace("{skipped}", &skipped.to_string());
        self.status = line.clone();
        self.toast = line;
    }

    fn assignment_text(&self) -> String {
        let Some(key) = self.selected else {
            return self.t("no_key").to_string();
        };
        self.settings
            .get("assignments")
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(&self.layer.to_string()))
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(&key.to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn poll_learn(&mut self) {
        let Some(learn) = self.learn.as_ref() else {
            return;
        };
        if let Ok(result) = learn.rx.try_recv() {
            self.learn = None;
            match result {
                LearnHit::Code(kind, code) => {
                    self.toast = format!("{} {code}", self.t("learn_done"));
                    self.apply_learned(kind, code);
                }
                LearnHit::NoDevice => self.toast = self.t("learn_no_device").to_string(),
                LearnHit::Timeout => self.toast = self.t("learn_timeout").to_string(),
            }
            return;
        }
        let left = learn.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            if Instant::now() > learn.until + Duration::from_millis(800) {
                self.learn = None;
                self.toast = self.t("learn_timeout").to_string();
            }
            return;
        }
        let seconds = left.as_secs() + 1;
        self.toast = i18n::learn_countdown(&self.lang, seconds);
    }

    fn apply_learned(&mut self, kind: u16, code: u16) {
        let dial = settings::ensure_object(&mut self.settings, "dial");
        if kind == crate::tables::EV_REL {
            let mut codes = dial
                .get("rotate_codes")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if !codes.iter().any(|v| v.as_i64() == Some(i64::from(code))) {
                codes.push(json!(code));
            }
            dial.insert("rotate_codes".into(), Value::Array(codes));
        } else if self.page == 5 {
            dial.insert("push_code".into(), json!(code));
        }
        self.save();
    }

    fn start_learn(&mut self, want_rel: bool) {
        self.toast = self.t("learning").to_string();
        let (tx, rx) = mpsc::channel();
        thread_learn(tx, want_rel);
        self.learn = Some(Learn {
            rx,
            until: Instant::now() + Duration::from_secs(5),
        });
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let receiver = tray_icon::menu::MenuEvent::receiver();
        while let Ok(event) = receiver.try_recv() {
            match event.id.0.as_str() {
                "open" => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                "toggle" => self.toggle_monitor(),
                "quit" => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                _ => {}
            }
        }
        let running = state::monitor_is_running();
        let label = if running {
            self.t("tray_stop")
        } else {
            self.t("tray_start")
        };
        self.menu_open.set_text(self.t("tray_open"));
        self.menu_quit.set_text(self.t("tray_quit"));
        self.menu_toggle.set_text(label);
    }

    fn toggle_monitor(&mut self) {
        if state::monitor_is_running() {
            if resident::stop_monitor_process() {
                self.toast = self.t("monitor_stopped").to_string();
            }
        } else if let Err(err) = resident::start_monitor_process() {
            self.toast = format!("{} ({err})", self.t("monitor_error"));
        } else {
            self.toast = self.t("monitor_running").to_string();
        }
    }

    fn request_shots(&mut self, ctx: &egui::Context) {
        let active = self.shot_dir.is_some() || self.shot_single.is_some();
        if !active {
            return;
        }
        self.shot_wait += 1;
        if self.shot_wait < 12 {
            ctx.request_repaint();
            return;
        }
        if self.shot_dir.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                self.page,
            )));
            self.shot_wait = 0;
            ctx.request_repaint();
            return;
        }
        if self.shot_phase < 7 {
            self.page = self.shot_phase;
        } else {
            self.tray_note = true;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
            self.shot_phase,
        )));
        self.shot_wait = 0;
        ctx.request_repaint();
    }

    fn store_shot(&mut self, ctx: &egui::Context, image: &egui::ColorImage, phase: usize) {
        let name = if phase >= 7 {
            "tray.png".to_string()
        } else {
            format!("tab-{phase}.png")
        };
        let path = if let Some(dir) = &self.shot_dir {
            let _ = std::fs::create_dir_all(dir);
            dir.join(name)
        } else if let Some(path) = &self.shot_single {
            path.clone()
        } else {
            return;
        };
        save_png(&path, image);
        if self.shot_dir.is_some() {
            self.shot_phase = phase + 1;
            self.shot_wait = 0;
            if self.shot_phase > 7 {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint();
            }
        } else {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.instance.take_raise() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.unlocked {
            self.poll_learn();
            self.poll_tray(ctx);
            self.refresh_tray_tooltip();
            ctx.request_repaint_after(Duration::from_millis(400));
            if !self.connect_tried || !self.connected {
                self.connect_tried = true;
                match self.device.connect() {
                    Ok(ok) => self.connected = ok,
                    Err(DeviceError { .. }) => self.connected = false,
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.tray.is_some() && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot {
                image, user_data, ..
            } = event
            {
                let phase = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<usize>().copied())
                    .unwrap_or(self.page);
                self.store_shot(ctx, &image, phase);
            }
        }
        self.request_shots(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_zoom_factor(self.zoom);
        self.paint(ui);
    }
}

impl App {
    fn paint(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("header").show(ui, |ui| {
            ui.horizontal(|ui| {
                let (color, label) = if self.connected {
                    (egui::Color32::from_rgb(40, 160, 60), self.t("connected"))
                } else {
                    (egui::Color32::from_rgb(200, 40, 40), self.t("disconnected"))
                };
                ui.colored_label(color, "●");
                ui.label(label);
                if let Some(err) = self.device.last_error()
                    && !self.connected
                {
                    ui.label(err);
                }
                ui.label(self.t("layer"))
                    .on_hover_text(self.t("layer_hint"));
                for layer in 1..=3 {
                    let response = ui.add_enabled(
                        edition::t1_features_allowed(),
                        egui::Button::selectable(self.layer == layer, layer.to_string()),
                    );
                    if response.clicked() {
                        self.layer = layer;
                        self.settings.insert("layer".into(), json!(layer));
                        self.save();
                    }
                    response.on_hover_text(self.t("layer_hint"));
                }
                if ui
                    .button("?")
                    .on_hover_text(self.t("help_button"))
                    .clicked()
                {
                    self.page = 6;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.lang_switch(ui);
                    ui.label(&self.toast);
                });
            });
            ui.label(self.t("layer_hint"))
                .on_hover_text(self.t("layer_hint"));
        });
        if !self.connected
            && let Some(code) = self.device.last_error()
        {
            let text = i18n::connect_fault_text(&self.lang, &code);
            egui::Panel::top("connect-fault").show(ui, |ui| {
                warn_block(ui, &self.lang, &text);
                if ui.button(self.t("reconnect")).clicked() {
                    self.device.close();
                    self.connected = false;
                    self.connect_tried = false;
                }
            });
        }

        egui::Panel::left("keys")
            .resizable(true)
            .min_size(280.0)
            .default_size(400.0)
            .max_size(720.0)
            .show(ui, |ui| {
                ui.heading(format!(
                    "{}: {}",
                    self.t("selected"),
                    self.selected
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| self.t("no_key").into())
                ));
                ui.columns(3, |cols| {
                    for (index, (name, num)) in PHYS_KEYS.iter().enumerate() {
                        let ui = &mut cols[index % 3];
                        ui.horizontal(|ui| {
                            if self.key_slot_selectable(*num) {
                                let selected = self.selected == Some(*num);
                                if ui.selectable_label(selected, *name).clicked() {
                                    self.selected = Some(*num);
                                    self.pair_sel = None;
                                }
                            } else {
                                ui.add_enabled_ui(false, |ui| {
                                    let _ = ui.selectable_label(
                                        false,
                                        egui::RichText::new(*name)
                                            .color(egui::Color32::from_rgb(150, 150, 150)),
                                    );
                                });
                            }
                        });
                        let _ = index;
                    }
                });
                ui.separator();
                ui.label(self.t("current"));
                let text = self.assignment_text();
                if text.is_empty() {
                    ui.label(self.t("none_paren"));
                } else {
                    ui.label(&text);
                }
                let (write_label, write_tip, can_write) = self.write_offer();
                let write = ui
                    .add_enabled(can_write, egui::Button::new(write_label))
                    .on_hover_text(write_tip);
                if write.clicked() {
                    self.write_current();
                }
                if ui.button(self.t("clear")).clicked() {
                    self.selected = None;
                    self.toast.clear();
                    self.status.clear();
                }
                let double_click_label = self.t("double_click_write");
                if ui
                    .checkbox(&mut self.double_click_write, double_click_label)
                    .changed()
                {
                    self.settings
                        .insert("double_click_write".into(), json!(self.double_click_write));
                    self.save();
                }
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(self.t("disable_key")).color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(0xA3, 0x2A, 0x2A)),
                    )
                    .clicked()
                {
                    self.confirm = Some(Confirm::DisableKey);
                }
                if !self.status.is_empty() {
                    ui.label(&self.status);
                }
                if self.undo.is_some()
                    && ui
                        .button(self.t("write_undo"))
                        .on_hover_text(self.t("write_undo_tip"))
                        .clicked()
                {
                    self.restore_undo();
                }
                ui.add_space(8.0);
                if ui.button(self.t("config_save")).clicked() {
                    self.export_button_config();
                }
                if ui.button(self.t("config_load")).clicked() {
                    self.import_button_config();
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, label) in i18n::pages(&self.lang).into_iter().enumerate() {
                    let caption = marked(self.page == index, label);
                    if ui.selectable_label(self.page == index, caption).clicked() {
                        self.page = index;
                    }
                }
            });
            ui.separator();
            match self.page {
                0 => self.page_keys(ui),
                1 => self.page_modifiers(ui),
                2 => self.page_actions(ui, keys::MEDIA_ACTIONS, true),
                3 => self.page_actions(ui, keys::MOUSE_ACTIONS, false),
                4 => self.page_led(ui),
                5 => self.page_dial(ui),
                _ => self.page_help(ui),
            }
            ui.separator();
            self.show_tray_strip(ui);
            if self.tray_note {
                ui.group(|ui| {
                    ui.heading(self.t("tray_title"));
                    ui.label(self.t("tray_open"));
                    ui.label(if state::monitor_is_running() {
                        self.t("tray_stop")
                    } else {
                        self.t("tray_start")
                    });
                    ui.label(self.t("tray_quit"));
                });
            }
        });
        self.paint_confirm(&ctx);
        ctx.request_repaint_after(Duration::from_millis(400));
    }

    fn write_offer(&self) -> (String, String, bool) {
        match self.page {
            0 => {
                if let Some(pair) = self.pair_sel {
                    return match self.key_sel {
                        Some(index) => (
                            i18n::write_button(
                                &self.lang,
                                &Self::pair_label(pair),
                                USAGE_KEYS[index].0,
                            ),
                            String::new(),
                            true,
                        ),
                        None => (
                            self.t("download").to_string(),
                            self.t("pick_function").to_string(),
                            false,
                        ),
                    };
                }
                match (self.selected, self.key_sel) {
                    (Some(key), Some(index)) => (
                        i18n::write_button(&self.lang, &format!("KEY{key}"), USAGE_KEYS[index].0),
                        String::new(),
                        true,
                    ),
                    (None, _) => (
                        self.t("download").to_string(),
                        self.t("pick_first").to_string(),
                        false,
                    ),
                    _ => (
                        self.t("download").to_string(),
                        self.t("pick_function").to_string(),
                        false,
                    ),
                }
            }
            1 => {
                if self.pair_sel.is_none() && self.selected.is_none() {
                    return (
                        self.t("download").to_string(),
                        self.t("pick_first").to_string(),
                        false,
                    );
                }
                let usage = keys::usage_for_label(&self.base_label);
                let (keys, lone) = keys::editor_key_combo(&self.groups, self.staged_mods(), usage);
                if keys.is_empty() && lone == 0 {
                    return (
                        self.t("download").to_string(),
                        self.t("write_need_combo").to_string(),
                        false,
                    );
                }
                let desc = keys::describe_key_combo(&keys, lone);
                let target = match self.pair_sel {
                    Some(pair) => Self::pair_label(pair),
                    None => format!("KEY{}", self.selected.unwrap()),
                };
                (
                    i18n::write_button(&self.lang, &target, &desc),
                    String::new(),
                    true,
                )
            }
            2 => self.named_offer(keys::MEDIA_ACTIONS, self.media_sel),
            3 => self.named_offer(keys::MOUSE_ACTIONS, self.mouse_sel),
            4 => {
                let label = keys::LED_MODES
                    .iter()
                    .find(|(mode, _)| *mode == self.led_sel)
                    .map(|(_, name)| *name)
                    .unwrap_or("");
                (
                    i18n::write_button(&self.lang, "LED", label),
                    String::new(),
                    true,
                )
            }
            _ => (
                self.t("download").to_string(),
                self.t("write_unavailable").to_string(),
                false,
            ),
        }
    }

    fn named_offer(
        &self,
        table: &[(&str, &str)],
        selected: Option<usize>,
    ) -> (String, String, bool) {
        let Some(key) = self.selected else {
            return (
                self.t("download").to_string(),
                self.t("pick_first").to_string(),
                false,
            );
        };
        let Some(index) = selected else {
            return (
                self.t("download").to_string(),
                self.t("pick_function").to_string(),
                false,
            );
        };
        (
            i18n::write_button(&self.lang, &format!("KEY{key}"), table[index].0),
            String::new(),
            true,
        )
    }

    fn paint_confirm(&mut self, ctx: &egui::Context) {
        let Some(confirm) = self.confirm else {
            return;
        };
        let title = match confirm {
            Confirm::DisableKey => self.t("disable_title").to_string(),
        };
        let mut keep = true;
        egui::Modal::new(egui::Id::new("t1-confirm")).show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.heading(title);
            ui.add_space(6.0);
            match confirm {
                Confirm::DisableKey => {
                    let Some(key) = self.selected else {
                        ui.label(self.t("pick_first"));
                        if ui.button(self.t("confirm_cancel")).clicked() {
                            keep = false;
                        }
                        return;
                    };
                    ui.label(i18n::disable_body(&self.lang, key, self.layer));
                    ui.horizontal(|ui| {
                        if ui.button(self.t("disable_key")).clicked() {
                            self.disable_key();
                            keep = false;
                        }
                        if ui.button(self.t("confirm_cancel")).clicked() {
                            keep = false;
                        }
                    });
                }
            }
        });
        if !keep {
            self.confirm = None;
        }
    }
}

impl App {
    fn page_list(&mut self, ui: &mut egui::Ui, labels: Vec<&str>, keys: bool) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, label) in labels.iter().enumerate() {
                let selected = self.key_sel == Some(index);
                let caption = marked(selected, (*label).to_string());
                let response = ui.selectable_label(selected, caption);
                if response.clicked() {
                    self.key_sel = Some(index);
                }
                if response.double_clicked() && keys && self.double_click_write {
                    self.key_sel = Some(index);
                    self.write_basic();
                }
            }
        });
    }

    fn page_modifiers(&mut self, ui: &mut egui::Ui) {
        ui.label(self.t("mod_groups"));
        ui.horizontal_wrapped(|ui| {
            for (index, (name, _)) in MODIFIERS.iter().enumerate() {
                ui.checkbox(&mut self.mod_on[index], *name);
            }
        });
        ui.label(self.t("mod_base"));
        egui::ComboBox::from_id_salt("base")
            .selected_text(&self.base_label)
            .show_ui(ui, |ui| {
                let only_label = self.t("mod_only");
                let only = only_label.to_string();
                ui.selectable_value(&mut self.base_label, only, only_label);
                for (name, _) in USAGE_KEYS {
                    ui.selectable_value(&mut self.base_label, (*name).to_string(), *name);
                }
            });
        ui.horizontal(|ui| {
            if ui.button(self.t("mod_add")).clicked() {
                let usage = keys::usage_for_label(&self.base_label);
                if usage == 0 {
                    self.toast = self.t("mod_need_base").to_string();
                } else {
                    let (next, status) =
                        keys::append_key_group(&self.groups, self.staged_mods(), usage);
                    self.groups = next;
                    if matches!(status, AppendStatus::Full) {
                        self.toast = self.t("mod_group_full").to_string();
                    }
                }
            }
            if ui.button(self.t("mod_reset")).clicked() {
                self.groups.clear();
                self.mod_on = [false; 8];
                self.base_label = self.t("mod_only").to_string();
            }
        });
        for (mods, usage) in &self.groups {
            ui.label(keys::describe_mod_usage(*mods, *usage));
        }
        let (preview_keys, lone) = keys::editor_key_combo(
            &self.groups,
            self.staged_mods(),
            keys::usage_for_label(&self.base_label),
        );
        ui.label(keys::describe_key_combo(&preview_keys, lone));
        let _ = lone;
        ui.horizontal(|ui| {
            ui.label(self.t("mod_name"));
            ui.text_edit_singleline(&mut self.combo_name);
            if ui.button(self.t("mod_save")).clicked() {
                if self.combo_name.trim().is_empty() {
                    self.toast = self.t("mod_need_name").to_string();
                } else if preview_keys.is_empty() && lone == 0 {
                    self.toast = self.t("mod_need_combo").to_string();
                } else {
                    let _ = keys::upsert_modifier_combo(
                        &mut self.settings,
                        &self.combo_name,
                        &preview_keys,
                        lone,
                    );
                    self.save();
                    self.toast = self.t("mod_saved_ok").to_string();
                }
            }
            if ui.button(self.t("mod_delete")).clicked() {
                if self.combo_name.trim().is_empty() {
                    self.toast = self.t("mod_pick_saved").to_string();
                } else {
                    keys::delete_modifier_combo(&mut self.settings, &self.combo_name);
                    self.save();
                    self.toast = self.t("mod_deleted").to_string();
                }
            }
        });
        ui.label(self.t("mod_saved"));
        let combos = keys::parse_modifier_combos(&self.settings);
        for (index, combo) in combos.iter().enumerate() {
            let desc = keys::describe_key_combo(&combo.keys, combo.lone_mod);
            if ui
                .selectable_label(
                    self.combo_sel == Some(index),
                    format!("{}  {desc}", combo.name),
                )
                .clicked()
            {
                self.combo_sel = Some(index);
                self.combo_name = combo.name.clone();
                self.groups = combo.keys.clone();
            }
        }
    }

    fn page_actions(&mut self, ui: &mut egui::Ui, table: &[(&str, &str)], media: bool) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, (label, _)) in table.iter().enumerate() {
                let selected = if media {
                    self.media_sel == Some(index)
                } else {
                    self.mouse_sel == Some(index)
                };
                let caption = marked(selected, (*label).to_string());
                let response = ui.selectable_label(selected, caption);
                if response.clicked() {
                    if media {
                        self.media_sel = Some(index);
                    } else {
                        self.mouse_sel = Some(index);
                    }
                }
                if response.double_clicked() && self.double_click_write {
                    if media {
                        self.media_sel = Some(index);
                    } else {
                        self.mouse_sel = Some(index);
                    }
                    self.write_named(table, media);
                }
            }
        });
    }

    fn page_led(&mut self, ui: &mut egui::Ui) {
        for (mode, label) in keys::LED_MODES {
            let selected = self.led_sel == *mode;
            let caption = marked(selected, (*label).to_string());
            let response = ui.selectable_label(selected, caption);
            if response.clicked() {
                self.led_sel = *mode;
            }
            if response.double_clicked() && self.double_click_write {
                self.led_sel = *mode;
                self.write_led();
            }
        }
    }

    fn show_tray_strip(&self, ui: &mut egui::Ui) {
        ui.label(if state::monitor_is_running() {
            self.t("monitor_running")
        } else {
            self.t("monitor_stopped")
        });
    }

    fn refresh_tray_tooltip(&self) {
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(edition::PRODUCT_NAME_JA.to_string()));
        }
    }

    fn page_dial(&mut self, ui: &mut egui::Ui) {
        ui.label(i18n::dial_hint(&self.lang));
        ui.separator();
        ui.label(self.t("dial_press_label"));
        let press = crate::resident::press_action(&self.settings);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    press == crate::resident::PressAction::Cycle,
                    self.t("dial_press_cycle"),
                )
                .clicked()
                && press != crate::resident::PressAction::Cycle
            {
                let dial = settings::ensure_object(&mut self.settings, "dial");
                dial.insert("press_action".into(), json!("cycle"));
                if self.selected.is_some_and(|num| num > T1_KEY_COUNT) {
                    self.selected = None;
                }
                self.save();
            }
            if ui
                .selectable_label(
                    press == crate::resident::PressAction::Key,
                    self.t("dial_press_key"),
                )
                .clicked()
                && press != crate::resident::PressAction::Key
            {
                let dial = settings::ensure_object(&mut self.settings, "dial");
                dial.insert("press_action".into(), json!("key"));
                self.save();
            }
        });
        ui.label(self.t("dial_press_note"));
        ui.label(self.t("dial_preset_note"));
        ui.separator();
        let current = crate::resident::dial_modes(&self.settings);
        let mut modes = [
            current.iter().any(|m| m == "mic"),
            current.iter().any(|m| m == "headphone"),
        ];
        let labels = [
            ("mic", self.t("dial_mode_mic")),
            ("headphone", self.t("dial_mode_headphone")),
        ];
        let mut changed = false;
        for (index, (_, label)) in labels.iter().enumerate() {
            if ui.checkbox(&mut modes[index], *label).changed() {
                changed = true;
            }
        }
        if changed {
            let selected: Vec<_> = labels
                .iter()
                .enumerate()
                .filter(|(i, _)| modes[*i])
                .map(|(_, (id, _))| json!(*id))
                .collect();
            let dial = settings::ensure_object(&mut self.settings, "dial");
            dial.insert(
                "modes".into(),
                Value::Array(if selected.is_empty() {
                    vec![json!("mic")]
                } else {
                    selected
                }),
            );
            self.save();
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(self.t("learn_rotate")).clicked() {
                self.start_learn(true);
            }
            if ui.button(self.t("learn_push")).clicked() {
                self.start_learn(false);
            }
        });
        ui.label(if state::monitor_is_running() {
            self.t("monitor_running")
        } else {
            self.t("monitor_stopped")
        });
        if ui
            .button(if state::monitor_is_running() {
                self.t("stop_monitor")
            } else {
                self.t("start_monitor")
            })
            .clicked()
        {
            self.toggle_monitor();
        }
    }

    fn page_help(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("help_heading"));
        ui.label(i18n::help_text(&self.lang));
        ui.add_space(8.0);
        ui.label(self.t("zoom"));
        let mut percent = (self.zoom * 100.0).round() as i32;
        if ui
            .add(egui::Slider::new(&mut percent, 80..=200).suffix("%"))
            .changed()
        {
            self.zoom = (percent as f32 / 100.0).clamp(0.8, 2.0);
            self.settings.insert("ui_zoom".into(), json!(self.zoom));
            self.save();
        }
        ui.separator();
        ui.label(i18n::about_text(
            &self.lang,
            VERSION,
            &paths::config_file().display().to_string(),
        ));
        ui.separator();
        ui.label(self.t("font_credit"));
        ui.label(format!(
            "{}: {}",
            self.t("font_in_use"),
            fonts::active_source()
        ));
        ui.collapsing(self.t("font_license"), |ui| {
            ui.label(fonts::ofl_text());
        });
    }
}

fn thread_learn(tx: mpsc::Sender<LearnHit>, want_rel: bool) {
    std::thread::spawn(move || {
        let Some(mut source) = crate::input::open_input_source() else {
            let _ = tx.send(LearnHit::NoDevice);
            return;
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            for event in source.wait_events(0.2) {
                if event.value != 1 {
                    continue;
                }
                if want_rel && event.etype == crate::tables::EV_REL {
                    let _ = tx.send(LearnHit::Code(event.etype, event.code));
                    return;
                }
                if !want_rel && event.etype == crate::tables::EV_KEY {
                    let _ = tx.send(LearnHit::Code(event.etype, event.code));
                    return;
                }
            }
        }
        let _ = tx.send(LearnHit::Timeout);
    });
}

fn install_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    let edge = egui::Color32::from_rgb(0x7A, 0x7A, 0x7A);
    let stroke = egui::Stroke::new(1.0, edge);
    visuals.widgets.inactive.bg_stroke = stroke;
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.5, egui::Color32::from_rgb(0xC8, 0xC8, 0xC8));
    visuals.widgets.active.bg_stroke = stroke;
    visuals.widgets.open.bg_stroke = stroke;
    visuals.selection.stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
    visuals.selection.bg_fill = egui::Color32::from_rgb(0x3A, 0x9E, 0xC8);
    visuals.extreme_bg_color = egui::Color32::from_rgb(0x2A, 0x2A, 0x2A);
    ctx.set_visuals(visuals);
}

fn marked(selected: bool, label: impl Into<String>) -> String {
    let label = label.into();
    if selected {
        format!("● {label}")
    } else {
        label
    }
}

fn warn_block(ui: &mut egui::Ui, lang: &str, text: &str) {
    let color = egui::Color32::from_rgb(0xFF, 0x6B, 0x6B);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, color))
        .inner_margin(8)
        .corner_radius(4)
        .show(ui, |ui| {
            ui.colored_label(
                color,
                format!("※ {}: {text}", i18n::tr(lang, "warn_prefix")),
            );
        });
}

fn build_tray(
    enabled: bool,
    lang: &str,
) -> (
    Option<tray_icon::TrayIcon>,
    tray_icon::menu::MenuItem,
    tray_icon::menu::MenuItem,
    tray_icon::menu::MenuItem,
) {
    use tray_icon::TrayIconBuilder;
    use tray_icon::menu::{Menu, MenuItem};
    #[cfg(target_os = "linux")]
    {
        let _ = gtk::init();
    }
    // Rust `&str` is Unicode. muda encodes it as UTF-16 on Windows
    // (`OsStr::encode_wide` into `AppendMenuW`), not as the ANSI code page.
    let open = MenuItem::with_id("open", tr(lang, "tray_open"), true, None);
    let toggle = MenuItem::with_id("toggle", tr(lang, "tray_start"), true, None);
    let quit = MenuItem::with_id("quit", tr(lang, "tray_quit"), true, None);
    if !enabled {
        return (None, open, toggle, quit);
    }
    let menu = Menu::new();
    let icon = load_tray_icon();
    let tray = if menu.append_items(&[&open, &toggle, &quit]).is_ok() {
        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(edition::PRODUCT_NAME_JA);
        if let Some(icon) = icon {
            builder = builder.with_icon(icon);
        }
        builder.build().ok()
    } else {
        None
    };
    (tray, open, toggle, quit)
}

fn icon_file(stem: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::var_os("T1_ICON_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("icons"));
        candidates.push(dir.join("../share/icons/hicolor/256x256/apps"));
    }
    candidates.push(PathBuf::from("icons"));
    candidates.push(PathBuf::from("/workspace/icons"));
    for dir in candidates {
        for name in [
            format!("{stem}-256.png"),
            format!("{stem}.png"),
            format!("{stem}-48.png"),
            "app-icon.png".into(),
            "tray-256.png".into(),
        ] {
            let path = dir.join(name);
            if path.is_file() {
                return path;
            }
        }
    }
    PathBuf::from("icons/app-icon.png")
}

fn load_rgba(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    let image = image::open(path).ok()?.to_rgba8();
    let (w, h) = image.dimensions();
    Some((image.into_raw(), w, h))
}

/// Named-action write dispatch for the Media (page 2) and Mouse
/// (page 3) pages: the action table plus the `media` flag for
/// `write_named`. The Mouse page must pass `media: false` — with
/// `true` the write reads the media selection instead of the mouse
/// selection and encodes through the media table (whose actions do
/// not include any mouse action), so mouse writes always failed.
fn named_write_for_page(page: usize) -> (&'static [(&'static str, &'static str)], bool) {
    match page {
        2 => (keys::MEDIA_ACTIONS, true),
        _ => (keys::MOUSE_ACTIONS, false),
    }
}

/// Split one row of a media/mouse action table into the value the
/// device encoder needs and the label shown to the user. Rows are
/// `(internal_name, display_label)` and the encoder
/// (`protocol::media_bytes` / `protocol::mouse_payload`) accepts ONLY
/// the internal name. `write_named` once destructured the row in the
/// wrong order and handed the display label ("Wheel Up") to the
/// encoder, so every media/mouse write through the Write button —
/// knob slots included — failed with "unknown action" before any
/// device traffic.
fn named_write_parts<'a>(
    table: &'a [(&'a str, &'a str)],
    index: usize,
) -> Option<(&'a str, &'a str)> {
    table.get(index).map(|(name, label)| (*name, *label))
}

fn load_tray_icon() -> Option<tray_icon::Icon> {
    let (rgba, w, h) = load_rgba(&icon_file("tray"))?;
    tray_icon::Icon::from_rgba(rgba, w, h).ok()
}

fn save_png(path: &Path, image: &egui::ColorImage) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let buffer = image::RgbaImage::from_raw(
        image.width() as u32,
        image.height() as u32,
        image.as_raw().to_vec(),
    );
    if let Some(buffer) = buffer {
        let _ = buffer.save(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_write_pages_use_their_own_table_and_path() {
        let (table, media) = named_write_for_page(2);
        assert!(media);
        assert_eq!(table, keys::MEDIA_ACTIONS);

        // Regression: the Mouse page once dispatched with
        // `media: true`, so its write button read the media
        // selection and encoded mouse actions through the media
        // path — mouse writes always failed, knob slots included.
        let (table, media) = named_write_for_page(3);
        assert!(!media);
        assert_eq!(table, keys::MOUSE_ACTIONS);
    }

    #[test]
    fn named_write_passes_the_internal_name_to_the_encoder() {
        // Regression: `write_named` destructured a table row as
        // `(label, action)`, so the encoder received the DISPLAY
        // label ("Wheel Up") and every media/mouse write failed with
        // "unknown action". Drive the exact accessor the GUI uses —
        // not just the table's own name field — into the encoders.
        for (index, (name, label)) in keys::MEDIA_ACTIONS.iter().enumerate() {
            let (action, shown) = named_write_parts(keys::MEDIA_ACTIONS, index).unwrap();
            assert_eq!(action, *name);
            assert_eq!(shown, *label);
            for report_id in [0u8, 1, 2, 3] {
                assert!(
                    crate::protocol::media_bytes(report_id, action).is_some(),
                    "media action {action} must encode (report_id {report_id})"
                );
                assert!(
                    crate::protocol::media_bytes(report_id, shown).is_none(),
                    "display label {shown} must NOT reach the media encoder"
                );
            }
        }
        for (index, (name, label)) in keys::MOUSE_ACTIONS.iter().enumerate() {
            let (action, shown) = named_write_parts(keys::MOUSE_ACTIONS, index).unwrap();
            assert_eq!(action, *name);
            assert_eq!(shown, *label);
            assert!(
                crate::protocol::mouse_payload(action).is_some(),
                "mouse action {action} must encode"
            );
            assert!(
                crate::protocol::mouse_payload(shown).is_none(),
                "display label {shown} must NOT reach the mouse encoder"
            );
        }
        assert!(named_write_parts(keys::MOUSE_ACTIONS, usize::MAX).is_none());
    }
}
