//! Japanese glyphs for egui.
//!
//! egui's built-in faces cover Latin, Cyrillic, and a few emoji. They do not
//! contain kana or kanji, and egui never asks the OS for a font, so Japanese
//! UI text is drawn as the replacement box on every platform until we install
//! a face ourselves.
//!
//! Search order when a face is installed:
//! Windows exact family names (name id 1, non-italic): `Yu Gothic UI` in
//! `YuGothM.ttc`, then `Yu Gothic UI` in `YuGothR.ttc`, then `Yu Gothic UI
//! Semilight` in `YuGothR.ttc`, then `Meiryo UI` in `meiryo.ttc`, then
//! `Meiryo` in `meiryo.ttc`; each is looked up under `%WINDIR%\Fonts` (normally
//! `C:\Windows\Fonts`) and then the per-user `%LOCALAPPDATA%\Microsoft\Windows\Fonts`.
//! macOS Hiragino Sans (`ヒラギノ角ゴシック`); Linux Noto Sans CJK / Source Han / IPA
//! paths, then `fc-list :lang=ja`. The bundled Noto Sans JP subset is always
//! installed last, so a machine with no system face, or a system face that is
//! missing a character, still draws the UI.
//!
//! The subset is Noto Sans JP Regular 2.004 (SIL Open Font License 1.1,
//! © 2014-2021 Adobe, Reserved Font Name "Source"), reduced to JIS X 0208
//! plus the punctuation and kana blocks the UI needs. See `OFL.txt`.
//!
//! Hangul comes from a second bundled face, a Noto Sans KR Regular 2.004
//! subset (same license and copyright, see `OFL.txt`): the 2,350 KS X 1001
//! syllables, the compatibility jamo, and every Hangul character in
//! `i18n_ko.rs` (`scripts/subset-noto-sans-kr.py`). It is added after the
//! Japanese faces and egui's own fonts, so it only draws what they cannot,
//! and Japanese text looks the same as before.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use egui::epaint::text::{self, FontData, FontFamily, FontId, Fonts, TextOptions};

const BUNDLED_GZ: &[u8] = include_bytes!("../../assets/fonts/NotoSansJP-Regular.subset.otf.gz");
const BUNDLED_KR_GZ: &[u8] = include_bytes!("../../assets/fonts/NotoSansKR-Regular.subset.otf.gz");

static ACTIVE_SOURCE: OnceLock<String> = OnceLock::new();

/// Tray labels and the profile-import dialog title. On Windows, `muda` sends
/// menu text through `OsStr::encode_wide` and `rfd` sends dialog text through
/// `str::encode_utf16`. Both are UTF-16, not the ANSI code page.
pub const NATIVE_LABELS: &[&str] = &[
    "画面を開く",
    "待受の開始",
    "待受の停止",
    "終了",
    "アバタープロファイルを読み込む",
];

struct LoadedFace {
    bytes: Vec<u8>,
    index: u32,
    label: String,
}

struct FontStack {
    system: Option<LoadedFace>,
    bundled: LoadedFace,
    /// Hangul fallback. A broken Korean face never blocks Japanese.
    korean: Option<LoadedFace>,
}

impl FontStack {
    fn load() -> Result<Self, String> {
        // `T1_FONT_BUNDLED_ONLY` skips Yu Gothic / Noto CJK / fc-list so a
        // check can prove the bundled subset draws the UI by itself.
        let system = if std::env::var_os("T1_FONT_BUNDLED_ONLY").is_some() {
            None
        } else {
            load_system_face()
        };
        Ok(Self {
            system,
            bundled: load_bundled()?,
            korean: load_bundled_kr().ok(),
        })
    }

    #[cfg(test)]
    fn bundled_only() -> Result<Self, String> {
        Ok(Self {
            system: None,
            bundled: load_bundled()?,
            korean: load_bundled_kr().ok(),
        })
    }

    /// Characters no face can draw: system face, bundled subset, or egui's
    /// built-in fonts. Whitespace and control characters are ignored.
    fn missing(&self, chars: impl IntoIterator<Item = char>) -> Vec<char> {
        let builtin = egui::FontDefinitions::default();
        let mut blobs: Vec<(&[u8], u32)> = Vec::new();
        if let Some(system) = &self.system {
            blobs.push((&system.bytes, system.index));
        }
        blobs.push((&self.bundled.bytes, self.bundled.index));
        if let Some(korean) = &self.korean {
            blobs.push((&korean.bytes, korean.index));
        }
        for data in builtin.font_data.values() {
            blobs.push((data.font.as_ref(), data.index));
        }
        let faces: Vec<ttf_parser::Face<'_>> = blobs
            .iter()
            .filter_map(|(bytes, index)| ttf_parser::Face::parse(bytes, *index).ok())
            .collect();
        let mut missing = Vec::new();
        for ch in chars {
            if ch.is_whitespace() || ch.is_control() || missing.contains(&ch) {
                continue;
            }
            if !faces.iter().any(|face| face.glyph_index(ch).is_some()) {
                missing.push(ch);
            }
        }
        missing
    }

    fn source_line(&self) -> String {
        let mut line = match &self.system {
            Some(system) => format!("system: {}; fallback: {}", system.label, self.bundled.label),
            None => format!("fallback: {}", self.bundled.label),
        };
        if let Some(korean) = &self.korean {
            line.push_str(&format!("; korean: {}", korean.label));
        }
        line
    }

    fn into_definitions(self) -> egui::FontDefinitions {
        let mut fonts = egui::FontDefinitions::default();
        let has_system = self.system.is_some();
        if let Some(system) = self.system {
            let mut data = FontData::from_owned(system.bytes);
            data.index = system.index;
            fonts
                .font_data
                .insert("t1-system-jp".to_string(), Arc::new(data));
            // First in Proportional: Latin and Japanese in one string share one
            // face, so "PC" or "LED" no longer sit on egui's Latin baseline.
            if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
                list.insert(0, "t1-system-jp".to_string());
            }
            if let Some(list) = fonts.families.get_mut(&FontFamily::Monospace) {
                let at = list.len().min(1);
                list.insert(at, "t1-system-jp".to_string());
            }
        }
        let mut data = FontData::from_owned(self.bundled.bytes);
        data.index = self.bundled.index;
        fonts
            .font_data
            .insert("t1-bundled-jp".to_string(), Arc::new(data));
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            if has_system {
                list.push("t1-bundled-jp".to_string());
            } else {
                list.insert(0, "t1-bundled-jp".to_string());
            }
        }
        if let Some(list) = fonts.families.get_mut(&FontFamily::Monospace) {
            list.push("t1-bundled-jp".to_string());
        }
        if let Some(korean) = self.korean {
            let mut data = FontData::from_owned(korean.bytes);
            data.index = korean.index;
            fonts
                .font_data
                .insert("t1-bundled-kr".to_string(), Arc::new(data));
            // Last: only Hangul, which no earlier face has, is drawn from it.
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(list) = fonts.families.get_mut(&family) {
                    list.push("t1-bundled-kr".to_string());
                }
            }
        }
        fonts
    }
}

pub fn install(ctx: &egui::Context) {
    let stack = match FontStack::load() {
        Ok(stack) => stack,
        Err(err) => {
            eprintln!("japanese font: {err}");
            return;
        }
    };
    let _ = ACTIVE_SOURCE.set(stack.source_line());
    ctx.set_fonts(stack.into_definitions());
}

pub fn active_source() -> String {
    ACTIVE_SOURCE
        .get()
        .cloned()
        .unwrap_or_else(|| "Noto Sans JP subset".to_string())
}

pub fn ofl_text() -> &'static str {
    include_str!("../../assets/fonts/OFL.txt")
}

/// Exit status for `T1_FONT_CHECK`. Writes `font-check.txt` next to `proof`
/// when that path is set, and a PNG of the Japanese UI strings.
pub fn font_check(proof: Option<&Path>) -> i32 {
    match run_font_check(proof) {
        Ok(report) => {
            println!("{report}");
            if let Some(path) = proof {
                let _ = write_note(path, &report);
            }
            0
        }
        Err(err) => {
            eprintln!("japanese-font: {err}");
            if let Some(path) = proof {
                let _ = write_note(path, &format!("FAIL\n{err}\n"));
            }
            1
        }
    }
}

fn write_note(proof: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = proof.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(proof.with_file_name("font-check.txt"), text)
}

fn run_font_check(proof: Option<&Path>) -> Result<String, String> {
    verify_native_text()?;
    let stack = FontStack::load()?;
    let source = stack.source_line();
    verify_ui_coverage(&stack)?;
    let defs = stack.into_definitions();
    if let Some(path) = proof {
        write_proof(path, &defs)?;
    }
    Ok(format!("japanese-font: ok source={source}"))
}

fn load_bundled() -> Result<LoadedFace, String> {
    let mut decoder = flate2::read::GzDecoder::new(BUNDLED_GZ);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut bytes).map_err(|err| err.to_string())?;
    let picked = pick_face(&bytes).ok_or("bundled Noto Sans JP subset has no Japanese face")?;
    Ok(LoadedFace {
        bytes,
        index: picked.index,
        label: format!("{} (bundled subset)", picked.label),
    })
}

/// Bundled Noto Sans KR subset (Hangul for the Korean UI).
fn load_bundled_kr() -> Result<LoadedFace, String> {
    let mut decoder = flate2::read::GzDecoder::new(BUNDLED_KR_GZ);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut bytes).map_err(|err| err.to_string())?;
    let face = ttf_parser::Face::parse(&bytes, 0).map_err(|err| err.to_string())?;
    if face.glyph_index('한').is_none() {
        return Err("bundled Noto Sans KR subset has no Hangul".into());
    }
    let label = format!("{} (bundled subset)", face_label(&face));
    Ok(LoadedFace {
        bytes,
        index: 0,
        label,
    })
}

fn load_system_face() -> Option<LoadedFace> {
    if std::env::consts::OS == "windows" {
        let dir = std::env::var_os("WINDIR")
            .map(|windir| PathBuf::from(windir).join("Fonts"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
        let user = std::env::var_os("LOCALAPPDATA").map(|root| {
            PathBuf::from(root)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts")
        });
        for (path, family) in windows_faces(&dir, user.as_deref()) {
            if !path.is_file() {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Some(picked) = pick_named_face(&bytes, family) else {
                continue;
            };
            return Some(LoadedFace {
                bytes,
                index: picked.index,
                label: format!("{} ({})", picked.label, path.display()),
            });
        }
        return None;
    }
    for path in runtime_candidates() {
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Some(picked) = pick_face(&bytes) else {
            continue;
        };
        return Some(LoadedFace {
            bytes,
            index: picked.index,
            label: format!("{} ({})", picked.label, path.display()),
        });
    }
    None
}

fn pick_named_face(bytes: &[u8], family: &str) -> Option<PickedFace> {
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1).max(1);
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(bytes, index) else {
            continue;
        };
        if face.is_italic() {
            continue;
        }
        if face.glyph_index('あ').is_none() || face.glyph_index('漢').is_none() {
            continue;
        }
        let mut matched = false;
        for name in face.names() {
            if name.name_id != 1 {
                continue;
            }
            let Some(text) = name.to_string() else {
                continue;
            };
            if text.trim().eq_ignore_ascii_case(family) {
                matched = true;
                break;
            }
        }
        if !matched {
            continue;
        }
        return Some(PickedFace {
            index,
            label: face_label(&face),
        });
    }
    None
}

struct PickedFace {
    index: u32,
    label: String,
}

fn pick_face(bytes: &[u8]) -> Option<PickedFace> {
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1).max(1);
    let mut best: Option<(i32, PickedFace)> = None;
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(bytes, index) else {
            continue;
        };
        if face.glyph_index('あ').is_none() || face.glyph_index('漢').is_none() {
            continue;
        }
        let label = face_label(&face);
        let score = face_score(&label);
        if best.as_ref().is_none_or(|(prev, _)| score > *prev) {
            best = Some((score, PickedFace { index, label }));
        }
    }
    best.map(|(_, picked)| picked)
}

fn face_label(face: &ttf_parser::Face<'_>) -> String {
    let mut typographic = None;
    let mut family = None;
    let mut full = None;
    for name in face.names() {
        let Some(text) = name.to_string() else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        match name.name_id {
            16 if typographic.is_none() => typographic = Some(text.to_string()),
            1 if family.is_none() => family = Some(text.to_string()),
            4 if full.is_none() => full = Some(text.to_string()),
            _ => {}
        }
    }
    full.or(typographic)
        .or(family)
        .unwrap_or_else(|| "japanese".to_string())
}

fn face_score(label: &str) -> i32 {
    let n = label.to_lowercase();
    let mut score = 0;
    if n.contains("jp") || n.contains("japanese") {
        score += 80;
    }
    if n.contains("hiragino") || n.contains("ヒラギノ") {
        score += 70;
    }
    if n.contains("yu gothic") || n.contains("meiryo") || n.contains("ms gothic") {
        score += 60;
    }
    if n.contains("source han") || n.contains("noto sans") {
        score += 50;
    }
    if n.contains("gothic") || n.contains("sans") {
        score += 10;
    }
    if n.contains("regular") || n.contains("medium") || n.contains(" w3") || n.contains(" w4") {
        score += 8;
    }
    if n.contains("bold") || n.contains("black") || n.contains("heavy") {
        score -= 15;
    }
    if n.contains("mono") {
        score -= 5;
    }
    if n.contains(" cjk sc")
        || n.contains(" cjk kr")
        || n.contains(" cjk tc")
        || n.contains(" cjk hk")
    {
        score -= 40;
    }
    score
}

fn runtime_candidates() -> Vec<PathBuf> {
    match std::env::consts::OS {
        "windows" => {
            let dir = std::env::var_os("WINDIR")
                .map(|windir| PathBuf::from(windir).join("Fonts"))
                .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
            let user = std::env::var_os("LOCALAPPDATA").map(|root| {
                PathBuf::from(root)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts")
            });
            windows_candidates(&dir, user.as_deref())
        }
        "macos" => macos_candidates(std::env::var_os("HOME").map(PathBuf::from).as_deref()),
        "linux" => {
            let mut paths =
                linux_candidates(std::env::var_os("HOME").map(PathBuf::from).as_deref());
            if let Some(text) = fc_list_ja() {
                paths.extend(parse_fc_list(&text));
            }
            dedup_paths(paths)
        }
        other => os_font_candidates(other),
    }
}

pub(crate) fn os_font_candidates(os: &str) -> Vec<PathBuf> {
    match os {
        "windows" => windows_candidates(Path::new(r"C:\Windows\Fonts"), None),
        "macos" => macos_candidates(None),
        "linux" => linux_candidates(None),
        _ => Vec::new(),
    }
}

fn windows_faces(dir: &Path, user: Option<&Path>) -> Vec<(PathBuf, &'static str)> {
    const FACES: &[(&str, &str)] = &[
        ("YuGothM.ttc", "Yu Gothic UI"),
        ("YuGothR.ttc", "Yu Gothic UI"),
        ("YuGothR.ttc", "Yu Gothic UI Semilight"),
        ("meiryo.ttc", "Meiryo UI"),
        ("meiryo.ttc", "Meiryo"),
    ];
    let mut faces = Vec::new();
    for (file, family) in FACES {
        faces.push((dir.join(file), *family));
        if let Some(user) = user {
            faces.push((user.join(file), *family));
        }
    }
    faces
}

fn windows_candidates(dir: &Path, user: Option<&Path>) -> Vec<PathBuf> {
    dedup_paths(
        windows_faces(dir, user)
            .into_iter()
            .map(|(path, _)| path)
            .collect(),
    )
}

fn macos_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    const NAMES: &[&str] = &[
        "ヒラギノ角ゴシック W4.ttc",
        "ヒラギノ角ゴシック W3.ttc",
        "ヒラギノ角ゴシック W5.ttc",
        "ヒラギノ角ゴシック W6.ttc",
        "ヒラギノ角ゴシック W2.ttc",
        "Hiragino Sans W4.ttc",
        "Hiragino Sans W3.ttc",
    ];
    let mut paths = Vec::new();
    for name in NAMES {
        paths.push(PathBuf::from("/System/Library/Fonts").join(name));
        paths.push(PathBuf::from("/Library/Fonts").join(name));
    }
    paths.push(PathBuf::from("/System/Library/Fonts/ヒラギノ明朝 ProN.ttc"));
    paths.push(PathBuf::from("/Library/Fonts/NotoSansJP-Regular.otf"));
    paths.push(PathBuf::from("/Library/Fonts/NotoSansCJK-Regular.ttc"));
    if let Some(home) = home {
        let fonts = home.join("Library/Fonts");
        paths.push(fonts.join("NotoSansJP-Regular.otf"));
        paths.push(fonts.join("NotoSansCJK-Regular.ttc"));
    }
    paths
}

fn linux_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansJP-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.otf",
        "/usr/share/fonts/noto/NotoSansJP-Regular.otf",
        "/usr/share/fonts/OTF/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/adobe-source-han-sans/SourceHanSansJP-Regular.otf",
        "/usr/share/fonts/source-han-sans/SourceHanSansJP-Regular.otf",
        "/usr/share/fonts/opentype/source-han-sans/SourceHanSansJP-Regular.otf",
        "/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf",
        "/usr/share/fonts/truetype/ipafont-gothic/ipag.ttf",
        "/usr/share/fonts/truetype/vlgothic/VL-Gothic-Regular.ttf",
        "/usr/share/fonts/truetype/takao-gothic/TakaoGothic.ttf",
        "/usr/share/fonts/truetype/takao-gothic/TakaoPGothic.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    if let Some(home) = home {
        paths.push(home.join(".local/share/fonts/NotoSansJP-Regular.otf"));
        paths.push(home.join(".local/share/fonts/NotoSansCJK-Regular.ttc"));
        paths.push(home.join(".fonts/NotoSansJP-Regular.otf"));
    }
    paths
}

fn fc_list_ja() -> Option<String> {
    let output = std::process::Command::new("fc-list")
        .args([":lang=ja", "file", "family"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

pub(crate) fn parse_fc_list(text: &str) -> Vec<PathBuf> {
    let mut ranked = Vec::new();
    for line in text.lines() {
        let Some((path, family)) = line.split_once(':') else {
            continue;
        };
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        let family = family.to_lowercase();
        let rank = if family.contains("jp")
            || family.contains("japanese")
            || family.contains("hiragino")
            || family.contains("source han")
            || family.contains("noto sans cjk")
            || family.contains("ipa")
            || family.contains("vl gothic")
            || family.contains("takao")
        {
            0
        } else {
            1
        };
        ranked.push((rank, PathBuf::from(path)));
    }
    ranked.sort_by_key(|(rank, _)| *rank);
    dedup_paths(ranked.into_iter().map(|(_, path)| path).collect())
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

/// UTF-16 code units plus a trailing NUL. This is the encoding `rfd` uses for
/// the Windows file dialog (`encode_utf16`) and the encoding `muda` produces
/// for tray menus (`OsStr::encode_wide` on a Unicode Rust string).
pub fn utf16_with_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn verify_native_text() -> Result<(), String> {
    for label in NATIVE_LABELS {
        let wide = utf16_with_nul(label);
        if wide.last().copied() != Some(0) {
            return Err(format!("missing NUL on {label}"));
        }
        let back = String::from_utf16(&wide[..wide.len() - 1]).map_err(|err| err.to_string())?;
        if back != *label {
            return Err(format!("utf-16 round trip changed {label} into {back}"));
        }
        if wide.len() - 1 != label.chars().count() {
            return Err(format!("{label} is not in the BMP"));
        }
    }
    verify_win32_menu()
}

fn verify_win32_menu() -> Result<(), String> {
    #[cfg(windows)]
    {
        win32_menu_roundtrip()?;
    }
    Ok(())
}

#[cfg(windows)]
fn win32_menu_roundtrip() -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateMenu, DestroyMenu, GetMenuItemInfoW, MENUITEMINFOW, MF_STRING,
        MIIM_STRING,
    };
    use windows::core::{PCWSTR, PWSTR};

    unsafe {
        let menu = CreateMenu().map_err(|err| err.to_string())?;
        let wide = utf16_with_nul("画面を開く");
        AppendMenuW(menu, MF_STRING, 1, PCWSTR(wide.as_ptr())).map_err(|err| err.to_string())?;
        let mut info: MENUITEMINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_STRING;
        info.dwTypeData = PWSTR(std::ptr::null_mut());
        info.cch = 0;
        let _ = GetMenuItemInfoW(menu, 1, false, &mut info);
        let mut buf = vec![0u16; info.cch as usize + 1];
        info.cch = buf.len() as u32;
        info.dwTypeData = PWSTR(buf.as_mut_ptr());
        if let Err(err) = GetMenuItemInfoW(menu, 1, false, &mut info) {
            let _ = DestroyMenu(menu);
            return Err(err.to_string());
        }
        let len = info.cch as usize;
        let got = String::from_utf16_lossy(&buf[..len.min(buf.len())]);
        let _ = DestroyMenu(menu);
        if got != "画面を開く" {
            return Err(format!("Win32 menu returned {got}"));
        }
    }
    Ok(())
}

fn ui_source() -> String {
    let mut text = include_str!("i18n.rs").to_string();
    text.push_str(include_str!("i18n_ko.rs"));
    text.push_str(include_str!("app.rs"));
    text
}

fn quoted_japanese(src: &str) -> Vec<char> {
    let bytes = src.as_bytes();
    let mut chars = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i] != b'"' {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            i += 1;
        }
        if let Ok(text) = std::str::from_utf8(&bytes[start..i.min(bytes.len())])
            && !text.is_ascii()
        {
            for ch in text.chars() {
                if !ch.is_ascii() && !ch.is_whitespace() && seen.insert(ch) {
                    chars.push(ch);
                }
            }
        }
        i += 1;
    }
    chars
}

/// Coverage is checked on the font files themselves. egui's `has_glyph`
/// reports a false negative for every character of the face that also owns
/// the replacement glyph, and Yu Gothic UI is that face once it is first.
fn verify_ui_coverage(stack: &FontStack) -> Result<(), String> {
    let mut chars = quoted_japanese(&ui_source());
    chars.extend(PROOF_LINES.iter().flat_map(|line| line.chars()));
    let missing = stack.missing(chars);
    if !missing.is_empty() {
        let sample: String = missing.iter().take(24).collect();
        return Err(format!(
            "missing {} ui glyphs, including {sample}",
            missing.len()
        ));
    }
    Ok(())
}

const PROOF_LINES: &[&str] = &[
    "T1-Customizer  プリセット 1 2 3",
    "ダイヤル  音量  マイク  ヘッドホン",
    "ダイヤルを回すと音量が変わります。短く押すとモードが切り替わります。",
    "設定ファイル  1234567890  ABC abc",
    "画面を開く    待受の開始    待受の停止    終了",
    "キー  修飾キー  マルチメディア  マウス  LED",
    "接続中  未接続  書き込み  クリア",
];

fn write_proof(path: &Path, defs: &egui::FontDefinitions) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut fonts = Fonts::new(TextOptions::default(), defs.clone());
    let mut view = fonts.with_pixels_per_point(2.0);
    let font_id = FontId::proportional(18.0);
    let margin = 16i32;
    let mut width = 640i32;
    let mut height = margin;
    let mut placed = Vec::new();
    for line in PROOF_LINES {
        let galley =
            view.layout_no_wrap((*line).to_string(), font_id.clone(), egui::Color32::BLACK);
        let atlas = view.image();
        width = width.max(galley.rect.width().ceil() as i32 + margin * 2);
        let top = height;
        height += galley.rect.height().ceil() as i32 + 10;
        placed.push((galley, atlas, top));
    }
    height += margin;
    let mut image = image::RgbaImage::from_pixel(
        width as u32,
        height as u32,
        image::Rgba([255, 255, 255, 255]),
    );
    let mut ink = 0u32;
    for (galley, atlas, top) in &placed {
        ink += blit_galley(atlas, galley, &mut image, margin, *top);
    }
    if ink < 400 {
        return Err(format!("proof image has only {ink} ink pixels"));
    }
    image.save(path).map_err(|err| err.to_string())
}

fn blit_galley(
    atlas: &egui::ColorImage,
    galley: &text::Galley,
    dest: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
) -> u32 {
    let mut ink = 0u32;
    for row in &galley.rows {
        for glyph in &row.glyphs {
            ink += blit_glyph(atlas, glyph, row.pos, dest, origin_x, origin_y);
        }
    }
    ink
}

fn blit_glyph(
    atlas: &egui::ColorImage,
    glyph: &text::Glyph,
    row_pos: egui::Pos2,
    dest: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
) -> u32 {
    let uv = glyph.uv_rect;
    if uv.is_nothing() {
        return 0;
    }
    let src_w = uv.max[0].saturating_sub(uv.min[0]) as i32;
    let src_h = uv.max[1].saturating_sub(uv.min[1]) as i32;
    if src_w <= 0 || src_h <= 0 {
        return 0;
    }
    let dest_w = uv.size.x.round().max(1.0) as i32;
    let dest_h = uv.size.y.round().max(1.0) as i32;
    let base_x = origin_x + (row_pos.x + glyph.pos.x + uv.offset.x).round() as i32;
    let base_y = origin_y + (row_pos.y + glyph.pos.y + uv.offset.y).round() as i32;
    let atlas_w = atlas.size[0] as i32;
    let mut ink = 0u32;
    for dy in 0..dest_h {
        for dx in 0..dest_w {
            let sx = uv.min[0] as i32 + dx * src_w / dest_w;
            let sy = uv.min[1] as i32 + dy * src_h / dest_h;
            if sx < 0 || sy < 0 || sx >= atlas_w || sy >= atlas.size[1] as i32 {
                continue;
            }
            let pixel = atlas.pixels[sy as usize * atlas.size[0] + sx as usize];
            let alpha = pixel.a();
            if alpha == 0 {
                continue;
            }
            let x = base_x + dx;
            let y = base_y + dy;
            if x < 0 || y < 0 || x >= dest.width() as i32 || y >= dest.height() as i32 {
                continue;
            }
            let value = 255u16.saturating_sub(u16::from(alpha)) as u8;
            dest.put_pixel(x as u32, y as u32, image::Rgba([value, value, value, 255]));
            ink += 1;
        }
    }
    ink
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_egui_font_has_no_kana() {
        let mut fonts = Fonts::new(TextOptions::default(), egui::FontDefinitions::default());
        let mut view = fonts.with_pixels_per_point(1.0);
        let font_id = FontId::proportional(16.0);
        assert!(!view.has_glyph(&font_id, 'あ'));
        assert!(!view.has_glyph(&font_id, '漢'));
    }

    #[test]
    fn windows_faces_prefer_yu_gothic_ui_then_meiryo() {
        let faces = windows_faces(Path::new(r"C:\Windows\Fonts"), None);
        assert_eq!(faces[0].1, "Yu Gothic UI");
        assert!(faces[0].0.ends_with("YuGothM.ttc"));
        let first_meiryo = faces
            .iter()
            .position(|(_, family)| family.starts_with("Meiryo"))
            .unwrap();
        for (path, family) in &faces[..first_meiryo] {
            assert!(family.starts_with("Yu Gothic"), "{path:?} {family}");
        }
        assert!(!faces.iter().any(|(_, family)| family.contains("MS Gothic")));
    }

    #[test]
    fn pick_named_face_matches_family() {
        let mut decoder = flate2::read::GzDecoder::new(BUNDLED_GZ);
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut bytes).unwrap();
        assert!(pick_named_face(&bytes, "Noto Sans JP").is_some());
        assert!(pick_named_face(&bytes, "Yu Gothic UI").is_none());
    }

    #[test]
    fn japanese_face_is_first_for_proportional() {
        let bundled = load_bundled().unwrap();
        let system = LoadedFace {
            bytes: bundled.bytes.clone(),
            index: bundled.index,
            label: "Test System".to_string(),
        };
        let defs = FontStack {
            system: Some(system),
            bundled,
            korean: load_bundled_kr().ok(),
        }
        .into_definitions();
        assert_eq!(defs.families[&FontFamily::Proportional][0], "t1-system-jp");
        assert_eq!(defs.families[&FontFamily::Monospace][1], "t1-system-jp");
        // The bundled Japanese subset is the last Japanese face; only the
        // Hangul fallback comes after it.
        let list = &defs.families[&FontFamily::Proportional];
        assert_eq!(list[list.len() - 2], "t1-bundled-jp");
        assert_eq!(list.last().unwrap(), "t1-bundled-kr");
    }

    #[test]
    fn bundled_only_goes_first_without_system_face() {
        let defs = FontStack::bundled_only().unwrap().into_definitions();
        assert_eq!(defs.families[&FontFamily::Proportional][0], "t1-bundled-jp");
    }

    #[test]
    fn macos_candidates_include_hiragino() {
        let paths = os_font_candidates("macos");
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("ヒラギノ角ゴシック W3.ttc"))
        );
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("ヒラギノ角ゴシック W4.ttc"))
        );
    }

    #[test]
    fn linux_candidates_include_noto_cjk() {
        let paths = os_font_candidates("linux");
        assert!(
            paths
                .iter()
                .any(|path| path.ends_with("NotoSansCJK-Regular.ttc"))
        );
        assert!(paths.iter().any(|path| path.ends_with("ipag.ttf")));
    }

    #[test]
    fn fc_list_prefers_a_japanese_family() {
        let text = "\
/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf: Droid Sans Fallback
/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc: Noto Sans CJK JP
/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc: Noto Sans CJK SC
";
        let paths = parse_fc_list(text);
        assert_eq!(
            paths[0],
            PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc")
        );
        assert!(paths.len() >= 2);
    }

    #[test]
    fn face_names_prefer_japanese_regular() {
        assert!(face_score("Noto Sans CJK JP") > face_score("Noto Sans CJK SC"));
        assert!(face_score("Yu Gothic Medium") > face_score("Yu Gothic Bold"));
        assert!(face_score("Meiryo") > 0);
        assert!(face_score("MS Gothic") > 0);
        assert!(face_score("Hiragino Sans W4") > face_score("Arial"));
    }

    #[test]
    fn native_labels_round_trip_as_utf16() {
        verify_native_text().unwrap();
    }

    #[test]
    fn bundled_subset_covers_the_japanese_ui() {
        let stack = FontStack::bundled_only().unwrap();
        verify_ui_coverage(&stack).unwrap();
        let defs = stack.into_definitions();
        let path = std::env::temp_dir().join("t1-vremokey-ja-proof.png");
        write_proof(&path, &defs).unwrap();
        assert!(path.metadata().unwrap().len() > 1000);
    }

    fn hangul_in(src: &str) -> Vec<char> {
        quoted_japanese(src)
            .into_iter()
            .filter(|c| {
                ('\u{AC00}'..='\u{D7A3}').contains(c) || ('\u{3131}'..='\u{318E}').contains(c)
            })
            .collect()
    }

    #[test]
    fn bundled_korean_face_draws_every_korean_ui_glyph() {
        let hangul = hangul_in(include_str!("i18n_ko.rs"));
        assert!(
            hangul.len() > 200,
            "only {} Hangul in i18n_ko.rs",
            hangul.len()
        );
        let kr = load_bundled_kr().unwrap();
        let face = ttf_parser::Face::parse(&kr.bytes, kr.index).unwrap();
        let missing: String = hangul
            .iter()
            .filter(|c| face.glyph_index(**c).is_none())
            .collect();
        assert!(missing.is_empty(), "bundled Korean face lacks {missing}");
        // Every KS X 1001 syllable, so Korean expression names draw too.
        for ch in ['가', '각', '한', '국', '어', '힝', 'ㄱ', 'ㅎ'] {
            assert!(face.glyph_index(ch).is_some(), "{ch}");
        }
        assert!(kr.label.contains("Noto Sans KR"), "{}", kr.label);
    }

    #[test]
    fn korean_face_comes_after_the_japanese_faces() {
        let defs = FontStack::bundled_only().unwrap().into_definitions();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let list = &defs.families[&family];
            let jp = list
                .iter()
                .position(|name| name == "t1-bundled-jp")
                .unwrap();
            let kr = list
                .iter()
                .position(|name| name == "t1-bundled-kr")
                .unwrap_or_else(|| panic!("{family:?} has no Korean face: {list:?}"));
            assert!(jp < kr, "{family:?} {list:?}");
        }
        // Japanese text still comes from the Japanese face first.
        assert_eq!(defs.families[&FontFamily::Proportional][0], "t1-bundled-jp");
    }

    #[test]
    fn bundled_korean_subset_stays_small() {
        assert!(
            (50 * 1024..400 * 1024).contains(&BUNDLED_KR_GZ.len()),
            "Korean subset is {} bytes",
            BUNDLED_KR_GZ.len()
        );
    }

    #[test]
    fn license_text_is_the_sil_ofl() {
        let text = ofl_text();
        assert!(text.contains("SIL OPEN FONT LICENSE"));
        assert!(text.contains("Reserved Font Name 'Source'"));
        assert!(text.contains("Adobe"));
    }
}
