//! Scripts on disk: a folder the user owns, plain markdown files, nothing else.
//! Every path from the frontend is relative to that folder and checked before use.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::lock::lock;

const SCRIPT_EXT: &str = "md";
const UNTITLED: &str = "Untitled";
const NEW_FOLDER: &str = "New Folder";
/// How long after our own write the watcher may still see it echo back.
const OWN_WRITE_WINDOW: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    Folder,
    Script,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Relative path with forward slashes.
    pub path: String,
    /// File name without the extension, or the folder name.
    pub name: String,
    pub kind: EntryKind,
    /// Last modification in milliseconds since the epoch.
    pub modified: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    pub content: String,
    pub modified: u64,
}

/// Remembers what we just wrote so the watcher can tell our writes from the user's.
#[derive(Default)]
pub struct RecentWrites {
    entries: Vec<(PathBuf, u64, Instant)>,
}

impl RecentWrites {
    pub fn note(&mut self, path: PathBuf, content: &str) {
        self.prune();
        self.entries.retain(|(p, _, _)| *p != path);
        self.entries.push((path, content_hash(content), Instant::now()));
    }

    /// True when `path` recently got exactly this content from us.
    pub fn matches(&mut self, path: &Path, content: &str) -> bool {
        self.prune();
        let hash = content_hash(content);
        self.entries.iter().any(|(p, h, _)| p == path && *h == hash)
    }

    fn prune(&mut self) {
        let now = Instant::now();
        self.entries.retain(|(_, _, at)| now.duration_since(*at) < OWN_WRITE_WINDOW);
    }
}

pub fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Managed state: the scripts folder and what we wrote to it.
pub struct Vault {
    root: Mutex<Option<PathBuf>>,
    pub recent_writes: Mutex<RecentWrites>,
}

impl Vault {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root: Mutex::new(root),
            recent_writes: Mutex::new(RecentWrites::default()),
        }
    }

    pub fn set_root(&self, root: Option<PathBuf>) {
        *lock(&self.root) = root;
    }

    pub fn root_path(&self) -> Option<PathBuf> {
        lock(&self.root).clone()
    }

    /// The folder, if it was chosen and still exists.
    pub fn root(&self) -> Result<PathBuf> {
        let root = self.root_path().ok_or(Error::NoFolder)?;
        if root.is_dir() { Ok(root) } else { Err(Error::FolderMissing) }
    }

    /// Turns a relative path from the frontend into an absolute one inside the folder.
    /// Rejects absolute paths, `..`, and anything that resolves outside through a symlink.
    pub fn resolve(&self, relative: &str) -> Result<PathBuf> {
        let root = self.root()?;
        let rel = Path::new(relative);
        for component in rel.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => return Err(Error::OutsideVault),
            }
        }
        let joined = root.join(rel);
        let canonical_root = root.canonicalize()?;
        let mut probe = joined.clone();
        while !probe.exists() {
            probe = probe.parent().map(Path::to_path_buf).ok_or(Error::OutsideVault)?;
        }
        if probe.canonicalize()?.starts_with(&canonical_root) {
            Ok(joined)
        } else {
            Err(Error::OutsideVault)
        }
    }

    /// Every folder and script, folders first, sorted by name, hidden entries skipped.
    pub fn entries(&self) -> Result<Vec<Entry>> {
        let root = self.root()?;
        let mut entries = Vec::new();
        walk(&root, &root, &mut entries)?;
        Ok(entries)
    }

    pub fn read(&self, relative: &str) -> Result<FileData> {
        let path = self.resolve(relative)?;
        Ok(FileData {
            content: fs::read_to_string(&path)?,
            modified: modified_ms(&path),
        })
    }

    /// Writes to `name.md.tmp` beside the file, syncs, renames over the original.
    pub fn write(&self, relative: &str, content: &str) -> Result<FileData> {
        let path = self.resolve(relative)?;
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| Error::BadName(relative.to_string()))?;
        let tmp = path.with_file_name(format!("{file_name}.tmp"));
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        lock(&self.recent_writes).note(path.clone(), content);
        Ok(FileData {
            content: String::new(),
            modified: modified_ms(&path),
        })
    }

    /// A new empty script called Untitled, Untitled 2, Untitled 3 inside `dir`.
    pub fn create(&self, dir: &str) -> Result<String> {
        let folder = self.resolve(dir)?;
        let name = unique_name(&folder, UNTITLED, Some(SCRIPT_EXT));
        let relative = join_relative(dir, &name);
        self.write(&relative, "")?;
        Ok(relative)
    }

    pub fn create_folder(&self, dir: &str) -> Result<String> {
        let folder = self.resolve(dir)?;
        let name = unique_name(&folder, NEW_FOLDER, None);
        fs::create_dir(folder.join(&name))?;
        Ok(join_relative(dir, &name))
    }

    /// Renames a script or folder inside its own folder. Scripts keep their extension.
    pub fn rename(&self, relative: &str, new_name: &str) -> Result<String> {
        let path = self.resolve(relative)?;
        let new_name = new_name.trim();
        if new_name.is_empty() || new_name.starts_with('.') || new_name.contains(['/', '\\']) {
            return Err(Error::BadName(new_name.to_string()));
        }
        let file_name = if path.is_dir() {
            new_name.to_string()
        } else {
            format!("{}.{SCRIPT_EXT}", new_name.strip_suffix(".md").unwrap_or(new_name))
        };
        let target = path.with_file_name(&file_name);
        if target.exists() && target != path {
            return Err(Error::NameTaken(new_name.to_string()));
        }
        fs::rename(&path, &target)?;
        let parent = Path::new(relative)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        Ok(join_relative(&parent, &file_name))
    }

    /// Moves a script or folder to the OS Trash. No confirmation: the Trash is recoverable.
    pub fn trash(&self, relative: &str) -> Result<()> {
        let path = self.resolve(relative)?;
        trash::delete(&path).map_err(|_| Error::Trash(relative.to_string()))
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Entry>) -> Result<()> {
    let mut children: Vec<(bool, String, PathBuf)> = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = item.path();
        let is_dir = path.is_dir();
        let is_script = path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(SCRIPT_EXT);
        if is_dir || is_script {
            children.push((is_dir, name, path));
        }
    }
    children.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    for (is_dir, name, path) in children {
        let relative = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        out.push(Entry {
            path: relative,
            name: if is_dir {
                name
            } else {
                name.strip_suffix(".md").unwrap_or(&name).to_string()
            },
            kind: if is_dir { EntryKind::Folder } else { EntryKind::Script },
            modified: modified_ms(&path),
        });
        if is_dir {
            walk(root, &path, out)?;
        }
    }
    Ok(())
}

fn join_relative(dir: &str, name: &str) -> String {
    if dir.is_empty() || dir == "." {
        name.to_string()
    } else {
        format!("{}/{name}", dir.trim_end_matches('/'))
    }
}

/// `Base`, `Base 2`, `Base 3`: the first name that is free in `folder`.
fn unique_name(folder: &Path, base: &str, ext: Option<&str>) -> String {
    let candidate = |n: u32| {
        let stem = if n == 1 { base.to_string() } else { format!("{base} {n}") };
        match ext {
            Some(ext) => format!("{stem}.{ext}"),
            None => stem,
        }
    };
    (1..).map(candidate).find(|name| !folder.join(name).exists()).unwrap_or_else(|| candidate(1))
}

pub fn modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn vault() -> (TempDir, Vault) {
        let dir = tempfile::tempdir().expect("temp dir");
        let vault = Vault::new(Some(dir.path().to_path_buf()));
        (dir, vault)
    }

    fn names(entries: &[Entry]) -> Vec<String> {
        entries.iter().map(|e| format!("{:?} {}", e.kind, e.path)).collect()
    }

    #[test]
    fn resolve_rejects_paths_outside_the_folder() {
        let (dir, vault) = vault();
        fs::write(dir.path().join("ok.md"), "x").expect("write");
        assert!(vault.resolve("ok.md").is_ok());
        assert!(vault.resolve("new/deeper/file.md").is_ok(), "new paths inside are fine");
        assert!(matches!(vault.resolve("../x.md"), Err(Error::OutsideVault)));
        assert!(matches!(vault.resolve("a/../../x.md"), Err(Error::OutsideVault)));
        assert!(matches!(vault.resolve("/etc/passwd"), Err(Error::OutsideVault)));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_rejects_symlinks_that_point_outside() {
        let (dir, vault) = vault();
        let outside = tempfile::tempdir().expect("outside dir");
        fs::write(outside.path().join("secret.md"), "s").expect("write");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).expect("symlink");
        assert!(matches!(vault.resolve("link/secret.md"), Err(Error::OutsideVault)));
        assert!(matches!(vault.resolve("link"), Err(Error::OutsideVault)));
    }

    #[test]
    fn missing_or_unset_folder_is_reported() {
        let vault = Vault::new(None);
        assert!(matches!(vault.root(), Err(Error::NoFolder)));
        let vault = Vault::new(Some(PathBuf::from("/definitely/not/here")));
        assert!(matches!(vault.root(), Err(Error::FolderMissing)));
    }

    #[test]
    fn entries_list_scripts_and_folders_sorted_and_skip_hidden() {
        let (dir, vault) = vault();
        fs::create_dir_all(dir.path().join("Talks/.obsidian")).expect("dirs");
        fs::create_dir_all(dir.path().join(".git")).expect("dirs");
        fs::write(dir.path().join("zeta.md"), "").expect("write");
        fs::write(dir.path().join("Alpha.md"), "").expect("write");
        fs::write(dir.path().join("notes.txt"), "").expect("write");
        fs::write(dir.path().join(".hidden.md"), "").expect("write");
        fs::write(dir.path().join("Talks/Demo.md"), "").expect("write");
        let entries = vault.entries().expect("entries");
        assert_eq!(
            names(&entries),
            vec!["Folder Talks", "Script Talks/Demo.md", "Script Alpha.md", "Script zeta.md"]
        );
        assert_eq!(entries[1].name, "Demo");
    }

    #[test]
    fn write_is_atomic_and_read_returns_it() {
        let (dir, vault) = vault();
        fs::write(dir.path().join("a.md"), "old").expect("write");
        let data = vault.write("a.md", "new text").expect("write");
        assert!(data.modified > 0);
        assert_eq!(fs::read_to_string(dir.path().join("a.md")).expect("read"), "new text");
        assert!(!dir.path().join("a.md.tmp").exists(), "temp file is gone");
        assert_eq!(vault.read("a.md").expect("read").content, "new text");
    }

    #[test]
    fn create_avoids_name_collisions() {
        let (_dir, vault) = vault();
        assert_eq!(vault.create("").expect("create"), "Untitled.md");
        assert_eq!(vault.create("").expect("create"), "Untitled 2.md");
        assert_eq!(vault.create("").expect("create"), "Untitled 3.md");
        assert_eq!(vault.create_folder("").expect("folder"), "New Folder");
        assert_eq!(vault.create_folder("").expect("folder"), "New Folder 2");
        assert_eq!(vault.create("New Folder").expect("create"), "New Folder/Untitled.md");
    }

    #[test]
    fn rename_keeps_the_extension_and_refuses_taken_names() {
        let (dir, vault) = vault();
        fs::write(dir.path().join("a.md"), "").expect("write");
        fs::write(dir.path().join("b.md"), "").expect("write");
        assert!(matches!(vault.rename("a.md", "b"), Err(Error::NameTaken(_))));
        assert!(matches!(vault.rename("a.md", "../c"), Err(Error::BadName(_))));
        assert_eq!(vault.rename("a.md", "Renamed").expect("rename"), "Renamed.md");
        assert!(dir.path().join("Renamed.md").exists());
        fs::create_dir(dir.path().join("Old")).expect("dir");
        assert_eq!(vault.rename("Old", "New").expect("rename"), "New");
        assert!(dir.path().join("New").is_dir());
    }

    #[test]
    fn trash_removes_the_file_from_the_folder() {
        let (dir, vault) = vault();
        fs::write(dir.path().join("gone.md"), "bye").expect("write");
        vault.trash("gone.md").expect("trash");
        assert!(!dir.path().join("gone.md").exists());
        assert!(matches!(vault.trash("../outside.md"), Err(Error::OutsideVault)));
    }

    #[test]
    fn recent_writes_match_our_content_and_not_a_foreign_change() {
        let mut recent = RecentWrites::default();
        let path = PathBuf::from("/tmp/x.md");
        recent.note(path.clone(), "hello");
        assert!(recent.matches(&path, "hello"));
        assert!(!recent.matches(&path, "hello, edited elsewhere"));
        assert!(!recent.matches(Path::new("/tmp/other.md"), "hello"));
    }
}
