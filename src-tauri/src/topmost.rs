//! Windows: keep the bubble above other topmost windows. A slide show that activates can
//! cover a topmost window, so every foreground change re-asserts `HWND_TOPMOST` without
//! moving, resizing or activating anything. Exclusive full screen games cannot be overlaid.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_SYSTEM_FOREGROUND, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SetWindowPos, WINEVENT_OUTOFCONTEXT,
};

/// The prompter window, 0 before it exists.
static PROMPTER: AtomicIsize = AtomicIsize::new(0);
/// Set by `prompter::set_visible`. A hidden bubble is left alone.
static VISIBLE: AtomicBool = AtomicBool::new(false);
static HOOK: AtomicIsize = AtomicIsize::new(0);

/// Registers the foreground hook for the prompter window. Main thread only: an out of
/// context hook delivers on the thread that set it, and the main thread runs the message loop.
pub fn install(hwnd: HWND) {
    PROMPTER.store(hwnd.0 as isize, Ordering::SeqCst);
    if HOOK.load(Ordering::SeqCst) != 0 {
        return;
    }
    // SAFETY: the callback has the documented signature and touches only atomics and our
    // own window. The hook stays registered until `remove` runs at exit.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    HOOK.store(hook.0 as isize, Ordering::SeqCst);
}

pub fn set_visible(visible: bool) {
    VISIBLE.store(visible, Ordering::SeqCst);
}

/// Unregisters the hook. Called once from the exit event.
pub fn remove() {
    let hook = HOOK.swap(0, Ordering::SeqCst);
    if hook != 0 {
        // SAFETY: the handle came from `SetWinEventHook` on this thread.
        let _ = unsafe { UnhookWinEvent(HWINEVENTHOOK(hook as *mut _)) };
    }
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if !VISIBLE.load(Ordering::SeqCst) {
        return;
    }
    let raw = PROMPTER.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    // SAFETY: our own top-level window. The flags keep its position, size, owner order and
    // activation state; only the z-order changes.
    let _ = unsafe {
        SetWindowPos(
            HWND(raw as *mut _),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )
    };
}
