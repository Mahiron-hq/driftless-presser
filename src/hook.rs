//! Global low-level input hook (Windows-only).
//!
//! Runs on its own thread with a message loop, which low-level hooks
//! require; callbacks stay minimal since Windows drops a hook that's slow
//! to return, delaying system-wide input while it runs.
//!
//! Has no GUI-toolkit dependency -- mixing `winapi` (native-windows-gui's
//! basis) with anything built on `windows-sys` in the same binary
//! previously corrupted the DLL import table (`GetWindowSubclass`
//! conflict) rather than failing to compile.

use crate::clicker::windows_impl::ClickerCommand;
use crate::config::vk;
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use winapi::shared::minwindef::{LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_KEYDOWN, WM_KEYUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Which field is currently being recorded from live physical key input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordTarget {
    /// Finalizes on the very first key press.
    SingleKey,
    /// Finalizes once every key pressed during the session has been
    /// released again.
    Combo,
}

/// Shared state for the settings window's press-to-record UX. The keyboard
/// hook updates it on every physical key event when armed; the GUI polls it
/// on a timer since nothing here pushes updates directly.
pub struct RecordingState {
    pub active_field: Mutex<Option<RecordTarget>>,
    pub captured_keys: Mutex<Vec<u16>>,
    pub done: AtomicBool,
}

impl RecordingState {
    fn new() -> Self {
        RecordingState {
            active_field: Mutex::new(None),
            captured_keys: Mutex::new(Vec::new()),
            done: AtomicBool::new(false),
        }
    }

    pub fn begin(&self, target: RecordTarget) {
        *self.active_field.lock().unwrap() = Some(target);
        self.captured_keys.lock().unwrap().clear();
        self.done.store(false, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        *self.active_field.lock().unwrap() = None;
        self.done.store(false, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Vec<u16> {
        let keys = self.captured_keys.lock().unwrap().clone();
        keys
    }
}

/// Tags this app's own `SendInput` output (via `dwExtraInfo`) so the hook
/// can ignore it without hiding it from other hooks -- `CallNextHookEx`
/// still runs unconditionally either way.
pub const SYNTHETIC_MARKER: usize = 0x53594B43; // "SYKC" in ASCII

/// Cross-thread application state shared by the clicker loop and the GUI.
pub struct InputState {
    pub active: AtomicBool,
    pub rmb_down: AtomicBool,
    /// A `Mutex<Vec<_>>` rather than atomics: it's replaced wholesale on
    /// config changes and read only a few times per second.
    pub hotkey_vks: Mutex<Vec<u16>>,
    pub recording: RecordingState,
}

impl InputState {
    pub fn new(hotkey_vks: Vec<u16>) -> Self {
        InputState {
            active: AtomicBool::new(false),
            rmb_down: AtomicBool::new(false),
            hotkey_vks: Mutex::new(hotkey_vks),
            recording: RecordingState::new(),
        }
    }
}

/// Bridges state into the hook callbacks via thread-local storage, since
/// `HOOKPROC` has no user-data parameter. Scoped to this thread only.
struct HookContext {
    state: Arc<InputState>,
    clicker_tx: Sender<ClickerCommand>,
    kb_hook: HHOOK,
    mouse_hook: HHOOK,
    /// Physical key state; only this thread needs per-key granularity, so
    /// a plain array with no locking is both correct and fast.
    keys_down: [bool; 256],
    /// Tracks the previous event's combo state so the toggle fires once
    /// per press, not continuously while held.
    combo_was_satisfied: bool,
}

thread_local! {
    static CTX: RefCell<Option<HookContext>> = RefCell::new(None);
}

/// Ctrl/Shift/Alt/Win match loosely: configuring "Ctrl" is satisfied by
/// either physical Ctrl key.
fn combo_satisfied(keys_down: &[bool; 256], combo: &[u16]) -> bool {
    !combo.is_empty() && combo.iter().all(|&want| key_is_down(keys_down, want))
}

fn key_is_down(keys_down: &[bool; 256], wanted_vk: u16) -> bool {
    match wanted_vk {
        vk::CONTROL => keys_down[vk::LCONTROL as usize] || keys_down[vk::RCONTROL as usize],
        vk::SHIFT => keys_down[vk::LSHIFT as usize] || keys_down[vk::RSHIFT as usize],
        vk::MENU => keys_down[vk::LMENU as usize] || keys_down[vk::RMENU as usize],
        vk::LWIN => keys_down[vk::LWIN as usize] || keys_down[vk::RWIN as usize],
        other => keys_down[other as usize],
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Per SetWindowsHookEx's documented contract for nCode < 0.
    if code < 0 {
        return forward_kb(code, wparam, lparam);
    }

    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    let vk_code = data.vkCode as u16;
    let msg = wparam as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    let is_our_synthetic_event =
        (data.flags & LLKHF_INJECTED) != 0 && data.dwExtraInfo == SYNTHETIC_MARKER;

    if !is_our_synthetic_event && (is_down || is_up) && (vk_code as usize) < 256 {
        CTX.with(|c| {
            if let Some(ctx) = c.borrow_mut().as_mut() {
                ctx.keys_down[vk_code as usize] = is_down;

                let combo = ctx.state.hotkey_vks.lock().unwrap().clone();
                let satisfied = combo_satisfied(&ctx.keys_down, &combo);
                if satisfied && !ctx.combo_was_satisfied {
                    let _ = ctx.clicker_tx.send(ClickerCommand::ToggleActive);
                }
                ctx.combo_was_satisfied = satisfied;

                // Stays active during recording rather than being
                // suppressed: worst case it harmlessly toggles the old
                // hotkey while a new one is being configured.
                let target = *ctx.state.recording.active_field.lock().unwrap();
                if let Some(target) = target {
                    if is_down {
                        let mut captured = ctx.state.recording.captured_keys.lock().unwrap();
                        if !captured.contains(&vk_code) && captured.len() < 3 {
                            captured.push(vk_code);
                        }
                    }
                    match target {
                        RecordTarget::SingleKey => {
                            if is_down {
                                ctx.state.recording.done.store(true, Ordering::Relaxed);
                            }
                        }
                        RecordTarget::Combo => {
                            if is_up {
                                let captured = ctx.state.recording.captured_keys.lock().unwrap();
                                let all_released =
                                    !captured.is_empty() && captured.iter().all(|&k| !ctx.keys_down[k as usize]);
                                if all_released {
                                    ctx.state.recording.done.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    forward_kb(code, wparam, lparam)
}

fn forward_kb(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    CTX.with(|c| {
        let hook = c.borrow().as_ref().map_or(null_mut(), |ctx| ctx.kb_hook);
        unsafe { CallNextHookEx(hook, code, wparam, lparam) }
    })
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam as u32;
        if msg == WM_RBUTTONDOWN || msg == WM_RBUTTONUP {
            // No self-generated-event filtering needed: this app never
            // synthesizes mouse input.
            CTX.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    ctx.state.rmb_down.store(msg == WM_RBUTTONDOWN, Ordering::Relaxed);
                    // Wakes the clicker thread so the speed change is
                    // instant rather than waiting out the current sleep.
                    let _ = ctx.clicker_tx.send(ClickerCommand::Recheck);
                }
            });
        }
    }

    CTX.with(|c| {
        let hook = c.borrow().as_ref().map_or(null_mut(), |ctx| ctx.mouse_hook);
        unsafe { CallNextHookEx(hook, code, wparam, lparam) }
    })
}

/// Installs both low-level hooks and runs the message loop that keeps them
/// alive; must run on its own dedicated thread. Reports its thread ID
/// through `thread_id_tx` solely so the caller can detect early failure.
///
/// Physical key events are always forwarded via `CallNextHookEx`, never
/// suppressed -- a hotkey prefix that's also a real shortcut elsewhere
/// (e.g. Ctrl+W) still triggers that shortcut while the combo builds up.
pub fn run(state: Arc<InputState>, clicker_tx: Sender<ClickerCommand>, thread_id_tx: Sender<u32>) {
    unsafe {
        let _ = thread_id_tx.send(GetCurrentThreadId());

        let hinstance = GetModuleHandleW(null());
        let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinstance, 0);
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hinstance, 0);

        if kb_hook.is_null() || mouse_hook.is_null() {
            // No console to print to, so a silent failure here would
            // otherwise make the whole process vanish without explanation.
            let err = GetLastError();
            crate::fatal_error(&format!(
                "Couldn't install the global keyboard/mouse hooks (SetWindowsHookExW \
                 failed, Win32 error code {err}).\n\n\
                 This app can't function without them -- it can't detect your hotkey \
                 or watch for the right mouse button. A common cause is antivirus or \
                 endpoint-security software blocking low-level input hooks; some \
                 organization Group Policies also restrict this. Try temporarily \
                 disabling such software (or asking your admin about the policy), \
                 then run the app again."
            ));
            std::process::exit(1);
        }

        CTX.with(|c| {
            *c.borrow_mut() = Some(HookContext {
                state,
                clicker_tx,
                kb_hook,
                mouse_hook,
                keys_down: [false; 256],
                combo_was_satisfied: false,
            });
        });

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(kb_hook);
        UnhookWindowsHookEx(mouse_hook);
    }
}
