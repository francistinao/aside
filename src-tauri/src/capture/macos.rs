//! macOS: set the legacy sharing type and turn the window into a panel.
//! Apple no longer honors `sharingType` in ScreenCaptureKit, so this never reports `Hidden`.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSWindowSharingType, NSWindowStyleMask};
use tauri::WebviewWindow;
use tauri_nspanel::{PanelLevel, WebviewWindowExt};

use super::CaptureStatus;
use crate::error::{Error, Result};
use panel::PrompterPanel;

/// The macro brings its own imports, so it lives in a module of its own.
mod panel {
    use tauri_nspanel::tauri_panel;

    tauri_panel! {
        panel!(PrompterPanel {
            config: {
                can_become_key_window: false,
                can_become_main_window: false,
                is_floating_panel: true,
                hides_on_deactivate: false
            }
        })
    }
}

/// Converts the prompter window into a panel that shows over other apps' full screen Spaces,
/// following Apple's recipe for an auxiliary panel. tauri-nspanel can only convert a built
/// window, so the nonactivating mask is added after the swap through AppKit's public setter;
/// tauri-nspanel's own `set_style_mask` calls a private selector. Content protection is
/// applied again at the end, because the class swap and the style mask change both reset it.
pub fn setup_panel(window: &WebviewWindow) -> Result<()> {
    let panel = window.to_panel::<PrompterPanel>()?;
    let ns_panel = panel.as_panel();
    ns_panel.setStyleMask(ns_panel.styleMask() | NSWindowStyleMask::NonactivatingPanel);
    panel.set_floating_panel(true);
    panel.set_hides_on_deactivate(false);
    panel.set_level(PanelLevel::ScreenSaver.value());
    panel.set_collection_behavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::CanJoinAllApplications
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary,
    );
    panel.set_ignores_mouse_events(true);
    window.set_content_protected(true)?;
    Ok(())
}

pub fn apply(window: &WebviewWindow, enabled: bool) -> CaptureStatus {
    let Some(mtm) = MainThreadMarker::new() else {
        return CaptureStatus::Failed;
    };
    let Ok(ns_window) = ns_window(window, mtm) else {
        return CaptureStatus::Failed;
    };
    if enabled {
        ns_window.setSharingType(NSWindowSharingType::None);
        CaptureStatus::MayBeVisible
    } else {
        ns_window.setSharingType(NSWindowSharingType::ReadOnly);
        CaptureStatus::Visible
    }
}

/// The AppKit window behind a Tauri window. The marker proves we are on the main thread.
pub fn ns_window(window: &WebviewWindow, _mtm: MainThreadMarker) -> Result<Retained<NSWindow>> {
    let ptr = window.ns_window()?.cast::<NSWindow>();
    // SAFETY: Tauri returns the pointer to its own live NSWindow and the window is never
    // destroyed while the app runs. Retaining it makes the reference safe to hold.
    unsafe { Retained::retain(ptr) }.ok_or(Error::MissingWindow("prompter"))
}
