//! Watches the scripts folder and tells the Library what changed, except for our own writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::Vault;
use crate::error::Result;
use crate::lock::lock;
use crate::prompter;

pub const CHANGED_EVENT: &str = "vault://changed";
const DEBOUNCE: Duration = Duration::from_millis(300);

pub type Watcher = Debouncer<RecommendedWatcher, RecommendedCache>;

#[derive(Clone, Debug, Serialize)]
struct ChangedPayload {
    paths: Vec<String>,
}

/// Starts watching `root` recursively. Dropping the returned watcher stops it.
pub fn start(app: AppHandle, root: &Path) -> Result<Watcher> {
    let root_owned = root.to_path_buf();
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        let Ok(events) = result else {
            return;
        };
        let vault = app.state::<Vault>();
        let paths = relevant_paths(
            &root_owned,
            events.iter().flat_map(|event| event.paths.iter()),
            |path, content| lock(&vault.recent_writes).matches(path, content),
        );
        if paths.is_empty() {
            return;
        }
        let _ = vault.rebuild_index();
        let _ = app.emit_to(prompter::LIBRARY, CHANGED_EVENT, ChangedPayload { paths });
    })?;
    debouncer.watch(root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

/// Keeps scripts and folders inside the root. Drops hidden items, temp files, anything that
/// is not markdown, and files that still hold exactly what we last wrote to them.
pub fn relevant_paths<'a>(
    root: &Path,
    paths: impl Iterator<Item = &'a PathBuf>,
    mut is_own: impl FnMut(&Path, &str) -> bool,
) -> Vec<String> {
    let mut out = Vec::new();
    for path in paths {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if relative
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let is_script = path.extension().and_then(|e| e.to_str()) == Some("md");
        // A folder has no extension. A removed folder cannot be checked, so the name decides.
        let is_folder = path.extension().is_none();
        if !is_script && !is_folder {
            continue;
        }
        if is_script && path.is_file() && fs::read_to_string(path).is_ok_and(|c| is_own(path, &c)) {
            continue;
        }
        let text = relative.to_string_lossy().replace('\\', "/");
        if !text.is_empty() && !out.contains(&text) {
            out.push(text);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::RecentWrites;

    #[test]
    fn own_writes_hidden_and_temp_files_are_filtered() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("mine.md"), "saved by us").expect("write");
        fs::write(root.join("theirs.md"), "edited elsewhere").expect("write");
        fs::write(root.join("mine.md.tmp"), "").expect("write");
        fs::create_dir(root.join(".git")).expect("dir");
        fs::create_dir(root.join("Talks")).expect("dir");
        let mut recent = RecentWrites::default();
        recent.note(root.join("mine.md"), "saved by us");
        recent.note(root.join("theirs.md"), "what we wrote before the user changed it");
        let paths = [
            root.join("mine.md"),
            root.join("theirs.md"),
            root.join("mine.md.tmp"),
            root.join(".git/HEAD"),
            root.join("notes.txt"),
            root.join("Talks"),
            root.join("Talks"),
            PathBuf::from("/elsewhere/other.md"),
        ];
        let kept = relevant_paths(root, paths.iter(), |p, c| recent.matches(p, c));
        assert_eq!(kept, vec!["theirs.md", "Talks"]);
    }
}
