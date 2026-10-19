pub mod capture;
pub mod error;
pub mod follow;
pub mod lock;
pub mod prompter;
pub mod session;

use serde::Serialize;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, State};

use session::engine::{Estimate, estimate};
use session::tokenize::tokenize;
use session::{Action, Control, Highlight, ScriptPayload, SessionHandle};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub title: String,
    pub words: u32,
    pub seconds: f64,
}

/// Reading speed until settings arrive.
const DEFAULT_WPM: u32 = 150;

#[tauri::command]
fn session_start(
    app: AppHandle,
    session: State<'_, SessionHandle>,
    markdown: String,
    path: Option<String>,
) -> error::Result<SessionInfo> {
    let script = tokenize(&markdown);
    let title = script
        .title
        .or_else(|| {
            path.as_deref()
                .and_then(|p| std::path::Path::new(p).file_stem())
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Untitled".to_string());
    let estimate = estimate(&script.tokens, DEFAULT_WPM);
    prompter::set_visible(&app, true)?;
    session.send(Control::Start(ScriptPayload {
        title: title.clone(),
        tokens: script.tokens,
        words: estimate.words,
        total: estimate.seconds,
    }));
    Ok(SessionInfo {
        title,
        words: estimate.words,
        seconds: estimate.seconds,
    })
}

#[tauri::command]
fn session_set_lines(session: State<'_, SessionHandle>, ranges: Vec<(u32, u32)>) {
    session.send(Control::SetLines(ranges));
}

#[tauri::command]
fn session_control(
    app: AppHandle,
    session: State<'_, SessionHandle>,
    action: Action,
) -> error::Result<()> {
    if matches!(action, Action::Stop) {
        prompter::set_visible(&app, false)?;
    }
    session.send(Control::Act(action));
    Ok(())
}

#[tauri::command]
fn session_estimate(markdown: String, wpm: u32) -> Estimate {
    estimate(&tokenize(&markdown).tokens, wpm)
}

#[tauri::command]
fn prompter_set_visible(app: AppHandle, visible: bool) -> error::Result<capture::CaptureStatus> {
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
        session_start,
        session_set_lines,
        session_control,
        session_estimate,
        prompter_set_visible,
        capture_status,
        follow_stats
    ]);
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        session_start,
        session_set_lines,
        session_control,
        session_estimate,
        prompter_set_visible,
        capture_status
    ]);
    builder
        .manage(prompter::Prompter::default())
        .setup(|app| {
            let session = session::spawn(app.handle().clone(), DEFAULT_WPM, Highlight::Word)?;
            app.manage(session);
            Ok(())
        })
        .on_page_load(|webview, payload| {
            // The overlay may finish loading after the status and script were first sent.
            if webview.label() == prompter::LABEL && matches!(payload.event(), PageLoadEvent::Finished) {
                let status = webview.state::<prompter::Prompter>().capture_status();
                let _ = webview.emit_to(prompter::LABEL, prompter::CAPTURE_EVENT, status);
                webview.state::<SessionHandle>().send(Control::Resend);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
