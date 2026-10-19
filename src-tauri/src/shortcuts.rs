//! Global shortcuts, registered while a session runs and released at Stop, so the keys
//! belong to other apps when Aside is idle. The table is in docs/PRODUCT.md; Settings
//! rebinds them. The peek key is not a shortcut: it is polled in the follow loop.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{Error, Result};
use crate::lock::lock;
use crate::prompter;
use crate::settings::{Settings, SettingsState};

/// Which shortcuts the OS refused, for the Settings window.
pub const STATUS_EVENT: &str = "shortcuts://status";
/// Faster and Slower change the speed by this much.
pub const WPM_STEP: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutAction {
    /// Pause or Resume, by state.
    Pause,
    Prev,
    Next,
    Faster,
    Slower,
    Restart,
    ToggleHidden,
    Stop,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 8] = [
        ShortcutAction::Pause,
        ShortcutAction::Prev,
        ShortcutAction::Next,
        ShortcutAction::Faster,
        ShortcutAction::Slower,
        ShortcutAction::Restart,
        ShortcutAction::ToggleHidden,
        ShortcutAction::Stop,
    ];

    /// docs/PRODUCT.md defaults. Alt is Option on macOS, so one string serves both.
    pub fn default_accelerator(self) -> &'static str {
        match self {
            ShortcutAction::Pause => "Control+Alt+Space",
            ShortcutAction::Prev => "Control+Alt+ArrowLeft",
            ShortcutAction::Next => "Control+Alt+ArrowRight",
            ShortcutAction::Faster => "Control+Alt+ArrowUp",
            ShortcutAction::Slower => "Control+Alt+ArrowDown",
            ShortcutAction::Restart => "Control+Alt+KeyR",
            ShortcutAction::ToggleHidden => "Control+Alt+KeyH",
            ShortcutAction::Stop => "Control+Alt+Period",
        }
    }
}

pub type Shortcuts = BTreeMap<ShortcutAction, String>;

pub fn defaults() -> Shortcuts {
    ShortcutAction::ALL
        .iter()
        .map(|action| (*action, action.default_accelerator().to_string()))
        .collect()
}

/// Adds any action an older settings file lacks.
pub fn complete(shortcuts: &mut Shortcuts) {
    for action in ShortcutAction::ALL {
        shortcuts
            .entry(action)
            .or_insert_with(|| action.default_accelerator().to_string());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub action: ShortcutAction,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub failed: Vec<Failure>,
}

/// Managed state. `active` is true between Start and Stop.
#[derive(Default)]
pub struct ShortcutsState {
    active: Mutex<bool>,
    failed: Mutex<Vec<Failure>>,
}

impl ShortcutsState {
    pub fn status(&self) -> ShortcutStatus {
        ShortcutStatus {
            failed: lock(&self.failed).clone(),
        }
    }
}

pub fn parse(accelerator: &str) -> Result<Shortcut> {
    accelerator.parse::<Shortcut>().map_err(|_| Error::BadShortcut)
}

/// The plugin flattens the OS error to a string. Windows says "already registered" for a
/// taken key; macOS says "RegisterEventHotKey failed" for any refusal, which is reported
/// as taken too, since that is the one cause a user can fix.
fn classify(error: tauri_plugin_global_shortcut::Error) -> Error {
    let text = error.to_string();
    if text.contains("already registered") || text.contains("RegisterEventHotKey") {
        Error::ShortcutTaken
    } else {
        Error::Shortcut(error)
    }
}

/// Registers every shortcut from the settings. A refused key is recorded and reported,
/// never fatal: the session starts either way. Main thread.
pub fn register_all(app: &AppHandle) -> Result<()> {
    let shortcuts = app.state::<SettingsState>().get().shortcuts;
    let state = app.state::<ShortcutsState>();
    *lock(&state.active) = true;
    let mut failed = Vec::new();
    for (action, accelerator) in &shortcuts {
        if let Err(error) = register_one(app, *action, accelerator) {
            failed.push(Failure {
                action: *action,
                message: error.to_string(),
            });
        }
    }
    *lock(&state.failed) = failed;
    emit_status(app);
    Ok(())
}

/// Releases every shortcut. The failure list stays, so Settings can still show it.
pub fn unregister_all(app: &AppHandle) {
    *lock(&app.state::<ShortcutsState>().active) = false;
    let _ = app.global_shortcut().unregister_all();
}

fn register_one(app: &AppHandle, action: ShortcutAction, accelerator: &str) -> Result<Shortcut> {
    let shortcut = parse(accelerator)?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                // Window and panel calls belong on the main thread.
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let _ = crate::perform(&handle, action);
                });
            }
        })
        .map_err(classify)?;
    Ok(shortcut)
}

/// Rebinds one action. While a session runs the new key replaces the old one at once; when
/// idle it is tried once and released, so a taken key is caught before it is saved.
pub fn rebind(app: &AppHandle, action: ShortcutAction, accelerator: &str) -> Result<Settings> {
    let shortcut = parse(accelerator)?;
    let settings = app.state::<SettingsState>();
    let state = app.state::<ShortcutsState>();
    let current = settings.get().shortcuts;
    if current.iter().any(|(other, bound)| *other != action && bound == accelerator) {
        return Err(Error::ShortcutTaken);
    }
    if *lock(&state.active) {
        let old = current.get(&action).and_then(|bound| parse(bound).ok());
        if let Some(old) = old {
            let _ = app.global_shortcut().unregister(old);
        }
        if let Err(error) = register_one(app, action, accelerator) {
            if let Some(bound) = current.get(&action) {
                let _ = register_one(app, action, bound);
            }
            return Err(error);
        }
        lock(&state.failed).retain(|failure| failure.action != action);
        emit_status(app);
    } else {
        app.global_shortcut().register(shortcut).map_err(classify)?;
        let _ = app.global_shortcut().unregister(shortcut);
    }
    settings.modify(|s| {
        s.shortcuts.insert(action, accelerator.to_string());
    })
}

/// Restores one action, or all of them, to the defaults.
pub fn reset(app: &AppHandle, action: Option<ShortcutAction>) -> Result<Settings> {
    let settings = app.state::<SettingsState>();
    let after = settings.modify(|s| match action {
        Some(action) => {
            s.shortcuts.insert(action, action.default_accelerator().to_string());
        }
        None => s.shortcuts = defaults(),
    })?;
    if *lock(&app.state::<ShortcutsState>().active) {
        let _ = app.global_shortcut().unregister_all();
        register_all(app)?;
    }
    Ok(after)
}

fn emit_status(app: &AppHandle) {
    let status = app.state::<ShortcutsState>().status();
    let _ = app.emit_to(prompter::SETTINGS, STATUS_EVENT, status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_parses_and_every_action_has_one() {
        let defaults = defaults();
        assert_eq!(defaults.len(), ShortcutAction::ALL.len());
        for (action, accelerator) in &defaults {
            assert!(parse(accelerator).is_ok(), "{action:?}: {accelerator}");
        }
    }

    #[test]
    fn complete_fills_missing_actions_and_keeps_custom_ones() {
        let mut shortcuts = Shortcuts::new();
        shortcuts.insert(ShortcutAction::Stop, "Control+Alt+KeyS".into());
        complete(&mut shortcuts);
        assert_eq!(shortcuts.len(), ShortcutAction::ALL.len());
        assert_eq!(shortcuts[&ShortcutAction::Stop], "Control+Alt+KeyS");
        assert_eq!(shortcuts[&ShortcutAction::Pause], "Control+Alt+Space");
    }

    #[test]
    fn a_bare_modifier_or_nonsense_is_a_bad_shortcut() {
        assert!(matches!(parse("Control"), Err(Error::BadShortcut)));
        assert!(matches!(parse("Ctrl+Alt+NoSuchKey"), Err(Error::BadShortcut)));
    }

    #[test]
    fn refusals_from_both_platforms_read_as_taken() {
        use tauri_plugin_global_shortcut::Error as PluginError;
        let windows = PluginError::GlobalHotkey("HotKey already registered: HotKey {}".into());
        let macos = PluginError::GlobalHotkey("Unable to register hotkey: RegisterEventHotKey failed for Space".into());
        let other = PluginError::GlobalHotkey("something else".into());
        assert!(matches!(classify(windows), Error::ShortcutTaken));
        assert!(matches!(classify(macos), Error::ShortcutTaken));
        assert!(matches!(classify(other), Error::Shortcut(_)));
    }

    #[test]
    fn actions_serialize_as_camel_case_map_keys() {
        let json = serde_json::to_value(defaults()).expect("serialize");
        assert_eq!(json["toggleHidden"], "Control+Alt+KeyH");
        let back: Shortcuts = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, defaults());
    }
}
