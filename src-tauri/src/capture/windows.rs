//! Windows: `SetWindowDisplayAffinity` with a build check and a readback, so the status is real.

use tauri::WebviewWindow;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowDisplayAffinity, SM_REMOTESESSION, SetWindowDisplayAffinity,
    WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

use super::CaptureStatus;

/// First Windows 10 build with `WDA_EXCLUDEFROMCAPTURE`. Older builds silently fall back to
/// `WDA_MONITOR`, which paints a black box into captures, so we do not set it there.
const MIN_BUILD: u32 = 19041;

pub fn apply(window: &WebviewWindow, enabled: bool) -> CaptureStatus {
    let Ok(hwnd) = window.hwnd() else {
        return CaptureStatus::Failed;
    };
    apply_to_hwnd(hwnd, enabled)
}

fn apply_to_hwnd(hwnd: HWND, enabled: bool) -> CaptureStatus {
    if !enabled {
        // SAFETY: hwnd is our own top-level prompter window.
        let _ = unsafe { SetWindowDisplayAffinity(hwnd, WDA_NONE) };
        return CaptureStatus::Visible;
    }
    if windows_version::OsVersion::current().build < MIN_BUILD {
        return CaptureStatus::Unsupported;
    }
    // SAFETY: as above.
    if unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) }.is_err() {
        return CaptureStatus::Failed;
    }
    let mut current = 0u32;
    // SAFETY: as above. `current` is a valid out pointer for the duration of the call.
    let read_back = unsafe { GetWindowDisplayAffinity(hwnd, &mut current) };
    if read_back.is_err() || current != WDA_EXCLUDEFROMCAPTURE.0 {
        return CaptureStatus::Failed;
    }
    // Remote Desktop sessions are reported to show the window anyway.
    // SAFETY: GetSystemMetrics has no preconditions.
    if unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0 {
        return CaptureStatus::MayBeVisible;
    }
    CaptureStatus::Hidden
}
