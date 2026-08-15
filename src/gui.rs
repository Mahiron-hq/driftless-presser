//! Settings window and system tray (Windows-only), built with
//! native-windows-gui.
//!
//! The window is built once and kept alive for the app's entire lifetime,
//! toggling only its visibility (`set_visible`) rather than being rebuilt.
//! The tray icon is the opposite: `TrayNotification` exposes no way to
//! query or change its own visibility after creation, so it's built when
//! needed and dropped (which removes it) when not, mirroring the window's
//! hidden/shown state so exactly one of the two is ever visible.

use crate::clicker::windows_impl::ClickerCommand;
use crate::config::{self, Config, HotkeyValidationError};
use crate::hook::{InputState, RecordTarget};
use crate::i18n::Strings;
use crate::theme::ThemeMode;
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldId {
    TargetKey,
    Hotkey,
}

pub struct AppUi {
    window: nwg::Window,
    layout: nwg::GridLayout,

    target_key_label: nwg::Label,
    target_key_button: nwg::Button,
    default_freq_label: nwg::Label,
    default_freq_input: nwg::TextInput,
    rmb_freq_label: nwg::Label,
    rmb_freq_input: nwg::TextInput,
    hotkey_label: nwg::Label,
    hotkey_button: nwg::Button,
    error_label: nwg::Label,
    start_button: nwg::Button,

    icon: nwg::Icon,

    // Built once and parented to the window (which is never destroyed), so
    // it's reused every time the tray icon itself is recreated.
    tray_menu: nwg::Menu,
    tray_open_item: nwg::MenuItem,
    tray_exit_item: nwg::MenuItem,
    tray: RefCell<Option<nwg::TrayNotification>>,

    recording_timer: nwg::AnimationTimer,

    state: Arc<InputState>,
    clicker_tx: Sender<ClickerCommand>,
    strings: &'static Strings,
    config: RefCell<Config>,
    recording: RefCell<Option<FieldId>>,
}

impl AppUi {
    /// Re-detects and applies the current system theme to the title bar.
    /// Called at startup and every time the window is shown again, so a
    /// theme change made while the app was in the tray takes effect.
    fn apply_current_theme(&self) {
        if let Some(hwnd) = self.window.handle.hwnd() {
            ThemeMode::detect().apply_to_title_bar(hwnd);
        }
    }

    fn populate_fields_from_config(&self) {
        let cfg = self.config.borrow();
        self.target_key_button.set_text(&cfg.target_key);
        self.default_freq_input.set_text(&format!("{}", cfg.default_frequency_hz));
        self.rmb_freq_input.set_text(&format!("{}", cfg.rmb_frequency_hz));
        self.hotkey_button.set_text(&cfg.hotkey.join("+"));
        self.error_label.set_text("");
    }

    /// No-op if the tray icon is already showing. Failure (e.g. a
    /// transient shell issue) is silently ignored rather than treated as
    /// fatal -- the window still works fine either way.
    fn show_tray(&self) {
        if self.tray.borrow().is_some() {
            return;
        }
        let mut tray = nwg::TrayNotification::default();
        let built = nwg::TrayNotification::builder()
            .parent(&self.window)
            .icon(Some(&self.icon))
            .tip(Some(self.strings.tray_tooltip))
            .build(&mut tray);
        if built.is_ok() {
            *self.tray.borrow_mut() = Some(tray);
        }
    }

    fn hide_tray(&self) {
        *self.tray.borrow_mut() = None;
    }

    /// Shared by the Start button (after a successful save) and the native
    /// close button (no save, just dismiss) -- neither one quits the app.
    fn hide_and_show_tray(&self) {
        self.window.set_visible(false);
        self.show_tray();
    }

    fn on_open_settings(&self) {
        self.hide_tray();
        self.populate_fields_from_config();
        self.apply_current_theme();
        self.window.set_visible(true);
    }

    fn show_tray_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn on_start(&self) {
        self.error_label.set_text("");

        let target_key_raw = self.target_key_button.text();
        let target_key = target_key_raw.trim().to_ascii_uppercase();
        if config::vk::from_name(&target_key).is_none() {
            self.error_label
                .set_text(&format!("'{}' {}", target_key_raw.trim(), self.strings.err_target_key_unknown));
            return;
        }

        let default_hz = match self.default_freq_input.text().trim().parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => v,
            _ => {
                self.error_label.set_text(self.strings.err_default_freq);
                return;
            }
        };

        let rmb_hz = match self.rmb_freq_input.text().trim().parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => v,
            _ => {
                self.error_label.set_text(self.strings.err_rmb_freq);
                return;
            }
        };

        let hotkey_tokens = config::parse_hotkey_string(&self.hotkey_button.text());
        let hotkey = match config::validate_hotkey(&hotkey_tokens) {
            Ok(keys) => keys,
            Err(e) => {
                self.error_label.set_text(&translate_hotkey_error(&e, self.strings));
                return;
            }
        };

        let new_config = Config {
            target_key,
            default_frequency_hz: default_hz,
            rmb_frequency_hz: rmb_hz,
            hotkey: hotkey.clone(),
        };

        if let Err(e) = new_config.save() {
            self.error_label.set_text(&format!("{} {e}", self.strings.err_save_failed));
            return;
        }

        let hotkey_vks: Vec<u16> = hotkey.iter().filter_map(|k| config::vk::from_name(k)).collect();
        *self.state.hotkey_vks.lock().unwrap() = hotkey_vks;
        let _ = self.clicker_tx.send(ClickerCommand::ConfigUpdated(new_config.clone()));
        *self.config.borrow_mut() = new_config;

        self.hide_and_show_tray();
    }

    /// Runs on every `AnimationTimer` tick; a cheap no-op when nothing is
    /// being recorded, which is most of the time.
    fn poll_recording(&self) {
        let field = match *self.recording.borrow() {
            Some(f) => f,
            None => return,
        };

        let snapshot = self.state.recording.snapshot();
        let live_text = if snapshot.is_empty() {
            match field {
                FieldId::TargetKey => self.strings.record_prompt_single.to_string(),
                FieldId::Hotkey => self.strings.record_prompt_combo.to_string(),
            }
        } else {
            snapshot.iter().map(|&vk| config::vk::name_for(vk)).collect::<Vec<_>>().join("+")
        };

        match field {
            FieldId::TargetKey => self.target_key_button.set_text(&live_text),
            FieldId::Hotkey => self.hotkey_button.set_text(&live_text),
        }

        if self.state.recording.done.load(Ordering::Relaxed) {
            self.state.recording.stop();
            *self.recording.borrow_mut() = None;
        }
    }

    fn toggle_recording(&self, field: FieldId, target: RecordTarget) {
        let currently = *self.recording.borrow();
        if currently == Some(field) {
            self.state.recording.stop();
            *self.recording.borrow_mut() = None;
            let cfg = self.config.borrow();
            match field {
                FieldId::TargetKey => self.target_key_button.set_text(&cfg.target_key),
                FieldId::Hotkey => self.hotkey_button.set_text(&cfg.hotkey.join("+")),
            }
        } else {
            self.state.recording.begin(target);
            *self.recording.borrow_mut() = Some(field);
        }
    }
}

fn translate_hotkey_error(e: &HotkeyValidationError, s: &'static Strings) -> String {
    match e {
        HotkeyValidationError::TooFewKeys => s.err_too_few_keys.to_string(),
        HotkeyValidationError::TooManyKeys => s.err_too_many_keys.to_string(),
        HotkeyValidationError::UnknownKey(k) => format!("'{k}' {}", s.err_unknown_key),
        HotkeyValidationError::Duplicate(k) => format!("'{k}' {}", s.err_duplicate_key),
        HotkeyValidationError::NoModifier => s.err_no_modifier.to_string(),
        HotkeyValidationError::Forbidden(reason) => format!("{} ({reason})", s.err_forbidden),
    }
}

/// `Icon::source_bin` (load from a memory buffer) requires the
/// "image-decoder" feature; `source_file` doesn't, since it goes through
/// native Win32 icon loading instead. Writing the embedded bytes out once
/// and loading from there keeps single-file deployment without that
/// extra dependency.
fn write_icon_to_temp_file() -> Option<std::path::PathBuf> {
    static ICON_ICO_BYTES: &[u8] = include_bytes!("../assets/icon.ico");
    let path = std::env::temp_dir().join("auto_clicker_icon.ico");
    std::fs::write(&path, ICON_ICO_BYTES).ok()?;
    Some(path)
}

fn centered_position(size: (i32, i32)) -> (i32, i32) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (screen_w, screen_h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    if screen_w <= 0 || screen_h <= 0 {
        return (200, 150);
    }
    (((screen_w - size.0) / 2).max(0), ((screen_h - size.1) / 2).max(0))
}

fn build_ui(
    config: Config,
    state: Arc<InputState>,
    clicker_tx: Sender<ClickerCommand>,
    strings: &'static Strings,
) -> Result<Rc<AppUi>, nwg::NwgError> {
    let mut ui = AppUi {
        window: Default::default(),
        layout: Default::default(),
        target_key_label: Default::default(),
        target_key_button: Default::default(),
        default_freq_label: Default::default(),
        default_freq_input: Default::default(),
        rmb_freq_label: Default::default(),
        rmb_freq_input: Default::default(),
        hotkey_label: Default::default(),
        hotkey_button: Default::default(),
        error_label: Default::default(),
        start_button: Default::default(),
        icon: Default::default(),
        tray_menu: Default::default(),
        tray_open_item: Default::default(),
        tray_exit_item: Default::default(),
        tray: RefCell::new(None),
        recording_timer: Default::default(),
        state,
        clicker_tx,
        strings,
        config: RefCell::new(config),
        recording: RefCell::new(None),
    };

    let icon_path = write_icon_to_temp_file();
    let icon_path_str = icon_path.as_deref().and_then(|p| p.to_str());
    nwg::Icon::builder().source_file(icon_path_str).strict(false).build(&mut ui.icon)?;

    let size = (440, 380);
    let position = centered_position(size);

    nwg::Window::builder()
        .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
        .size(size)
        .position(position)
        .title(strings.window_title)
        .icon(Some(&ui.icon))
        .build(&mut ui.window)?;

    ui.apply_current_theme();

    nwg::Label::builder().text(strings.target_key_label).parent(&ui.window).build(&mut ui.target_key_label)?;
    nwg::Button::builder()
        .text(&ui.config.borrow().target_key.clone())
        .parent(&ui.window)
        .build(&mut ui.target_key_button)?;

    nwg::Label::builder().text(strings.default_freq_label).parent(&ui.window).build(&mut ui.default_freq_label)?;
    nwg::TextInput::builder()
        .text(&format!("{}", ui.config.borrow().default_frequency_hz))
        .parent(&ui.window)
        .build(&mut ui.default_freq_input)?;

    nwg::Label::builder().text(strings.rmb_freq_label).parent(&ui.window).build(&mut ui.rmb_freq_label)?;
    nwg::TextInput::builder()
        .text(&format!("{}", ui.config.borrow().rmb_frequency_hz))
        .parent(&ui.window)
        .build(&mut ui.rmb_freq_input)?;

    nwg::Label::builder().text(strings.hotkey_label).parent(&ui.window).build(&mut ui.hotkey_label)?;
    nwg::Button::builder()
        .text(&ui.config.borrow().hotkey.join("+"))
        .parent(&ui.window)
        .build(&mut ui.hotkey_button)?;

    nwg::Label::builder().text("").parent(&ui.window).build(&mut ui.error_label)?;

    nwg::Button::builder().text(strings.start_button).parent(&ui.window).build(&mut ui.start_button)?;

    nwg::GridLayout::builder()
        .parent(&ui.window)
        .spacing(6)
        .max_row(Some(6))
        .child(0, 0, &ui.target_key_label)
        .child(1, 0, &ui.target_key_button)
        .child(0, 1, &ui.default_freq_label)
        .child(1, 1, &ui.default_freq_input)
        .child(0, 2, &ui.rmb_freq_label)
        .child(1, 2, &ui.rmb_freq_input)
        .child(0, 3, &ui.hotkey_label)
        .child(1, 3, &ui.hotkey_button)
        .child(1, 4, &ui.error_label)
        .child(1, 5, &ui.start_button)
        .build(&ui.layout)?;

    nwg::Menu::builder().popup(true).parent(&ui.window).build(&mut ui.tray_menu)?;
    nwg::MenuItem::builder().text(strings.tray_open_settings).parent(&ui.tray_menu).build(&mut ui.tray_open_item)?;
    nwg::MenuItem::builder().text(strings.tray_exit).parent(&ui.tray_menu).build(&mut ui.tray_exit_item)?;

    nwg::AnimationTimer::builder()
        .parent(&ui.window)
        .interval(Duration::from_millis(50))
        .build(&mut ui.recording_timer)?;

    let ui = Rc::new(ui);

    let evt_ui = Rc::downgrade(&ui);
    let handle_events = move |evt, evt_data, handle| {
        use nwg::Event as E;
        if let Some(ui) = evt_ui.upgrade() {
            match evt {
                E::OnButtonClick => {
                    if &handle == &ui.start_button {
                        ui.on_start();
                    } else if &handle == &ui.target_key_button {
                        ui.toggle_recording(FieldId::TargetKey, RecordTarget::SingleKey);
                        if ui.recording.borrow().is_some() {
                            ui.recording_timer.start();
                        } else {
                            ui.recording_timer.stop();
                        }
                    } else if &handle == &ui.hotkey_button {
                        ui.toggle_recording(FieldId::Hotkey, RecordTarget::Combo);
                        if ui.recording.borrow().is_some() {
                            ui.recording_timer.start();
                        } else {
                            ui.recording_timer.stop();
                        }
                    }
                }
                E::OnWindowClose => {
                    if &handle == &ui.window {
                        // Without this, nwg's default handling destroys the
                        // window after this returns, making it permanently
                        // unusable the next time Open Settings shows it.
                        if let nwg::EventData::OnWindowClose(close_data) = &evt_data {
                            close_data.close(false);
                        }
                        ui.hide_and_show_tray();
                    }
                }
                E::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) | E::OnContextMenu => {
                    let is_tray = ui.tray.borrow().as_ref().map_or(false, |t| &handle == &*t);
                    if is_tray {
                        ui.show_tray_menu();
                    }
                }
                E::OnMenuItemSelected => {
                    if &handle == &ui.tray_open_item {
                        ui.on_open_settings();
                    } else if &handle == &ui.tray_exit_item {
                        // Every thread and handle this app owns is torn
                        // down atomically by the OS on process exit, so
                        // exiting directly here needs no further
                        // coordination with the other threads.
                        std::process::exit(0);
                    }
                }
                E::OnTimerTick => {
                    if &handle == &ui.recording_timer {
                        ui.poll_recording();
                    }
                }
                _ => {}
            }
        }
    };
    nwg::full_bind_event_handler(&ui.window.handle, handle_events);

    Ok(ui)
}

/// Builds the settings window and runs the app's single, permanent event
/// loop. Blocks until `nwg::stop_thread_dispatch()` is called, which this
/// app currently never does -- the tray's Exit calls `std::process::exit`
/// directly instead (see the Exit handler above).
pub fn run(
    config: Config,
    state: Arc<InputState>,
    clicker_tx: Sender<ClickerCommand>,
    strings: &'static Strings,
) -> Result<(), nwg::NwgError> {
    nwg::init()?;
    let _ = nwg::Font::set_global_family("Segoe UI");

    let _ui = build_ui(config, state, clicker_tx, strings)?;

    nwg::dispatch_thread_events();

    Ok(())
}
