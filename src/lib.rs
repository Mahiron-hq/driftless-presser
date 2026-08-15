//! Auto-clicker library crate.
//!
//! `config` and `clicker::ClickScheduler` are platform-independent and unit
//! tested on any host; `hook`, `gui`, and `theme` wrap Win32 APIs and only
//! compile on Windows.

pub mod clicker;
pub mod config;
pub mod i18n;

#[cfg(windows)]
pub mod hook;

#[cfg(windows)]
pub mod gui;

#[cfg(windows)]
pub mod theme;

/// Shows a native message box and blocks until it's dismissed.
///
/// Used as the global panic hook and by fallible startup steps, since this
/// app has no console for a panic message to print to otherwise.
#[cfg(windows)]
pub fn fatal_error(message: &str) {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide_msg: Vec<u16> = OsStr::new(message).encode_wide().chain(once(0)).collect();
    let wide_title: Vec<u16> = OsStr::new("Auto-Clicker - Error").encode_wide().chain(once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
