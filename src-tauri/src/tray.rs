//! The menu bar extra on macOS and the tray icon on Windows: the menu from docs/PRODUCT.md,
//! kept in step with the session and the settings. The menu is rebuilt on state and
//! settings changes; only the progress line is updated in place while a session runs.
//! While a session runs Aside is an Accessory app, so this menu is the way back in.

use std::sync::Mutex;

use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::error::Result;
use crate::follow::placement::Corner;
use crate::lock::lock;
use crate::session::engine::StateKind;
use crate::session::{Highlight, SessionHandle, Status};
use crate::settings::{self, Settings, SettingsPatch, SettingsState};
use crate::shortcuts::ShortcutAction;

pub const ID: &str = "main";
const OPACITIES: [u8; 5] = [40, 55, 70, 85, 100];
const CORNERS: [(Corner, &str); 4] = [
    (Corner::TopLeft, "Top Left"),
    (Corner::TopRight, "Top Right"),
    (Corner::BottomLeft, "Bottom Left"),
    (Corner::BottomRight, "Bottom Right"),
];

/// Managed state: the icon, the progress item of the current menu and the last status.
pub struct TrayState {
    icon: TrayIcon<Wry>,
    progress: Mutex<Option<MenuItem<Wry>>>,
    last: Mutex<Status>,
}

struct Built {
    menu: Menu<Wry>,
    progress: Option<MenuItem<Wry>>,
}

/// Builds the icon and its first menu. Call from setup, after settings and session exist.
pub fn setup(app: &AppHandle) -> Result<()> {
    let status = app.state::<SessionHandle>().status();
    let settings = app.state::<SettingsState>().get();
    let built = build_menu(app, &status, &settings)?;
    let builder = TrayIconBuilder::with_id(ID).menu(&built.menu).tooltip("Aside");
    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(tauri::include_image!("icons/tray/trayTemplate@2x.png"))
        .icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    };
    let icon = builder.build(app)?;
    app.manage(TrayState {
        icon,
        progress: Mutex::new(built.progress),
        last: Mutex::new(status),
    });
    Ok(())
}

/// Called by the session thread whenever its status changes. Hops to the main thread and
/// never blocks the caller.
pub fn notify(app: &AppHandle, status: Status) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || apply(&handle, status));
}

fn apply(app: &AppHandle, status: Status) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let last = std::mem::replace(&mut *lock(&state.last), status.clone());
    let structure_changed =
        last.state != status.state || last.title != status.title || last.highlight != status.highlight;
    if structure_changed {
        let _ = rebuild(app);
    } else if let Some(item) = lock(&state.progress).as_ref() {
        let _ = item.set_text(progress_line(&status));
    }
}

/// Rebuilds the menu from the last status and the current settings.
pub fn rebuild(app: &AppHandle) -> Result<()> {
    let Some(state) = app.try_state::<TrayState>() else {
        return Ok(());
    };
    let status = lock(&state.last).clone();
    let settings = app.state::<SettingsState>().get();
    let built = build_menu(app, &status, &settings)?;
    state.icon.set_menu(Some(built.menu))?;
    *lock(&state.progress) = built.progress;
    Ok(())
}

/// True when a settings change affects what the menu shows.
pub fn shows(settings: &Settings) -> impl PartialEq + use<> {
    (
        settings.highlight,
        settings.corner,
        settings.opacity,
        settings.hide_from_capture,
        settings.shortcuts.clone(),
    )
}

fn build_menu(app: &AppHandle, status: &Status, settings: &Settings) -> Result<Built> {
    let running = status.state != StateKind::Idle;
    let accelerator = |action: ShortcutAction| {
        settings
            .shortcuts
            .get(&action)
            .map(|stored| menu_accelerator(stored))
    };
    let playback = |id: &str, text: &str, action: ShortcutAction| -> Result<MenuItem<Wry>> {
        let plain = MenuItemBuilder::with_id(id, text).enabled(running);
        if let Some(label) = accelerator(action) {
            // A label the menu cannot parse is left off rather than failing the menu.
            if let Ok(item) = MenuItemBuilder::with_id(id, text).enabled(running).accelerator(label).build(app) {
                return Ok(item);
            }
        }
        Ok(plain.build(app)?)
    };
    let check = |id: String, text: &str, checked: bool| CheckMenuItemBuilder::with_id(id, text).checked(checked).build(app);

    let title_text = if running { status.title.as_str() } else { "Not running" };
    let title = MenuItemBuilder::with_id("tray.title", title_text).enabled(false).build(app)?;
    let mut builder = MenuBuilder::new(app).item(&title);
    let mut progress = None;
    if running {
        let item = MenuItemBuilder::with_id("tray.progress", progress_line(status))
            .enabled(false)
            .build(app)?;
        builder = builder.item(&item);
        progress = Some(item);
    }
    let pause_text = if status.state == StateKind::Paused { "Resume" } else { "Pause" };
    builder = builder
        .separator()
        .item(&playback("tray.pause", pause_text, ShortcutAction::Pause)?)
        .item(&playback("tray.restart", "Restart", ShortcutAction::Restart)?)
        .item(&playback("tray.faster", "Faster", ShortcutAction::Faster)?)
        .item(&playback("tray.slower", "Slower", ShortcutAction::Slower)?)
        .separator();

    let highlight = SubmenuBuilder::new(app, "Highlight")
        .item(&check("tray.highlight.word".into(), "Word", settings.highlight == Highlight::Word)?)
        .item(&check("tray.highlight.line".into(), "Line", settings.highlight == Highlight::Line)?)
        .build()?;
    let mut position = SubmenuBuilder::new(app, "Position");
    for (corner, text) in CORNERS {
        let id = format!("tray.position.{}", corner_id(corner));
        position = position.item(&check(id, text, settings.corner == corner)?);
    }
    let mut opacity = SubmenuBuilder::new(app, "Opacity");
    for value in OPACITIES {
        opacity = opacity.item(&check(format!("tray.opacity.{value}"), &format!("{value}%"), settings.opacity == value)?);
    }
    builder = builder
        .item(&highlight)
        .item(&position.build()?)
        .item(&opacity.build()?)
        .separator()
        .item(&check("tray.hide-capture".into(), "Hide from Screen Sharing", settings.hide_from_capture)?)
        .item(&MenuItemBuilder::with_id("tray.open-library", "Open Library").accelerator("CmdOrCtrl+O").build(app)?)
        .separator()
        .item(&playback("tray.stop", "Stop Prompter", ShortcutAction::Stop)?);
    Ok(Built {
        menu: builder.build()?,
        progress,
    })
}

/// A tray menu choice. Ids are `tray.<item>` or `tray.<group>.<value>`.
pub fn on_action(app: &AppHandle, id: &str) -> Result<()> {
    let settings = app.state::<SettingsState>();
    let current = settings.get();
    let mut patch = SettingsPatch::default();
    match id {
        "tray.pause" => return crate::perform(app, ShortcutAction::Pause),
        "tray.restart" => return crate::perform(app, ShortcutAction::Restart),
        "tray.faster" => return crate::perform(app, ShortcutAction::Faster),
        "tray.slower" => return crate::perform(app, ShortcutAction::Slower),
        "tray.stop" => return crate::perform(app, ShortcutAction::Stop),
        "tray.open-library" => return crate::show_library(app),
        "tray.hide-capture" => patch.hide_from_capture = Some(!current.hide_from_capture),
        "tray.highlight.word" => patch.highlight = Some(Highlight::Word),
        "tray.highlight.line" => patch.highlight = Some(Highlight::Line),
        _ => {
            if let Some(corner) = id.strip_prefix("tray.position.") {
                patch.corner = CORNERS.iter().find(|(c, _)| corner_id(*c) == corner).map(|(c, _)| *c);
            } else if let Some(value) = id.strip_prefix("tray.opacity.") {
                patch.opacity = value.parse().ok();
            }
        }
    }
    let (before, after) = settings.update(patch)?;
    settings::apply(app, &before, &after)
}

fn corner_id(corner: Corner) -> &'static str {
    match corner {
        Corner::TopLeft => "top-left",
        Corner::TopRight => "top-right",
        Corner::BottomLeft => "bottom-left",
        Corner::BottomRight => "bottom-right",
    }
}

/// "0:58 of 2:45 · Word mode", as docs/PRODUCT.md shows it.
pub fn progress_line(status: &Status) -> String {
    let mode = match status.highlight {
        Highlight::Word => "Word",
        Highlight::Line => "Line",
    };
    format!("{} of {} · {mode} mode", clock(status.elapsed), clock(status.total))
}

fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// Turns a stored shortcut ("Control+Alt+KeyR") into the form menu labels accept
/// ("Ctrl+Alt+R"). Both parsers agree on Space, Period, the arrows and the modifiers.
pub fn menu_accelerator(stored: &str) -> String {
    stored
        .split('+')
        .map(|token| match token {
            "Control" => "Ctrl".to_string(),
            "Option" => "Alt".to_string(),
            "Command" | "Super" => "Cmd".to_string(),
            key if key.len() == 4 && key.starts_with("Key") => key[3..].to_string(),
            digit if digit.len() == 6 && digit.starts_with("Digit") => digit[5..].to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_reads_like_the_product_doc() {
        let status = Status {
            state: StateKind::Running,
            title: "Q3 Roadmap Demo".into(),
            elapsed: 58.4,
            total: 165.0,
            highlight: Highlight::Word,
        };
        assert_eq!(progress_line(&status), "0:58 of 2:45 · Word mode");
    }

    #[test]
    fn stored_shortcuts_become_menu_labels() {
        assert_eq!(menu_accelerator("Control+Alt+KeyR"), "Ctrl+Alt+R");
        assert_eq!(menu_accelerator("Control+Alt+ArrowLeft"), "Ctrl+Alt+ArrowLeft");
        assert_eq!(menu_accelerator("Control+Alt+Period"), "Ctrl+Alt+Period");
        assert_eq!(menu_accelerator("Shift+Digit8"), "Shift+8");
    }
}
