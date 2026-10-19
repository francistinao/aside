pub mod capture;
pub mod error;
pub mod follow;
pub mod lock;
pub mod menu;
pub mod prompter;
pub mod session;
pub mod settings;
pub mod vault;

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

use session::engine::{Estimate, estimate};
use session::tokenize::tokenize;
use session::{Action, Control, ScriptPayload, SessionHandle};
use settings::{Settings, SettingsPatch, SettingsState};
use vault::index::Hit;
use vault::{ChosenFolder, ConflictChoice, FileData, Tree, Vault, VaultStatus};

/// What the Library needs to know about the platform before it paints.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub os: &'static str,
    /// True when the window has a real backdrop: vibrancy on macOS, Mica on Windows 11.
    pub backdrop: bool,
}

#[tauri::command]
fn platform_info() -> PlatformInfo {
    #[cfg(target_os = "macos")]
    {
        PlatformInfo {
            os: "macos",
            backdrop: true,
        }
    }
    #[cfg(windows)]
    {
        PlatformInfo {
            os: "windows",
            backdrop: windows_version::OsVersion::current().build >= 22000,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub title: String,
    pub words: u32,
    pub seconds: f64,
}

#[tauri::command]
fn session_start(
    app: AppHandle,
    settings: State<'_, SettingsState>,
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
    let estimate = estimate(&script.tokens, settings.get().wpm);
    let payload = ScriptPayload {
        title: title.clone(),
        tokens: script.tokens,
        words: estimate.words,
        total: estimate.seconds,
    };
    let info = SessionInfo {
        title,
        words: estimate.words,
        seconds: estimate.seconds,
    };
    #[cfg(target_os = "macos")]
    if !settings.get().capture_notice_shown {
        // Once, before the first session: macOS cannot promise to keep the bubble out of a
        // full screen share. Copy from docs/INVISIBILITY.md. The session starts after OK.
        use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
        settings.modify(|s| s.capture_notice_shown = true)?;
        let after = app.clone();
        app.dialog()
            .message("macOS may include the Prompter when you share your entire screen. Share a single window or tab to keep it out.")
            .title("Aside")
            .buttons(MessageDialogButtons::Ok)
            .show(move |_| {
                let on_main = after.clone();
                let _ = after.run_on_main_thread(move || {
                    let _ = begin_session(&on_main, payload);
                });
            });
        return Ok(info);
    }
    begin_session(&app, payload)?;
    Ok(info)
}

/// Shows the bubble and hands the script to the session thread. Main thread only.
fn begin_session(app: &AppHandle, payload: ScriptPayload) -> error::Result<()> {
    prompter::set_visible(app, true)?;
    app.state::<SessionHandle>().send(Control::Start(payload));
    Ok(())
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
fn vault_status(vault: State<'_, Vault>) -> VaultStatus {
    vault.status()
}

/// Async so the blocking native picker stays off the main thread.
#[tauri::command]
async fn vault_choose_folder(app: AppHandle) -> error::Result<Option<ChosenFolder>> {
    app.state::<Vault>().choose_folder(&app)
}

#[tauri::command]
fn vault_tree(vault: State<'_, Vault>, settings: State<'_, SettingsState>) -> error::Result<Tree> {
    vault.tree(&settings)
}

#[tauri::command]
fn library_pin(
    vault: State<'_, Vault>,
    settings: State<'_, SettingsState>,
    path: String,
    pinned: bool,
) -> error::Result<Vec<String>> {
    vault.set_pinned(&settings, &path, pinned)
}

/// Async: the native dialog blocks until the user answers.
#[tauri::command]
async fn vault_conflict_dialog(app: AppHandle, name: String) -> ConflictChoice {
    Vault::conflict_dialog(&app, &name)
}

#[tauri::command]
fn library_popup_menu(app: AppHandle, kind: menu::PopupKind, x: f64, y: f64) -> error::Result<()> {
    menu::popup_menu(&app, kind, x, y)
}

#[tauri::command]
fn library_context_menu(
    app: AppHandle,
    path: String,
    target: menu::ContextTarget,
    x: f64,
    y: f64,
) -> error::Result<()> {
    menu::context_menu(&app, path, target, x, y)
}

#[tauri::command]
fn vault_reveal(vault: State<'_, Vault>, path: String) -> error::Result<()> {
    vault.reveal(&path)
}

#[tauri::command]
fn vault_search(vault: State<'_, Vault>, query: String) -> Vec<Hit> {
    vault.search(&query)
}

/// Opening a script records it in recents. It never touches the file.
#[tauri::command]
fn vault_read(
    vault: State<'_, Vault>,
    settings: State<'_, SettingsState>,
    path: String,
) -> error::Result<FileData> {
    let data = vault.read(&path)?;
    vault.record_recent(&settings, &path)?;
    Ok(data)
}

#[tauri::command]
fn vault_write(vault: State<'_, Vault>, path: String, content: String) -> error::Result<FileData> {
    vault.write(&path, &content)
}

#[tauri::command]
fn vault_create(vault: State<'_, Vault>, dir: String) -> error::Result<String> {
    vault.create(&dir)
}

#[tauri::command]
fn vault_create_folder(vault: State<'_, Vault>, dir: String) -> error::Result<String> {
    vault.create_folder(&dir)
}

#[tauri::command]
fn vault_rename(vault: State<'_, Vault>, path: String, new_name: String) -> error::Result<String> {
    vault.rename(&path, &new_name)
}

#[tauri::command]
fn vault_trash(vault: State<'_, Vault>, path: String) -> error::Result<()> {
    vault.trash(&path)
}

pub const FLUSH_EVENT: &str = "library://flush";

/// Why the Library was asked to save: the window is closing or the app is quitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlushReason {
    Close,
    Quit,
}

#[derive(Clone, Copy, Serialize)]
struct FlushPayload {
    then: FlushReason,
}

/// Set once the Library has saved and the exit may proceed.
#[derive(Default)]
pub struct Quitting(AtomicBool);

fn ask_to_flush(app: &AppHandle, then: FlushReason) {
    if app.emit_to(prompter::LIBRARY, FLUSH_EVENT, FlushPayload { then }).is_err() {
        finish_flush(app, then);
    }
}

/// Close Window hides the Library on macOS and keeps the app in the Dock. On Windows the
/// window is the app, so closing it quits.
fn finish_flush(app: &AppHandle, then: FlushReason) {
    let quit = then == FlushReason::Quit || !cfg!(target_os = "macos");
    if quit {
        app.state::<Quitting>().0.store(true, Ordering::SeqCst);
        app.exit(0);
    } else if let Some(window) = app.get_webview_window(prompter::LIBRARY) {
        let _ = window.hide();
    }
}

#[tauri::command]
fn library_flushed(app: AppHandle, then: FlushReason) {
    finish_flush(&app, then);
}

#[tauri::command]
fn settings_get(settings: State<'_, SettingsState>) -> Settings {
    settings.get()
}

#[tauri::command]
fn settings_set(
    app: AppHandle,
    settings: State<'_, SettingsState>,
    patch: SettingsPatch,
) -> error::Result<Settings> {
    let (before, after) = settings.update(patch)?;
    settings::apply(&app, &before, &after)?;
    Ok(after)
}

#[tauri::command]
fn prompter_resize(app: AppHandle, width: f64, height: f64) -> error::Result<()> {
    prompter::resize(&app, width, height)
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
    let start = std::time::Instant::now();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&[prompter::LABEL])
                .build(),
        );
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        session_start,
        session_set_lines,
        session_control,
        session_estimate,
        vault_status,
        vault_choose_folder,
        vault_tree,
        vault_reveal,
        vault_search,
        library_pin,
        library_context_menu,
        library_popup_menu,
        vault_conflict_dialog,
        platform_info,
        library_flushed,
        vault_read,
        vault_write,
        vault_create,
        vault_create_folder,
        vault_rename,
        vault_trash,
        settings_get,
        settings_set,
        prompter_resize,
        capture_status,
        follow_stats
    ]);
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        session_start,
        session_set_lines,
        session_control,
        session_estimate,
        vault_status,
        vault_choose_folder,
        vault_tree,
        vault_reveal,
        vault_search,
        library_pin,
        library_context_menu,
        library_popup_menu,
        vault_conflict_dialog,
        platform_info,
        library_flushed,
        vault_read,
        vault_write,
        vault_create,
        vault_create_folder,
        vault_rename,
        vault_trash,
        settings_get,
        settings_set,
        prompter_resize,
        capture_status
    ]);
    builder
        .manage(prompter::Prompter::default())
        .manage(menu::MenuContext::default())
        .manage(Quitting::default())
        .on_menu_event(menu::handle)
        .on_window_event(|window, event| {
            if window.label() == prompter::LIBRARY
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                ask_to_flush(window.app_handle(), FlushReason::Close);
            }
        })
        .setup(|app| {
            // Settings are read before anything can show, so nothing flashes a default.
            let settings = SettingsState::load(app.handle())?;
            let current = settings.get();
            app.manage(settings);
            #[cfg(target_os = "macos")]
            app.set_menu(menu::app_menu(app.handle())?)?;
            let vault = Vault::new(current.scripts_folder.as_deref().map(std::path::PathBuf::from));
            // A missing folder is reported to the Library on its first call, not here.
            let _ = vault.start_watching(app.handle());
            app.manage(vault);
            let session = session::spawn(app.handle().clone(), current.wpm, current.highlight)?;
            app.manage(session);
            Ok(())
        })
        .on_page_load(move |webview, payload| {
            // ASIDE_MEASURE_STARTUP=1 prints the time from run() to the Library's page load
            // on stdout and quits. docs/PERFORMANCE.md explains how to read it.
            if webview.label() == prompter::LIBRARY
                && matches!(payload.event(), PageLoadEvent::Finished)
                && std::env::var_os("ASIDE_MEASURE_STARTUP").is_some()
            {
                println!("startup {} ms", start.elapsed().as_millis());
                std::process::exit(0);
            }
            // The overlay may finish loading after the status, style and script were first sent.
            if webview.label() == prompter::LABEL && matches!(payload.event(), PageLoadEvent::Finished) {
                let status = webview.state::<prompter::Prompter>().capture_status();
                let _ = webview.emit_to(prompter::LABEL, prompter::CAPTURE_EVENT, status);
                let _ = settings::emit_style(webview.app_handle(), &webview.state::<SettingsState>().get());
                webview.state::<SessionHandle>().send(Control::Resend);
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // A quit the user asked for waits until the Library has saved.
            RunEvent::ExitRequested { api, code, .. } => {
                if code.is_none() && !app.state::<Quitting>().0.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    ask_to_flush(app, FlushReason::Quit);
                }
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => {
                if let Some(window) = app.get_webview_window(prompter::LIBRARY) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        });
}
