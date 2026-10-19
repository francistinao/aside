//! Native menus: the sidebar context menu now, the macOS app menu with the editor.
//! Every choice reaches the Library as one `menu://action` event.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{ContextMenu, MenuBuilder, MenuEvent};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager};

use crate::error::{Error, Result};
use crate::lock::lock;
use crate::prompter;
use crate::vault::EntryKind;

pub const ACTION_EVENT: &str = "menu://action";

/// The item a context menu was opened for, so its choice can name it.
#[derive(Default)]
pub struct MenuContext {
    path: Mutex<Option<String>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionPayload {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTarget {
    pub kind: EntryKind,
    pub pinned: bool,
}

/// Pops the sidebar menu at a point in the Library's webview (CSS pixels).
pub fn context_menu(app: &AppHandle, path: String, target: ContextTarget, x: f64, y: f64) -> Result<()> {
    let reveal = if cfg!(target_os = "windows") { "Show in Explorer" } else { "Show in Finder" };
    let mut builder = MenuBuilder::new(app)
        .text("new-script", "New Script")
        .text("new-folder", "New Folder")
        .separator()
        .text("rename", "Rename");
    if target.kind == EntryKind::Script {
        builder = builder.text(if target.pinned { "unpin" } else { "pin" }, if target.pinned { "Unpin" } else { "Pin" });
    }
    let menu = builder
        .text("reveal", reveal)
        .separator()
        .text("trash", "Move to Trash")
        .build()?;
    *lock(&app.state::<MenuContext>().path) = Some(path);
    let webview = app
        .get_webview_window(prompter::LIBRARY)
        .ok_or(Error::MissingWindow(prompter::LIBRARY))?;
    menu.popup_at(webview.as_ref().window(), LogicalPosition::new(x, y))?;
    Ok(())
}

/// Forwards any menu choice to the Library.
pub fn handle(app: &AppHandle, event: MenuEvent) {
    let path = lock(&app.state::<MenuContext>().path).take();
    let payload = ActionPayload {
        id: event.id().as_ref().to_string(),
        path,
    };
    let _ = app.emit_to(prompter::LIBRARY, ACTION_EVENT, payload);
}
