//! In-memory search over every script: file names first, then content, capped at 100 hits.
//! Rebuilt whenever the folder changes, so a query is a scan of strings already in memory.

use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{Entry, EntryKind};

pub const MAX_HITS: usize = 100;
/// Characters shown on each side of a content match.
const SNIPPET_RADIUS: usize = 30;

struct Doc {
    path: String,
    name: String,
    name_lower: String,
    content: String,
    content_lower: String,
}

#[derive(Default)]
pub struct Index {
    docs: Vec<Doc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub snippet: String,
}

impl Index {
    /// Reads every script under `root`. Unreadable files are left out.
    pub fn build(root: &Path, entries: &[Entry]) -> Self {
        let docs = entries
            .iter()
            .filter(|e| e.kind == EntryKind::Script)
            .filter_map(|e| {
                let content = fs::read_to_string(root.join(&e.path)).ok()?;
                Some(Doc {
                    path: e.path.clone(),
                    name: e.name.clone(),
                    name_lower: e.name.to_lowercase(),
                    content_lower: content.to_lowercase(),
                    content,
                })
            })
            .collect();
        Self { docs }
    }

    /// Keeps the index current after one of our own writes without re-reading the folder.
    pub fn update(&mut self, path: &str, content: &str) {
        if let Some(doc) = self.docs.iter_mut().find(|d| d.path == path) {
            doc.content_lower = content.to_lowercase();
            doc.content = content.to_string();
        }
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Name prefix matches rank first, then names that contain the query, then content.
    pub fn search(&self, query: &str) -> Vec<Hit> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut ranked: Vec<(u8, &Doc, Option<usize>)> = self
            .docs
            .iter()
            .filter_map(|doc| {
                let in_content = doc.content_lower.find(&query);
                let rank = if doc.name_lower.starts_with(&query) {
                    0
                } else if doc.name_lower.contains(&query) {
                    1
                } else {
                    in_content.map(|_| 2)?
                };
                Some((rank, doc, in_content))
            })
            .collect();
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name_lower.cmp(&b.1.name_lower)));
        ranked
            .into_iter()
            .take(MAX_HITS)
            .map(|(_, doc, at)| Hit {
                path: doc.path.clone(),
                name: doc.name.clone(),
                snippet: snippet(doc, at, query.chars().count()),
            })
            .collect()
    }
}

/// Text around the first content match, or the start of the script for name-only hits.
fn snippet(doc: &Doc, at: Option<usize>, query_chars: usize) -> String {
    let chars: Vec<char> = doc.content.chars().collect();
    let (start, end) = match at {
        Some(byte) => {
            let center = doc.content_lower[..byte].chars().count().min(chars.len());
            (
                center.saturating_sub(SNIPPET_RADIUS),
                (center + query_chars + SNIPPET_RADIUS).min(chars.len()),
            )
        }
        None => (0, (SNIPPET_RADIUS * 2).min(chars.len())),
    };
    chars[start..end]
        .iter()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut entries = Vec::new();
        for (path, content) in files {
            fs::write(dir.path().join(path), content).expect("write");
            entries.push(Entry {
                path: path.to_string(),
                name: path.trim_end_matches(".md").to_string(),
                kind: EntryKind::Script,
                modified: 0,
            });
        }
        let index = Index::build(dir.path(), &entries);
        (dir, index)
    }

    /// Performance check: 1,000 scripts of about 300 words.
    /// Run with `cargo test -- --ignored --nocapture search_a_thousand_scripts`.
    #[test]
    #[ignore]
    fn search_a_thousand_scripts() {
        let words = ["demo", "settings", "click", "record", "script", "pointer", "bubble", "share"];
        let dir = tempfile::tempdir().expect("temp dir");
        let mut entries = Vec::new();
        for i in 0..1000 {
            let path = format!("Script {i}.md");
            let body: Vec<String> = (0..300).map(|j| format!("{}{}", words[(i + j) % words.len()], j)).collect();
            fs::write(dir.path().join(&path), body.join(" ")).expect("write");
            entries.push(Entry {
                path: path.clone(),
                name: format!("Script {i}"),
                kind: EntryKind::Script,
                modified: 0,
            });
        }
        let built = std::time::Instant::now();
        let index = Index::build(dir.path(), &entries);
        println!("build: {} ms", built.elapsed().as_millis());
        for query in ["Script 5", "pointer12", "zzz"] {
            let started = std::time::Instant::now();
            let hits = index.search(query);
            let took = started.elapsed();
            println!("search {query:?}: {} hits in {:.2} ms", hits.len(), took.as_secs_f64() * 1000.0);
            assert!(took.as_millis() < 50, "search {query:?} took {took:?}");
        }
    }

    #[test]
    fn names_rank_before_content() {
        let (_dir, index) = index_with(&[
            ("Notes about demos.md", "nothing here"),
            ("Demo day.md", "plain"),
            ("Agenda.md", "The demo starts at nine and ends at ten."),
            ("Unrelated.md", "no match"),
        ]);
        let hits = index.search("demo");
        let names: Vec<&str> = hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, vec!["Demo day", "Notes about demos", "Agenda"]);
        assert!(hits[2].snippet.starts_with("The demo starts at nine"), "{}", hits[2].snippet);
    }

    #[test]
    fn search_is_case_insensitive_and_handles_korean() {
        let (_dir, index) = index_with(&[("안내.md", "안녕하세요 여러분, 시작합니다."), ("Demo.md", "")]);
        assert_eq!(index.search("여러분")[0].snippet, "안녕하세요 여러분, 시작합니다.");
        assert_eq!(index.search("안")[0].name, "안내");
        assert_eq!(index.search("DEMO")[0].name, "Demo");
        assert!(index.search("   ").is_empty());
    }

    #[test]
    fn hits_are_capped_at_one_hundred() {
        let files: Vec<(String, String)> = (0..120)
            .map(|i| (format!("Script {i:03}.md"), "shared word".to_string()))
            .collect();
        let refs: Vec<(&str, &str)> = files.iter().map(|(p, c)| (p.as_str(), c.as_str())).collect();
        let (_dir, index) = index_with(&refs);
        assert_eq!(index.len(), 120);
        assert_eq!(index.search("shared").len(), MAX_HITS);
        assert_eq!(index.search("script 00").len(), 10);
    }

    #[test]
    fn update_changes_the_content_without_a_rebuild() {
        let (_dir, mut index) = index_with(&[("a.md", "old words")]);
        assert_eq!(index.search("old").len(), 1);
        index.update("a.md", "fresh words");
        assert!(index.search("old").is_empty());
        assert_eq!(index.search("fresh")[0].snippet, "fresh words");
    }

    #[test]
    fn long_content_gets_a_window_around_the_match() {
        let body = format!("{} needle {}", "x".repeat(200), "y".repeat(200));
        let (_dir, index) = index_with(&[("long.md", &body)]);
        let snippet = &index.search("needle")[0].snippet;
        assert!(snippet.contains("needle"));
        assert!(snippet.chars().count() <= SNIPPET_RADIUS * 2 + 6 + 2);
    }
}
