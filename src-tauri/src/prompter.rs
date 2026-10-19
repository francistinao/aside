//! The prompter window: created once from its JSON config, shown and hidden on request.
//! Creation, capture exclusion and the follow thread all start from the main thread.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::capture::{self, CaptureStatus};
use crate::error::{Error, Result};
use crate::follow::placement::{Corner, Size};
use crate::follow::{self, FollowConfig, FollowHandle, FollowStats};
use crate::lock::lock;
use crate::settings::SettingsState;

pub const LABEL: &str = "prompter";
pub const LIBRARY: &str = "library";
pub const CAPTURE_EVENT: &str = "capture://status";
/// Windows that show the capture status.
const CAPTURE_LISTENERS: [&str; 2] = [LIBRARY, LABEL];

/// App state for the prompter. Managed by the Tauri builder.
#[derive(Default)]
pub struct Prompter {
    capture: Mutex<CaptureStatus>,
    follow: Mutex<Option<FollowHandle>>,
}

impl Prompter {
    pub fn capture_status(&self) -> CaptureStatus {
        *lock(&self.capture)
    }

    pub fn follow_stats(&self) -> FollowStats {
        lock(&self.follow)
            .as_ref()
            .map(FollowHandle::stats)
            .unwrap_or_default()
    }
}

/// Shows or hides the bubble. Each show re-applies capture exclusion and reports the result.
/// Call on the main thread: sync commands already run there.
pub fn set_visible(app: &AppHandle, visible: bool) -> Result<CaptureStatus> {
    let state = app.state::<Prompter>();
    let window = match app.get_webview_window(LABEL) {
        Some(window) => window,
        None => create(app, &state)?,
    };
    #[cfg(windows)]
    crate::topmost::set_visible(visible);
    if visible {
        show(app, &window)?;
        let hide = app.state::<SettingsState>().get().hide_from_capture;
        let status = apply_capture(app, &state, &window, hide);
        if let Some(follow) = lock(&state.follow).as_ref() {
            follow.set_active(true);
        }
        Ok(status)
    } else {
        if let Some(follow) = lock(&state.follow).as_ref() {
            follow.set_active(false);
        }
        hide(app, &window)?;
        Ok(state.capture_status())
    }
}

/// Turns capture exclusion on or off for an existing bubble and reports the result.
pub fn set_capture(app: &AppHandle, enabled: bool) -> Result<CaptureStatus> {
    let state = app.state::<Prompter>();
    match app.get_webview_window(LABEL) {
        Some(window) => Ok(apply_capture(app, &state, &window, enabled)),
        None => Ok(state.capture_status()),
    }
}

fn apply_capture(app: &AppHandle, state: &Prompter, window: &WebviewWindow, enabled: bool) -> CaptureStatus {
    let status = capture::apply(window, enabled);
    *lock(&state.capture) = status;
    for label in CAPTURE_LISTENERS {
        let _ = app.emit_to(label, CAPTURE_EVENT, status);
    }
    status
}

/// Resizes the window to the overlay's layout. The follow loop places the new size.
pub fn resize(app: &AppHandle, width: f64, height: f64) -> Result<()> {
    let state = app.state::<Prompter>();
    if let Some(window) = app.get_webview_window(LABEL) {
        window.set_size(LogicalSize::new(width, height))?;
    }
    if let Some(follow) = lock(&state.follow).as_ref() {
        follow.set_bubble(Size { w: width, h: height });
    }
    Ok(())
}

pub fn set_corner(app: &AppHandle, corner: Corner) {
    if let Some(follow) = lock(&app.state::<Prompter>().follow).as_ref() {
        follow.set_corner(corner);
    }
}

fn create(app: &AppHandle, state: &Prompter) -> Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == LABEL)
        .ok_or(Error::MissingWindow(LABEL))?;
    let window = WebviewWindowBuilder::from_config(app, config)?.build()?;
    window.set_ignore_cursor_events(true)?;
    #[cfg(target_os = "macos")]
    capture::macos::setup_panel(&window)?;
    #[cfg(windows)]
    crate::topmost::install(window.hwnd()?);
    let bubble = Size {
        w: config.width,
        h: config.height,
    };
    let corner = app.state::<SettingsState>().get().corner;
    let handle = follow::spawn(app, &window, FollowConfig::default(), corner, bubble)?;
    *lock(&state.follow) = Some(handle);
    Ok(window)
}

#[cfg(target_os = "macos")]
fn show(app: &AppHandle, _window: &WebviewWindow) -> Result<()> {
    panel(app)?.order_front_regardless();
    Ok(())
}

#[cfg(target_os = "macos")]
fn hide(app: &AppHandle, _window: &WebviewWindow) -> Result<()> {
    panel(app)?.hide();
    Ok(())
}

#[cfg(target_os = "macos")]
fn panel(app: &AppHandle) -> Result<tauri_nspanel::PanelHandle<tauri::Wry>> {
    use tauri_nspanel::ManagerExt;
    app.get_webview_panel(LABEL)
        .map_err(|_| Error::MissingWindow(LABEL))
}

#[cfg(not(target_os = "macos"))]
fn show(_app: &AppHandle, window: &WebviewWindow) -> Result<()> {
    window.show()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn hide(_app: &AppHandle, window: &WebviewWindow) -> Result<()> {
    window.hide()?;
    Ok(())
}
