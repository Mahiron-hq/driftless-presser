//! Configuration schema, persistence, and hotkey validation.
//!
//! Platform-independent (no Win32 dependency) so it can be unit tested on
//! any host. `hook.rs` reuses the VK-code table here as the single source
//! of truth for key-name parsing.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// User-configurable settings, persisted to `config.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Keyboard key that gets auto-pressed while the clicker is active.
    pub target_key: String,
    /// Key presses per second while RMB is *not* held.
    pub default_frequency_hz: f64,
    /// Key presses per second while RMB *is* held.
    pub rmb_frequency_hz: f64,
    /// Global toggle hotkey, 2-3 keys, e.g. `["Ctrl", "W", "S"]`.
    pub hotkey: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            target_key: "E".to_string(),
            default_frequency_hz: 4.0,
            rmb_frequency_hz: 7.0,
            hotkey: vec!["Ctrl".to_string(), "W".to_string(), "S".to_string()],
        }
    }
}

impl Config {
    /// Resolves next to the executable rather than the current working
    /// directory, which varies depending on how Windows launched the app
    /// (shortcut, startup entry, terminal).
    pub fn file_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("config.json")))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }

    /// Loads `config.json`, creating it with default values if it's
    /// missing or unreadable.
    pub fn load_or_init() -> Config {
        Self::load_or_init_from(&Self::file_path())
    }

    fn load_or_init_from(path: &Path) -> Config {
        let existing = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Config>(&text).ok());

        match existing {
            Some(cfg) => cfg,
            None => {
                let cfg = Config::default();
                let _ = cfg.save_to(path);
                cfg
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::file_path())
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .expect("Config only contains plain serializable fields");
        fs::write(path, json)
    }
}

/// Win32 virtual-key constants, re-declared here rather than imported from
/// `winapi` so this module stays platform-independent.
pub mod vk {
    pub const LBUTTON: u16 = 0x01;
    pub const RBUTTON: u16 = 0x02;
    pub const MBUTTON: u16 = 0x04;
    pub const XBUTTON1: u16 = 0x05;
    pub const XBUTTON2: u16 = 0x06;
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12; // Alt
    pub const PAUSE: u16 = 0x13;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21; // Page Up
    pub const NEXT: u16 = 0x22; // Page Down
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const LWIN: u16 = 0x5B;
    pub const RWIN: u16 = 0x5C;
    pub const F1: u16 = 0x70;
    pub const LSHIFT: u16 = 0xA0;
    pub const RSHIFT: u16 = 0xA1;
    pub const LCONTROL: u16 = 0xA2;
    pub const RCONTROL: u16 = 0xA3;
    pub const LMENU: u16 = 0xA4;
    pub const RMENU: u16 = 0xA5;

    /// Parses a human-typed key name ("Ctrl", "E", "F5", "Space", ...)
    /// into its Win32 virtual-key code. Case-insensitive.
    pub fn from_name(name: &str) -> Option<u16> {
        let upper = name.trim().to_ascii_uppercase();
        if upper.is_empty() {
            return None;
        }

        // VK codes for A-Z and 0-9 are identical to their ASCII values.
        if upper.len() == 1 {
            let c = upper.as_bytes()[0];
            if c.is_ascii_uppercase() || c.is_ascii_digit() {
                return Some(c as u16);
            }
        }

        if let Some(rest) = upper.strip_prefix('F') {
            if let Ok(n) = rest.parse::<u16>() {
                if (1..=24).contains(&n) {
                    return Some(F1 + (n - 1));
                }
            }
        }

        Some(match upper.as_str() {
            "CTRL" | "CONTROL" => CONTROL,
            "LCTRL" | "LCONTROL" => LCONTROL,
            "RCTRL" | "RCONTROL" => RCONTROL,
            "SHIFT" => SHIFT,
            "LSHIFT" => LSHIFT,
            "RSHIFT" => RSHIFT,
            "ALT" | "MENU" => MENU,
            "LALT" | "LMENU" => LMENU,
            "RALT" | "RMENU" => RMENU,
            "WIN" | "WINDOWS" | "LWIN" => LWIN,
            "RWIN" => RWIN,
            "SPACE" | "SPACEBAR" => SPACE,
            "TAB" => TAB,
            "ENTER" | "RETURN" => RETURN,
            "ESC" | "ESCAPE" => ESCAPE,
            "BACKSPACE" | "BACK" => BACK,
            "DELETE" | "DEL" => DELETE,
            "INSERT" | "INS" => INSERT,
            "HOME" => HOME,
            "END" => END,
            "PAGEUP" | "PGUP" | "PRIOR" => PRIOR,
            "PAGEDOWN" | "PGDN" | "NEXT" => NEXT,
            "UP" => UP,
            "DOWN" => DOWN,
            "LEFT" => LEFT,
            "RIGHT" => RIGHT,
            "PAUSE" | "BREAK" => PAUSE,
            "MOUSE4" | "XBUTTON1" => XBUTTON1,
            "MOUSE5" | "XBUTTON2" => XBUTTON2,
            _ => return None,
        })
    }

    /// Modifier names, used both for the "at least one modifier" rule and
    /// to normalize left/right variants when matching a combo.
    pub fn is_modifier_name(upper_name: &str) -> bool {
        matches!(
            upper_name,
            "CTRL" | "CONTROL" | "LCTRL" | "LCONTROL" | "RCTRL" | "RCONTROL"
                | "SHIFT" | "LSHIFT" | "RSHIFT"
                | "ALT" | "MENU" | "LALT" | "LMENU" | "RALT" | "RMENU"
                | "WIN" | "WINDOWS" | "LWIN" | "RWIN"
        )
    }

    /// Inverse of [`from_name`]. Left/right modifier variants collapse to
    /// the generic name, matching how the hook matches them either way.
    pub fn name_for(vk: u16) -> String {
        if (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) {
            return (vk as u8 as char).to_string();
        }
        if (F1..F1 + 24).contains(&vk) {
            return format!("F{}", vk - F1 + 1);
        }
        match vk {
            CONTROL | LCONTROL | RCONTROL => "Ctrl",
            SHIFT | LSHIFT | RSHIFT => "Shift",
            MENU | LMENU | RMENU => "Alt",
            LWIN | RWIN => "Win",
            SPACE => "Space",
            TAB => "Tab",
            RETURN => "Enter",
            ESCAPE => "Esc",
            BACK => "Backspace",
            DELETE => "Delete",
            INSERT => "Insert",
            HOME => "Home",
            END => "End",
            PRIOR => "PageUp",
            NEXT => "PageDown",
            UP => "Up",
            DOWN => "Down",
            LEFT => "Left",
            RIGHT => "Right",
            PAUSE => "Pause",
            XBUTTON1 => "Mouse4",
            XBUTTON2 => "Mouse5",
            other => return format!("VK{other:#04X}"),
        }
        .to_string()
    }
}

/// Splits a user-typed hotkey string like `"Ctrl+W+S"` or `"Ctrl, W, S"`
/// into individual key-name tokens.
pub fn parse_hotkey_string(input: &str) -> Vec<String> {
    input
        .split(|c| c == '+' || c == ',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyValidationError {
    TooFewKeys,
    TooManyKeys,
    UnknownKey(String),
    Duplicate(String),
    NoModifier,
    Forbidden(&'static str),
}

impl std::fmt::Display for HotkeyValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewKeys => write!(f, "Choose at least 2 keys for the hotkey."),
            Self::TooManyKeys => write!(f, "Hotkeys support at most 3 keys."),
            Self::UnknownKey(k) => write!(f, "'{k}' isn't a recognized key name."),
            Self::Duplicate(k) => write!(f, "'{k}' is listed more than once."),
            Self::NoModifier => write!(
                f,
                "Include at least one modifier (Ctrl, Alt, Shift, or Win) so the hotkey doesn't trigger during normal typing."
            ),
            Self::Forbidden(reason) => write!(
                f,
                "That combination is reserved by Windows ({reason}) and can't be used here."
            ),
        }
    }
}

/// A candidate combo is rejected if its key set is a *superset* of any
/// entry here: extra keys don't defuse e.g. Ctrl+Alt+Delete, since Windows
/// still intercepts the secure attention sequence regardless of what else
/// is held.
const BLACKLIST: &[(&[&str], &str)] = &[
    (&["CTRL", "ALT", "DELETE"], "Ctrl+Alt+Delete"),
    (&["CTRL", "ALT", "DEL"], "Ctrl+Alt+Delete"),
    (&["ALT", "F4"], "Alt+F4, close window"),
    (&["WIN", "R"], "Win+R, Run dialog"),
    (&["WIN", "L"], "Win+L, lock workstation"),
    (&["WIN", "D"], "Win+D, show desktop"),
    (&["WIN", "E"], "Win+E, File Explorer"),
    (&["WIN", "TAB"], "Win+Tab, Task View"),
    (&["ALT", "TAB"], "Alt+Tab, switch window"),
    (&["CTRL", "SHIFT", "ESCAPE"], "Ctrl+Shift+Esc, Task Manager"),
    (&["CTRL", "ESCAPE"], "Ctrl+Esc, Start menu"),
    (&["CTRL", "SHIFT", "S"], "Ctrl+Shift+S"),
    (&["WIN", "SHIFT", "S"], "Win+Shift+S, Snipping Tool"),
    (&["WIN", "I"], "Win+I, Settings"),
    (&["WIN", "PAUSE"], "Win+Pause, System Properties"),
];

/// Validates a candidate hotkey combo (raw typed key names). On success,
/// returns the canonicalized (trimmed, uppercased) names to store in
/// `Config::hotkey`.
pub fn validate_hotkey(keys: &[String]) -> Result<Vec<String>, HotkeyValidationError> {
    if keys.len() < 2 {
        return Err(HotkeyValidationError::TooFewKeys);
    }
    if keys.len() > 3 {
        return Err(HotkeyValidationError::TooManyKeys);
    }

    let mut canonical = Vec::with_capacity(keys.len());
    let mut seen = HashSet::new();
    for k in keys {
        let upper = k.trim().to_ascii_uppercase();
        if vk::from_name(&upper).is_none() {
            return Err(HotkeyValidationError::UnknownKey(k.clone()));
        }
        if !seen.insert(upper.clone()) {
            return Err(HotkeyValidationError::Duplicate(k.clone()));
        }
        canonical.push(upper);
    }

    if !canonical.iter().any(|k| vk::is_modifier_name(k)) {
        return Err(HotkeyValidationError::NoModifier);
    }

    let user_set: HashSet<&str> = canonical.iter().map(String::as_str).collect();
    for (forbidden, label) in BLACKLIST {
        if forbidden.iter().all(|k| user_set.contains(k)) {
            return Err(HotkeyValidationError::Forbidden(label));
        }
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch_path() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("auto_clicker_test_{nanos}_{n}.json"))
    }

    #[test]
    fn default_config_has_expected_values() {
        let cfg = Config::default();
        assert_eq!(cfg.target_key, "E");
        assert_eq!(cfg.default_frequency_hz, 4.0);
        assert_eq!(cfg.rmb_frequency_hz, 7.0);
        assert_eq!(cfg.hotkey, vec!["Ctrl", "W", "S"]);
    }

    #[test]
    fn missing_file_is_initialized_with_defaults_and_persisted() {
        let path = scratch_path();
        assert!(!path.exists());

        let cfg = Config::load_or_init_from(&path);
        assert_eq!(cfg, Config::default());
        assert!(path.exists(), "load_or_init must write the file back out");

        let reloaded = Config::load_or_init_from(&path);
        assert_eq!(reloaded, cfg);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_roundtrips_custom_values() {
        let path = scratch_path();
        let cfg = Config {
            target_key: "Q".to_string(),
            default_frequency_hz: 5.5,
            rmb_frequency_hz: 12.0,
            hotkey: vec!["Ctrl".to_string(), "Q".to_string()],
        };
        cfg.save_to(&path).unwrap();

        let loaded = Config::load_or_init_from(&path);
        assert_eq!(loaded, cfg);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path = scratch_path();
        fs::write(&path, "{ not valid json ").unwrap();

        let cfg = Config::load_or_init_from(&path);
        assert_eq!(cfg, Config::default());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn vk_mapping_letters_digits_and_names() {
        assert_eq!(vk::from_name("E"), Some(b'E' as u16));
        assert_eq!(vk::from_name("e"), Some(b'E' as u16));
        assert_eq!(vk::from_name("5"), Some(b'5' as u16));
        assert_eq!(vk::from_name("F5"), Some(vk::F1 + 4));
        assert_eq!(vk::from_name("ctrl"), Some(vk::CONTROL));
        assert_eq!(vk::from_name("Win"), Some(vk::LWIN));
        assert_eq!(vk::from_name("nonsense_key"), None);
        assert_eq!(vk::from_name(""), None);
    }

    #[test]
    fn name_for_round_trips_through_from_name() {
        let cases = [
            b'E' as u16,
            b'5' as u16,
            vk::F1 + 4,
            vk::LCONTROL,
            vk::RSHIFT,
            vk::LWIN,
            vk::SPACE,
            vk::RETURN,
        ];
        for vk_code in cases {
            let name = vk::name_for(vk_code);
            let reparsed = vk::from_name(&name).unwrap_or_else(|| panic!("name_for({vk_code:#04x}) = {name:?} didn't reparse"));
            let reparsed_name = vk::name_for(reparsed);
            assert_eq!(name, reparsed_name, "round trip mismatch for {vk_code:#04x}");
        }
    }

    #[test]
    fn name_for_common_keys_is_human_readable() {
        assert_eq!(vk::name_for(b'E' as u16), "E");
        assert_eq!(vk::name_for(vk::F1 + 4), "F5");
        assert_eq!(vk::name_for(vk::CONTROL), "Ctrl");
        assert_eq!(vk::name_for(vk::LCONTROL), "Ctrl");
        assert_eq!(vk::name_for(vk::RCONTROL), "Ctrl");
        assert_eq!(vk::name_for(vk::LWIN), "Win");
    }

    #[test]
    fn accepts_default_hotkey() {
        let keys = vec!["Ctrl".to_string(), "W".to_string(), "S".to_string()];
        assert_eq!(validate_hotkey(&keys), Ok(vec!["CTRL".to_string(), "W".to_string(), "S".to_string()]));
    }

    #[test]
    fn rejects_single_key() {
        let keys = vec!["Ctrl".to_string()];
        assert_eq!(validate_hotkey(&keys), Err(HotkeyValidationError::TooFewKeys));
    }

    #[test]
    fn rejects_four_keys() {
        let keys = vec!["Ctrl".to_string(), "Alt".to_string(), "Shift".to_string(), "S".to_string()];
        assert_eq!(validate_hotkey(&keys), Err(HotkeyValidationError::TooManyKeys));
    }

    #[test]
    fn rejects_unknown_key_name() {
        let keys = vec!["Ctrl".to_string(), "Blorp".to_string()];
        assert_eq!(
            validate_hotkey(&keys),
            Err(HotkeyValidationError::UnknownKey("Blorp".to_string()))
        );
    }

    #[test]
    fn rejects_combo_without_modifier() {
        let keys = vec!["W".to_string(), "S".to_string()];
        assert_eq!(validate_hotkey(&keys), Err(HotkeyValidationError::NoModifier));
    }

    #[test]
    fn rejects_ctrl_alt_delete() {
        let keys = vec!["Ctrl".to_string(), "Alt".to_string(), "Delete".to_string()];
        assert!(matches!(validate_hotkey(&keys), Err(HotkeyValidationError::Forbidden(_))));
    }

    #[test]
    fn rejects_win_r() {
        let keys = vec!["Win".to_string(), "R".to_string()];
        assert!(matches!(validate_hotkey(&keys), Err(HotkeyValidationError::Forbidden(_))));
    }

    #[test]
    fn rejects_alt_f4() {
        let keys = vec!["Alt".to_string(), "F4".to_string()];
        assert!(matches!(validate_hotkey(&keys), Err(HotkeyValidationError::Forbidden(_))));
    }

    #[test]
    fn rejects_superset_of_blacklisted_combo() {
        let keys = vec!["Ctrl".to_string(), "Alt".to_string(), "Delete".to_string()];
        assert!(matches!(validate_hotkey(&keys), Err(HotkeyValidationError::Forbidden(_))));
    }

    #[test]
    fn accepts_reasonable_custom_combo() {
        let keys = vec!["Ctrl".to_string(), "Shift".to_string(), "F9".to_string()];
        assert!(validate_hotkey(&keys).is_ok());
    }

    #[test]
    fn parses_plus_and_comma_separated_strings() {
        assert_eq!(parse_hotkey_string("Ctrl+W+S"), vec!["Ctrl", "W", "S"]);
        assert_eq!(parse_hotkey_string("Ctrl, W, S"), vec!["Ctrl", "W", "S"]);
        assert_eq!(parse_hotkey_string(" Ctrl + Alt "), vec!["Ctrl", "Alt"]);
    }
}
