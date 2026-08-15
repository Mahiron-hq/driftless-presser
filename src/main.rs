// Suppresses the console window that a plain Rust binary would otherwise
// pop up alongside the GUI on Windows.
#![windows_subsystem = "windows"]

#[cfg(not(windows))]
compile_error!(
    "This application only supports Windows 10/11: it relies on native \
     Win32 APIs (low-level input hooks, SendInput, and the system tray) \
     that have no equivalent on other platforms."
);

#[cfg(windows)]
fn main() {
    use auto_clicker::clicker::windows_impl::{self, ClickerCommand};
    use auto_clicker::config::{vk, Config};
    use auto_clicker::gui;
    use auto_clicker::hook::{self, InputState};
    use auto_clicker::i18n::Lang;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    // No console for a panic's default message to go to, so route every
    // panic through a real dialog instead. Still fires under panic=abort:
    // the hook always runs before the abort happens.
    std::panic::set_hook(Box::new(|info| {
        auto_clicker::fatal_error(&format!("The application hit an unexpected error and needs to close:\n\n{info}"));
    }));

    let config = Config::load_or_init();
    let strings = Lang::detect().strings();

    let hotkey_vks: Vec<u16> = config.hotkey.iter().filter_map(|k| vk::from_name(k)).collect();
    let state = Arc::new(InputState::new(hotkey_vks));

    let (clicker_tx, clicker_rx) = mpsc::channel::<ClickerCommand>();

    // Pure Win32, no GUI-toolkit dependency (see hook.rs). Reports its
    // thread ID back only so early failure can be detected and handled
    // cleanly instead of waiting forever for an ID that isn't coming.
    let (tid_tx, tid_rx) = mpsc::channel::<u32>();
    let hook_state = Arc::clone(&state);
    let hook_clicker_tx = clicker_tx.clone();
    let hook_thread = thread::Builder::new()
        .name("input-hook".into())
        .spawn(move || hook::run(hook_state, hook_clicker_tx, tid_tx))
        .expect("failed to spawn input-hook thread");
    if tid_rx.recv().is_err() {
        return; // hook thread already showed its own fatal_error and exited
    }

    let clicker_state = Arc::clone(&state);
    let clicker_config = config.clone();
    let clicker_thread = thread::Builder::new()
        .name("clicker".into())
        .spawn(move || windows_impl::run(clicker_state, clicker_rx, clicker_config))
        .expect("failed to spawn clicker thread");

    // Runs on the main thread for the app's lifetime (see gui.rs). Quitting
    // happens via process::exit in the tray's Exit handler, not by this
    // call returning; the joins below are a fallback for the unlikely case
    // it does return some other way.
    if let Err(e) = gui::run(config, state, clicker_tx, strings) {
        auto_clicker::fatal_error(&format!("Failed to start the user interface:\n\n{e}"));
    }

    let _ = clicker_thread.join();
    let _ = hook_thread.join();
}
