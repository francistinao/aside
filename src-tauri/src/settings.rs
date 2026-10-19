//! Typed settings on top of tauri-plugin-store. Defaults come from docs/PRODUCT.md.
//! A change is saved, applied to the running session and sent to the prompter.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};

use crate::error::Result;
use crate::follow::placement::Corner;
use crate::lock::lock;
use crate::prompter;
use crate::session::engine::{MAX_WPM, MIN_WPM};
use crate::session::{Control, Highlight, SessionHandle};

const FILE: &str = "settings.json";
const KEY: &str = "settings";
pub const STYLE_EVENT: &str = "prompter://style";

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub wpm: u32,
    pub highlight: Highlight,
    pub opacity: u8,
    pub corner: Corner,
    pub text_size: TextSize,
    pub hide_from_capture: bool,
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
        }
    }
}

/// Fields the frontend wants to change. Missing fields stay as they are.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub wpm: Option<u32>,
    pub highlight: Option<Highlight>,
    pub opacity: Option<u8>,
    pub corner: Option<Corner>,
    pub text_size: Option<TextSize>,
    pub hide_from_capture: Option<bool>,
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
        self.normalized()
    }

    /// Keeps every value inside the ranges the controls offer.
    fn normalized(mut self) -> Self {
        self.wpm = (self.wpm.clamp(MIN_WPM, MAX_WPM) / WPM_STEP) * WPM_STEP;
        self.opacity = (self.opacity.clamp(MIN_OPACITY, MAX_OPACITY) / OPACITY_STEP) * OPACITY_STEP;
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

impl From<Settings> for StylePayload {
    fn from(s: Settings) -> Self {
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
        })
    }

    pub fn get(&self) -> Settings {
        *lock(&self.current)
    }

    /// Applies a patch, saves it and returns the old and new values.
    pub fn update(&self, patch: SettingsPatch) -> Result<(Settings, Settings)> {
        let before = self.get();
        let after = before.patched(patch);
        if after != before {
            *lock(&self.current) = after;
            self.store.set(KEY, serde_json::to_value(after)?);
            self.store.save()?;
        }
        Ok((before, after))
    }
}

/// Pushes a change into the running session, the follow loop, capture exclusion and the overlay.
pub fn apply(app: &AppHandle, before: Settings, after: Settings) -> Result<()> {
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
    if StylePayload::from(after) != StylePayload::from(before) {
        emit_style(app, after)?;
    }
    Ok(())
}

pub fn emit_style(app: &AppHandle, settings: Settings) -> Result<()> {
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
        assert_eq!(json["textSize"], "small");
        assert_eq!(json["hideFromCapture"], true);
        assert_eq!(json["corner"], "bottom-right");
        let back: Settings = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, Settings::default());
        let partial: Settings = serde_json::from_str(r#"{"wpm": 200}"#).expect("partial");
        assert_eq!(partial.wpm, 200);
        assert_eq!(partial.opacity, 85);
    }
}
