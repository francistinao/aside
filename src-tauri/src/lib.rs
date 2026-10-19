pub mod capture;
pub mod error;
pub mod follow;
pub mod lock;
pub mod prompter;
pub mod session;
pub mod settings;
pub mod vault;

use serde::Serialize;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, State};

use session::engine::{Estimate, estimate};
use session::tokenize::tokenize;
use session::{Action, Control, ScriptPayload, SessionHandle};
use settings::{Settings, SettingsPatch, SettingsState};
use vault::{Entry, FileData, Vault};

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
    session: State<'_, SessionHandle>,
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
fn vault_tree(vault: State<'_, Vault>) -> error::Result<Vec<Entry>> {
    vault.entries()
}

#[tauri::command]
fn vault_read(vault: State<'_, Vault>, path: String) -> error::Result<FileData> {
    vault.read(&path)
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
    let builder = tauri::Builder::default().plugin(tauri_plugin_store::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        session_start,
        session_set_lines,
        session_control,
        session_estimate,
        vault_tree,
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
        vault_tree,
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
        .setup(|app| {
            // Settings are read before anything can show, so nothing flashes a default.
            let settings = SettingsState::load(app.handle())?;
            let current = settings.get();
            app.manage(settings);
            app.manage(Vault::new(current.scripts_folder.as_deref().map(std::path::PathBuf::from)));
            let session = session::spawn(app.handle().clone(), current.wpm, current.highlight)?;
            app.manage(session);
            Ok(())
        })
        .on_page_load(|webview, payload| {
            // The overlay may finish loading after the status, style and script were first sent.
            if webview.label() == prompter::LABEL && matches!(payload.event(), PageLoadEvent::Finished) {
                let status = webview.state::<prompter::Prompter>().capture_status();
                let _ = webview.emit_to(prompter::LABEL, prompter::CAPTURE_EVENT, status);
                let _ = settings::emit_style(webview.app_handle(), &webview.state::<SettingsState>().get());
                webview.state::<SessionHandle>().send(Control::Resend);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
