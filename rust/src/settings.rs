//! settings.json. Unknown keys stay in the map so a newer field is not dropped.
//! The file is pretty-printed without a trailing newline, matching Python's
//! `json.dump(..., indent=2)`.

use std::fs;

use serde_json::{Map, Value};

use crate::paths;

pub type Settings = Map<String, Value>;

pub fn load_settings() -> Settings {
    match fs::read_to_string(paths::config_file()) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        },
        Err(_) => Map::new(),
    }
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    paths::ensure_parent(&paths::config_file()).map_err(|err| err.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    let tmp = paths::config_file().with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|err| err.to_string())?;
    fs::rename(&tmp, paths::config_file()).map_err(|err| err.to_string())?;
    Ok(())
}

pub fn settings_mtime() -> u128 {
    fs::metadata(paths::config_file())
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|dur| dur.as_nanos())
        .unwrap_or(0)
}

pub fn as_bool(value: Option<&Value>, default: bool) -> bool {
    match value {
        None => default,
        Some(Value::String(text)) => !matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        ),
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(1) != 0,
        Some(_) => default,
    }
}

pub fn ensure_object<'a>(map: &'a mut Settings, key: &str) -> &'a mut Map<String, Value> {
    if !map.get(key).is_some_and(|v| v.is_object()) {
        map.insert(key.to_string(), Value::Object(Map::new()));
    }
    map.get_mut(key)
        .and_then(|v| v.as_object_mut())
        .expect("object")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_bool_defaults() {
        assert!(as_bool(None, true));
        assert!(!as_bool(None, false));
        assert!(!as_bool(Some(&Value::Bool(false)), true));
        assert!(as_bool(Some(&Value::String("on".into())), false));
        assert!(!as_bool(Some(&Value::String("off".into())), true));
    }

    #[test]
    fn ensure_object_creates_map() {
        let mut settings = Settings::new();
        let obj = ensure_object(&mut settings, "dial");
        obj.insert("step".into(), Value::from(0.05));
        assert_eq!(settings["dial"]["step"].as_f64(), Some(0.05));
    }
}
