//! Keeping the bubble out of captures, and being honest about it. See docs/INVISIBILITY.md.

use serde::Serialize;
use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

/// What the OS let us verify. Only `Hidden` is a promise, and only Windows can make it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureStatus {
    Hidden,
    MayBeVisible,
    #[default]
    Visible,
    Unsupported,
    Failed,
}

/// Turns capture exclusion on or off for the prompter window and reports what really happened.
/// Call it on the main thread, and again after anything that changes the window's styles.
pub fn apply(window: &WebviewWindow, enabled: bool) -> CaptureStatus {
    #[cfg(target_os = "macos")]
    {
        macos::apply(window, enabled)
    }
    #[cfg(windows)]
    {
        windows::apply(window, enabled)
    }
}
