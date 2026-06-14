//! Snapshot history for stream documents.

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::{history_dir, read_to_string_opt, write_atomic};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// ISO-8601 timestamp string (as passed by the caller).
    pub ts: String,
    /// Filename relative to the history dir, e.g. `"2026-06-14T10:00:00Z.md"`.
    pub file: String,
}

// ── Internal paths ────────────────────────────────────────────────────────────

fn index_path(root: &Path, id: &str) -> std::path::PathBuf {
    history_dir(root, id).join("index.json")
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Write a snapshot of `doc` for stream `id` at timestamp `iso_ts`.
///
/// Creates `<root>/.freshet/history/<id>/<iso_ts>.md` and updates
/// `<root>/.freshet/history/<id>/index.json` (a `Vec<HistoryEntry>`).
///
/// The caller is responsible for providing the timestamp (no `chrono::Utc::now()`
/// inside this function) so tests can be deterministic.
pub fn snapshot(root: &Path, id: &str, doc: &str, iso_ts: &str) -> anyhow::Result<()> {
    let dir = history_dir(root, id);
    fs::create_dir_all(&dir).with_context(|| format!("create history dir {dir:?}"))?;

    // Write the snapshot file.
    let filename = format!("{iso_ts}.md");
    let snap_path = dir.join(&filename);
    write_atomic(&snap_path, doc).with_context(|| format!("write snapshot {snap_path:?}"))?;

    // Load existing index (or start fresh).
    let idx_path = index_path(root, id);
    let mut entries: Vec<HistoryEntry> = read_to_string_opt(&idx_path)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    entries.push(HistoryEntry {
        ts: iso_ts.to_string(),
        file: filename,
    });

    let json = serde_json::to_string_pretty(&entries).context("serialize history index")?;
    write_atomic(&idx_path, &json).context("write history index")?;

    Ok(())
}

/// Return all history entries for stream `id`, newest-first.
pub fn list_history(root: &Path, id: &str) -> Vec<HistoryEntry> {
    let idx_path = index_path(root, id);
    let mut entries: Vec<HistoryEntry> = read_to_string_opt(&idx_path)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Reverse so newest (last appended) is first.
    entries.reverse();
    entries
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    #[test]
    fn snapshot_writes_file_and_index() {
        let dir = tmp();
        let ts = "2026-06-14T10:00:00Z";
        snapshot(dir.path(), "s1", "# Doc\n\nContent.", ts).expect("snapshot");

        // Snapshot file exists with correct content.
        let snap_path = history_dir(dir.path(), "s1").join(format!("{ts}.md"));
        let content = read_to_string_opt(&snap_path).expect("snapshot file must exist");
        assert_eq!(content, "# Doc\n\nContent.");

        // Index has one entry.
        let entries = list_history(dir.path(), "s1");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ts, ts);
        assert_eq!(entries[0].file, format!("{ts}.md"));
    }

    #[test]
    fn second_snapshot_appends_to_index() {
        let dir = tmp();
        snapshot(dir.path(), "s1", "v1", "2026-06-14T10:00:00Z").expect("snap 1");
        snapshot(dir.path(), "s1", "v2", "2026-06-14T11:00:00Z").expect("snap 2");

        // Both snapshot files exist.
        let dir1 = history_dir(dir.path(), "s1");
        assert!(dir1.join("2026-06-14T10:00:00Z.md").exists());
        assert!(dir1.join("2026-06-14T11:00:00Z.md").exists());

        // Index has two entries.
        let entries = list_history(dir.path(), "s1");
        assert_eq!(entries.len(), 2, "expected 2 entries, got: {entries:?}");
    }

    #[test]
    fn list_history_newest_first() {
        let dir = tmp();
        snapshot(dir.path(), "s1", "v1", "2026-06-14T10:00:00Z").expect("snap 1");
        snapshot(dir.path(), "s1", "v2", "2026-06-14T11:00:00Z").expect("snap 2");

        let entries = list_history(dir.path(), "s1");
        assert_eq!(entries.len(), 2);
        // Newest first: 11:00 before 10:00.
        assert_eq!(entries[0].ts, "2026-06-14T11:00:00Z");
        assert_eq!(entries[1].ts, "2026-06-14T10:00:00Z");
    }

    #[test]
    fn list_history_empty_when_no_snapshots() {
        let dir = tmp();
        let entries = list_history(dir.path(), "nonexistent");
        assert!(entries.is_empty());
    }

    #[test]
    fn snapshot_content_preserved() {
        let dir = tmp();
        let doc = "# Rust news\n\n## What changed\n\n- Something new.\n\n[^hn1]: https://example.com\n\n## My notes\n\nMy thought.\n";
        let ts = "2026-06-14T12:00:00Z";
        snapshot(dir.path(), "s1", doc, ts).expect("snapshot");

        let snap_path = history_dir(dir.path(), "s1").join(format!("{ts}.md"));
        let loaded = read_to_string_opt(&snap_path).unwrap();
        assert_eq!(loaded, doc, "snapshot content must be byte-identical");
    }
}
