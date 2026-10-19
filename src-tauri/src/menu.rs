//! Native menus: the macOS app menu, the sidebar context menu and the heading popup.
//! Every choice reaches the Library as one `menu://action` event with the item's id.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{ContextMenu, Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
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

/// Which popup the toolbar asked for.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PopupKind {
    Heading,
}

const HEADING_ITEMS: [(&str, &str); 4] = [
    ("format.title", "Title"),
    ("format.heading", "Heading"),
    ("format.subheading", "Subheading"),
    ("format.body", "Body"),
];

/// The macOS menu bar. Windows has none; the same shortcuts live in the webview there.
#[cfg(target_os = "macos")]
pub fn app_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>> {
    let item = |id: &str, text: &str, accelerator: &str| {
        MenuItemBuilder::with_id(id, text)
            .accelerator(accelerator)
            .build(app)
    };
    let aside = SubmenuBuilder::new(app, "Aside")
        .about(None)
        .separator()
        .item(&item("app.settings", "Settings…", "CmdOrCtrl+,")?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item("file.new-script", "New Script", "CmdOrCtrl+N")?)
        .item(&item("file.new-folder", "New Folder", "Shift+CmdOrCtrl+N")?)
        .separator()
        .item(&MenuItemBuilder::with_id("file.show-folder", "Show Scripts Folder").build(app)?)
        .separator()
        .close_window()
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&item("edit.find", "Find", "CmdOrCtrl+F")?)
        .build()?;
    let format = SubmenuBuilder::new(app, "Format")
        .item(&item("format.bold", "Bold", "CmdOrCtrl+B")?)
        .item(&item("format.italic", "Italic", "CmdOrCtrl+I")?)
        .separator()
        .item(&item("format.title", "Title", "Alt+CmdOrCtrl+1")?)
        .item(&item("format.heading", "Heading", "Alt+CmdOrCtrl+2")?)
        .item(&item("format.subheading", "Subheading", "Alt+CmdOrCtrl+3")?)
        .item(&item("format.body", "Body", "Alt+CmdOrCtrl+0")?)
        .separator()
        .item(&item("format.bulleted-list", "Bulleted List", "Shift+CmdOrCtrl+8")?)
        .item(&item("format.numbered-list", "Numbered List", "Shift+CmdOrCtrl+7")?)
        .separator()
        .item(&item("format.insert-pause", "Insert Pause", "Alt+CmdOrCtrl+P")?)
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item("view.toggle-sidebar", "Toggle Sidebar", "Ctrl+CmdOrCtrl+S")?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .fullscreen()
        .build()?;
    Ok(MenuBuilder::new(app)
        .items(&[&aside, &file, &edit, &format, &view, &window])
        .build()?)
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
    popup(app, &menu, x, y)
}

/// Pops a toolbar menu, such as the heading picker.
pub fn popup_menu(app: &AppHandle, kind: PopupKind, x: f64, y: f64) -> Result<()> {
    let mut builder = MenuBuilder::new(app);
    match kind {
        PopupKind::Heading => {
            for (id, text) in HEADING_ITEMS {
                builder = builder.text(id, text);
            }
        }
    }
    let menu = builder.build()?;
    *lock(&app.state::<MenuContext>().path) = None;
    popup(app, &menu, x, y)
}

fn popup(app: &AppHandle, menu: &Menu<tauri::Wry>, x: f64, y: f64) -> Result<()> {
    let webview = app
        .get_webview_window(prompter::LIBRARY)
        .ok_or(Error::MissingWindow(prompter::LIBRARY))?;
    menu.popup_at(webview.as_ref().window(), LogicalPosition::new(x, y))?;
    Ok(())
}

/// Routes a menu choice: tray items are handled in Rust, the rest goes to the Library.
pub fn handle(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    if id.starts_with("tray.") {
        let _ = crate::tray::on_action(app, &id);
        return;
    }
    if id == "app.settings" {
        let _ = crate::open_settings(app);
        return;
    }
    let path = lock(&app.state::<MenuContext>().path).take();
    let payload = ActionPayload { id, path };
    let _ = app.emit_to(prompter::LIBRARY, ACTION_EVENT, payload);
}
