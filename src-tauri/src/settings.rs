//! Typed settings on top of tauri-plugin-store. Defaults come from docs/PRODUCT.md.
//! A change is saved, applied to the running session and sent to the prompter.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};

use crate::error::Result;
use crate::follow::peek::PeekKey;
use crate::follow::placement::Corner;
use crate::follow::{Distance, Feel, FollowConfig};
use crate::lock::lock;
use crate::prompter;
use crate::session::engine::{MAX_WPM, MIN_WPM};
use crate::session::{Control, Highlight, SessionHandle};
use crate::shortcuts::{self, Shortcuts};

const FILE: &str = "settings.json";
const KEY: &str = "settings";
pub const STYLE_EVENT: &str = "prompter://style";
/// The full settings, sent after every save so every window shows the same values.
pub const CHANGED_EVENT: &str = "settings://changed";
const CHANGED_LISTENERS: [&str; 2] = [prompter::LIBRARY, prompter::SETTINGS];

const WPM_STEP: u32 = 5;
const MIN_OPACITY: u8 = 30;
const MAX_OPACITY: u8 = 100;
const OPACITY_STEP: u8 = 5;

/// Prompter text sizes from docs/DESIGN.md: 18, 20 and 24 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextSize {
    #[default]
    Small,
    Medium,
    Large,
}

const MIN_SIDEBAR: u32 = 200;
const MAX_SIDEBAR: u32 = 320;
pub const MAX_RECENTS: usize = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub wpm: u32,
    pub highlight: Highlight,
    pub opacity: u8,
    pub corner: Corner,
    pub text_size: TextSize,
    pub hide_from_capture: bool,
    pub follow: Feel,
    pub distance: Distance,
    pub flip_at_edges: bool,
    pub peek_key: PeekKey,
    pub fade_while_clicking: bool,
    /// Accelerator per action, changed through the shortcuts commands.
    pub shortcuts: Shortcuts,
    /// Absolute path of the scripts folder, None before first run.
    pub scripts_folder: Option<String>,
    /// Relative paths inside the scripts folder.
    pub pins: Vec<String>,
    /// Relative paths, newest first.
    pub recents: Vec<String>,
    pub sidebar_width: u32,
    pub sidebar_visible: bool,
    pub capture_notice_shown: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wpm: 150,
            highlight: Highlight::Word,
            opacity: 85,
            corner: Corner::BottomRight,
            text_size: TextSize::Small,
            hide_from_capture: true,
            follow: Feel::Smooth,
            distance: Distance::Normal,
            flip_at_edges: true,
            peek_key: PeekKey::Control,
            fade_while_clicking: false,
            shortcuts: shortcuts::defaults(),
            scripts_folder: None,
            pins: Vec::new(),
            recents: Vec::new(),
            sidebar_width: 240,
            sidebar_visible: true,
            capture_notice_shown: false,
        }
    }
}

/// Fields the frontend wants to change. Missing fields stay as they are.
/// The scripts folder, pins and recents change through vault commands instead.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub wpm: Option<u32>,
    pub highlight: Option<Highlight>,
    pub opacity: Option<u8>,
    pub corner: Option<Corner>,
    pub text_size: Option<TextSize>,
    pub hide_from_capture: Option<bool>,
    pub follow: Option<Feel>,
    pub distance: Option<Distance>,
    pub flip_at_edges: Option<bool>,
    pub peek_key: Option<PeekKey>,
    pub fade_while_clicking: Option<bool>,
    pub sidebar_width: Option<u32>,
    pub sidebar_visible: Option<bool>,
    pub capture_notice_shown: Option<bool>,
}

impl Settings {
    fn patched(mut self, patch: SettingsPatch) -> Self {
        if let Some(v) = patch.wpm {
            self.wpm = v;
        }
        if let Some(v) = patch.highlight {
            self.highlight = v;
        }
        if let Some(v) = patch.opacity {
            self.opacity = v;
        }
        if let Some(v) = patch.corner {
            self.corner = v;
        }
        if let Some(v) = patch.text_size {
            self.text_size = v;
        }
        if let Some(v) = patch.hide_from_capture {
            self.hide_from_capture = v;
        }
        if let Some(v) = patch.follow {
            self.follow = v;
        }
        if let Some(v) = patch.distance {
            self.distance = v;
        }
        if let Some(v) = patch.flip_at_edges {
            self.flip_at_edges = v;
        }
        if let Some(v) = patch.peek_key {
            self.peek_key = v;
        }
        if let Some(v) = patch.fade_while_clicking {
            self.fade_while_clicking = v;
        }
        if let Some(v) = patch.sidebar_width {
            self.sidebar_width = v;
        }
        if let Some(v) = patch.sidebar_visible {
            self.sidebar_visible = v;
        }
        if let Some(v) = patch.capture_notice_shown {
            self.capture_notice_shown = v;
        }
        self.normalized()
    }

    /// What the follow thread needs from the settings.
    pub fn follow_config(&self) -> FollowConfig {
        FollowConfig {
            flip_at_edges: self.flip_at_edges,
            ..FollowConfig::new(self.follow, self.distance)
        }
    }

    /// Keeps every value inside the ranges the controls offer.
    fn normalized(mut self) -> Self {
        self.wpm = (self.wpm.clamp(MIN_WPM, MAX_WPM) / WPM_STEP) * WPM_STEP;
        self.opacity = (self.opacity.clamp(MIN_OPACITY, MAX_OPACITY) / OPACITY_STEP) * OPACITY_STEP;
        self.sidebar_width = self.sidebar_width.clamp(MIN_SIDEBAR, MAX_SIDEBAR);
        self.recents.truncate(MAX_RECENTS);
        shortcuts::complete(&mut self.shortcuts);
        self
    }
}

/// What the overlay needs to style itself.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StylePayload {
    pub corner: Corner,
    pub opacity: u8,
    pub highlight: Highlight,
    pub text_size: TextSize,
}

impl From<&Settings> for StylePayload {
    fn from(s: &Settings) -> Self {
        Self {
            corner: s.corner,
            opacity: s.opacity,
            highlight: s.highlight,
            text_size: s.text_size,
        }
    }
}

/// Managed state: the current settings and the store they live in.
pub struct SettingsState {
    current: Mutex<Settings>,
    store: Arc<Store<Wry>>,
    app: AppHandle,
}

impl SettingsState {
    /// Reads the store once. Missing or malformed values fall back to the defaults.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let store = app.store(FILE)?;
        let current = store
            .get(KEY)
            .and_then(|value| serde_json::from_value::<Settings>(value).ok())
            .unwrap_or_default()
            .normalized();
        Ok(Self {
            current: Mutex::new(current),
            store,
            app: app.clone(),
        })
    }

    pub fn get(&self) -> Settings {
        lock(&self.current).clone()
    }

    /// Applies a patch, saves it and returns the old and new values.
    pub fn update(&self, patch: SettingsPatch) -> Result<(Settings, Settings)> {
        let before = self.get();
        let after = before.clone().patched(patch);
        self.replace(after.clone())?;
        Ok((before, after))
    }

    /// Changes settings from Rust (folder, pins, recents) and saves.
    pub fn modify(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let mut after = self.get();
        change(&mut after);
        let after = after.normalized();
        self.replace(after.clone())?;
        Ok(after)
    }

    /// Saves when something changed, then tells every window. The lock is released before
    /// the emit so a listener's own settings call cannot wait on it.
    fn replace(&self, after: Settings) -> Result<()> {
        let changed = {
            let mut current = lock(&self.current);
            if *current == after {
                false
            } else {
                self.store.set(KEY, serde_json::to_value(&after)?);
                self.store.save()?;
                *current = after.clone();
                true
            }
        };
        if changed {
            for label in CHANGED_LISTENERS {
                let _ = self.app.emit_to(label, CHANGED_EVENT, &after);
            }
        }
        Ok(())
    }
}

/// Pushes a change into the running session, the follow loop, capture exclusion and the overlay.
pub fn apply(app: &AppHandle, before: &Settings, after: &Settings) -> Result<()> {
    let session = app.state::<SessionHandle>();
    if after.wpm != before.wpm {
        session.send(Control::SetWpm(after.wpm));
    }
    if after.highlight != before.highlight {
        session.send(Control::SetHighlight(after.highlight));
    }
    if after.corner != before.corner {
        prompter::set_corner(app, after.corner);
    }
    if after.hide_from_capture != before.hide_from_capture {
        prompter::set_capture(app, after.hide_from_capture)?;
    }
    if after.follow_config() != before.follow_config() {
        prompter::set_follow(app, after.follow_config());
    }
    if after.peek_key != before.peek_key || after.fade_while_clicking != before.fade_while_clicking {
        prompter::set_peek(app, after.peek_key, after.fade_while_clicking);
    }
    if StylePayload::from(after) != StylePayload::from(before) {
        emit_style(app, after)?;
    }
    Ok(())
}

pub fn emit_style(app: &AppHandle, settings: &Settings) -> Result<()> {
    app.emit_to(prompter::LABEL, STYLE_EVENT, StylePayload::from(settings))?;
    Ok(())
}

impl PartialEq for StylePayload {
    fn eq(&self, other: &Self) -> bool {
        self.corner == other.corner
            && self.opacity == other.opacity
            && self.highlight == other.highlight
            && self.text_size == other.text_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_changes_only_the_given_fields_and_clamps() {
        let patched = Settings::default().patched(SettingsPatch {
            wpm: Some(999),
            opacity: Some(3),
            text_size: Some(TextSize::Large),
            ..Default::default()
        });
        assert_eq!(patched.wpm, MAX_WPM);
        assert_eq!(patched.opacity, MIN_OPACITY);
        assert_eq!(patched.text_size, TextSize::Large);
        assert_eq!(patched.highlight, Highlight::Word);
        assert_eq!(patched.corner, Corner::BottomRight);
        assert!(patched.hide_from_capture);
    }

    #[test]
    fn follow_settings_become_a_follow_config() {
        let settings = Settings::default().patched(SettingsPatch {
            follow: Some(Feel::Floaty),
            distance: Some(Distance::Far),
            flip_at_edges: Some(false),
            ..Default::default()
        });
        let config = settings.follow_config();
        assert_eq!(config.omega, 9.0);
        assert_eq!(config.distance, 36.0);
        assert!(!config.flip_at_edges);
        assert_eq!(Settings::default().follow_config(), FollowConfig::default());
    }

    #[test]
    fn values_snap_to_their_steps() {
        let patched = Settings::default().patched(SettingsPatch {
            wpm: Some(163),
            opacity: Some(77),
            ..Default::default()
        });
        assert_eq!((patched.wpm, patched.opacity), (160, 75));
    }

    #[test]
    fn settings_round_trip_through_json_with_camel_case() {
        let json = serde_json::to_value(Settings::default()).expect("serialize");
        assert_eq!(json["sidebarWidth"], 240);
        assert!(json["scriptsFolder"].is_null());
        assert_eq!(json["textSize"], "small");
        assert_eq!(json["hideFromCapture"], true);
        assert_eq!(json["corner"], "bottom-right");
        assert_eq!(json["follow"], "smooth");
        assert_eq!(json["distance"], "normal");
        assert_eq!(json["flipAtEdges"], true);
        assert_eq!(json["peekKey"], "control");
        assert_eq!(json["fadeWhileClicking"], false);
        assert_eq!(json["shortcuts"]["pause"], "Control+Alt+Space");
        let back: Settings = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, Settings::default());
        let partial: Settings = serde_json::from_str(r#"{"wpm": 200}"#).expect("partial");
        assert_eq!(partial.wpm, 200);
        assert_eq!(partial.opacity, 85);
    }

    #[test]
    fn an_older_file_gets_the_missing_shortcuts() {
        let json = r#"{"shortcuts": {"stop": "Control+Alt+KeyS"}}"#;
        let settings: Settings = serde_json::from_str(json).expect("partial");
        let settings = settings.normalized();
        assert_eq!(settings.shortcuts.len(), 8);
        assert_eq!(settings.shortcuts[&crate::shortcuts::ShortcutAction::Stop], "Control+Alt+KeyS");
    }
}
