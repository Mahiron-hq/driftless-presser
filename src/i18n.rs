//! Minimal localization: system-language detection plus English/Russian
//! string tables. Hand-rolled rather than a full i18n crate since there
//! are only two languages and a couple dozen strings.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

impl Lang {
    /// Detects the system UI language: Russian if the locale is Russian,
    /// English otherwise.
    pub fn detect() -> Self {
        Self::from_locale_str(sys_locale::get_locale().as_deref().unwrap_or("en"))
    }

    fn from_locale_str(locale: &str) -> Self {
        // Locale strings look like "ru", "ru-RU", "ru_RU.UTF-8", etc.
        // Matching on the leading "ru" segment covers all of these.
        if locale.to_ascii_lowercase().starts_with("ru") {
            Lang::Ru
        } else {
            Lang::En
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::Ru => &RU,
        }
    }
}

pub struct Strings {
    pub window_title: &'static str,
    pub target_key_label: &'static str,
    pub default_freq_label: &'static str,
    pub rmb_freq_label: &'static str,
    pub hotkey_label: &'static str,
    pub start_button: &'static str,
    pub record_prompt_single: &'static str,
    pub record_prompt_combo: &'static str,
    pub click_to_record: &'static str,
    pub tray_tooltip: &'static str,
    pub tray_open_settings: &'static str,
    pub tray_exit: &'static str,
    pub err_save_failed: &'static str,
    pub err_default_freq: &'static str,
    pub err_rmb_freq: &'static str,
    pub err_target_key_unknown: &'static str,
    pub err_too_few_keys: &'static str,
    pub err_too_many_keys: &'static str,
    pub err_unknown_key: &'static str,
    pub err_duplicate_key: &'static str,
    pub err_no_modifier: &'static str,
    pub err_forbidden: &'static str,
}

static EN: Strings = Strings {
    window_title: "Auto-Clicker Settings",
    target_key_label: "Target Key",
    default_freq_label: "Default Frequency (Hz)",
    rmb_freq_label: "RMB Frequency (Hz)",
    hotkey_label: "Toggle Hotkey",
    start_button: "Start",
    record_prompt_single: "Press a key\u{2026}",
    record_prompt_combo: "Press 2-3 keys, then release\u{2026}",
    click_to_record: "Click, then press a key",
    tray_tooltip: "Auto-Clicker",
    tray_open_settings: "Open Settings",
    tray_exit: "Exit",
    err_save_failed: "Couldn't save config.json:",
    err_default_freq: "Default frequency must be a positive number.",
    err_rmb_freq: "RMB frequency must be a positive number.",
    err_target_key_unknown: "isn't a recognized key.",
    err_too_few_keys: "Choose at least 2 keys for the hotkey.",
    err_too_many_keys: "Hotkeys support at most 3 keys.",
    err_unknown_key: "isn't a recognized key name.",
    err_duplicate_key: "is listed more than once.",
    err_no_modifier: "Include at least one modifier (Ctrl, Alt, Shift, or Win) so the hotkey doesn't trigger during normal typing.",
    err_forbidden: "That combination is reserved by Windows and can't be used here.",
};

static RU: Strings = Strings {
    window_title: "Настройки автокликера",
    target_key_label: "Целевая клавиша",
    default_freq_label: "Частота по умолчанию (Гц)",
    rmb_freq_label: "Частота при ПКМ (Гц)",
    hotkey_label: "Горячая клавиша вкл/выкл",
    start_button: "Старт",
    record_prompt_single: "Нажмите клавишу\u{2026}",
    record_prompt_combo: "Нажмите 2-3 клавиши и отпустите\u{2026}",
    click_to_record: "Нажмите, затем клавишу",
    tray_tooltip: "Автокликер",
    tray_open_settings: "Открыть настройки",
    tray_exit: "Выход",
    err_save_failed: "Не удалось сохранить config.json:",
    err_default_freq: "Частота по умолчанию должна быть положительным числом.",
    err_rmb_freq: "Частота при ПКМ должна быть положительным числом.",
    err_target_key_unknown: "— нераспознанная клавиша.",
    err_too_few_keys: "Выберите как минимум 2 клавиши для горячей клавиши.",
    err_too_many_keys: "Горячая клавиша поддерживает не более 3 клавиш.",
    err_unknown_key: "— нераспознанное имя клавиши.",
    err_duplicate_key: "указана более одного раза.",
    err_no_modifier: "Добавьте хотя бы один модификатор (Ctrl, Alt, Shift или Win), чтобы клавиша не срабатывала во время обычного набора текста.",
    err_forbidden: "Эта комбинация зарезервирована Windows и не может быть использована.",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_locale_variants_all_select_russian() {
        for locale in ["ru", "ru-RU", "ru_RU.UTF-8", "RU", "ru-UA"] {
            assert_eq!(Lang::from_locale_str(locale), Lang::Ru, "failed for {locale}");
        }
    }

    #[test]
    fn everything_else_defaults_to_english() {
        for locale in ["en", "en-US", "de-DE", "fr_FR", "uk-UA", "", "zh-Hans-CN"] {
            assert_eq!(Lang::from_locale_str(locale), Lang::En, "failed for {locale}");
        }
    }

    #[test]
    fn both_string_tables_are_non_empty() {
        // A cheap guard against a copy-paste leaving a field blank.
        let check = |s: &Strings| {
            assert!(!s.window_title.is_empty());
            assert!(!s.start_button.is_empty());
            assert!(!s.tray_open_settings.is_empty());
            assert!(!s.tray_exit.is_empty());
            assert!(!s.err_forbidden.is_empty());
        };
        check(Lang::En.strings());
        check(Lang::Ru.strings());
    }
}
