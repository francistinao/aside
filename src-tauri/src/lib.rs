pub mod capture;
pub mod error;
pub mod follow;
pub mod lock;
pub mod prompter;
pub mod session;

use tauri::webview::PageLoadEvent;
use tauri::{Emitter, Manager, State};

#[tauri::command]
fn prompter_set_visible(app: tauri::AppHandle, visible: bool) -> error::Result<capture::CaptureStatus> {
    prompter::set_visible(&app, visible)
}

#[tauri::command]
fn capture_status(prompter: State<'_, prompter::Prompter>) -> capture::CaptureStatus {
    prompter.capture_status()
}

#[cfg(debug_assertions)]
#[tauri::command]
fn follow_stats(prompter: State<'_, prompter::Prompter>) -> follow::FollowStats {
    prompter.follow_stats()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        prompter_set_visible,
        capture_status,
        follow_stats
    ]);
    #[cfg(not(debug_assertions))]
    let builder =
        builder.invoke_handler(tauri::generate_handler![prompter_set_visible, capture_status]);
    builder
        .manage(prompter::Prompter::default())
        .on_page_load(|webview, payload| {
            // The overlay may finish loading after the status was first applied. Send it again.
            if webview.label() == prompter::LABEL && matches!(payload.event(), PageLoadEvent::Finished) {
                let status = webview.state::<prompter::Prompter>().capture_status();
                let _ = webview.emit_to(prompter::LABEL, prompter::CAPTURE_EVENT, status);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
