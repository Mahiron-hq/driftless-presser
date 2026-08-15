//! System theme detection and application (Windows-only).
//!
//! Only the title bar is re-themed, not client-area controls: properly
//! recoloring native Win32 controls needs `WM_CTLCOLOR*` handling and GDI
//! brush lifetime management, and pushbuttons specifically need full
//! owner-draw since `WM_CTLCOLORBTN` doesn't affect their background. The
//! title bar is one documented DWM call instead, and the most visible
//! theme signal an app can give.
//!
//! Uses raw `winapi` calls rather than the `winreg` crate to avoid
//! reintroducing `windows-sys` into the dependency graph (see hook.rs).

use std::ptr::null_mut;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, HKEY, TRUE};
use winapi::shared::windef::HWND;
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::winnt::{KEY_READ, REG_DWORD};
use winapi::um::winreg::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// Reads Windows' "app mode" setting from the registry; falls back to
    /// `Dark` if it's never been written (Settings > Colors never opened).
    pub fn detect() -> Self {
        match read_apps_use_light_theme() {
            Some(1) => ThemeMode::Light,
            _ => ThemeMode::Dark,
        }
    }

    /// Applies this mode to a window's title bar via
    /// `DwmSetWindowAttribute`. Best-effort: failure (e.g. no DWM
    /// composition) just leaves the title bar unthemed.
    pub fn apply_to_title_bar(self, hwnd: HWND) {
        // Numbered 20 on Windows 10 2004+ and Windows 11, but 19 on earlier
        // Windows 10 -- try 20 first, fall back to 19, rather than parsing
        // the OS build number.
        const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
        const DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1: DWORD = 19;

        let value: BOOL = if self == ThemeMode::Dark { TRUE } else { FALSE };
        let size = std::mem::size_of::<BOOL>() as DWORD;

        unsafe {
            let hr = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &value as *const BOOL as *const _,
                size,
            );
            if hr < 0 {
                // FAILED(hr): error codes have the high bit set, so they're
                // negative as a signed 32-bit value.
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE_BEFORE_20H1,
                    &value as *const BOOL as *const _,
                    size,
                );
            }
        }
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn read_apps_use_light_theme() -> Option<DWORD> {
    let subkey = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = to_wide("AppsUseLightTheme");

    unsafe {
        let mut key: HKEY = null_mut();
        // LSTATUS signedness conventions vary across Win32 headers;
        // comparing against 0 avoids needing to pick one just for this.
        let opened = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key);
        if opened != 0 {
            return None;
        }

        let mut value_type: DWORD = 0;
        let mut data: DWORD = 0;
        let mut data_size: DWORD = std::mem::size_of::<DWORD>() as DWORD;

        let queried = RegQueryValueExW(
            key,
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            &mut data as *mut DWORD as *mut u8,
            &mut data_size,
        );

        RegCloseKey(key);

        if queried == 0 && value_type == REG_DWORD {
            Some(data)
        } else {
            None
        }
    }
}
