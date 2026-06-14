pub mod document;
pub mod history;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::model::{AgentKind, StreamDescription, StreamState};

// ── Path helpers ─────────────────────────────────────────────────────────────

/// `<root>/.freshet/`
pub fn freshet_dir(root: &Path) -> PathBuf {
    root.join(".freshet")
}

/// `<root>/.freshet/config.json`
pub fn config_path(root: &Path) -> PathBuf {
    freshet_dir(root).join("config.json")
}

/// `<root>/.freshet/streams/`
pub fn streams_dir(root: &Path) -> PathBuf {
    freshet_dir(root).join("streams")
}

/// `<root>/.freshet/streams/<id>.json`
pub fn stream_desc_path(root: &Path, id: &str) -> PathBuf {
    streams_dir(root).join(format!("{id}.json"))
}

/// `<root>/.freshet/streams/<id>.state.json`
pub fn state_path(root: &Path, id: &str) -> PathBuf {
    streams_dir(root).join(format!("{id}.state.json"))
}

/// `<root>/.freshet/history/<id>/`
pub fn history_dir(root: &Path, id: &str) -> PathBuf {
    freshet_dir(root).join("history").join(id)
}

/// `<root>/<sanitized-title>.md`
///
/// Sanitizes the title into a safe filename: replaces path-separator characters
/// (`/`, `\`) and trims leading/trailing whitespace, then appends `.md`.
pub fn doc_path(root: &Path, title: &str) -> PathBuf {
    let safe: String = title
        .replace('/', "-")
        .replace('\\', "-")
        .trim()
        .to_string();
    root.join(format!("{safe}.md"))
}

// ── Atomic I/O ───────────────────────────────────────────────────────────────

/// Write `contents` to `path` atomically: write a temp file in the same
/// directory, `sync_all`, then `rename` over the target.  Creates parent
/// directories as needed and leaves no `.tmp` file on success or failure.
pub fn write_atomic(path: &Path, contents: &str) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("write_atomic: path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create_dir_all({parent:?})"))?;

    // Use tempfile in the same directory so rename is always same-filesystem.
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .context("create temp file")?;
    use std::io::Write as _;
    tmp.write_all(contents.as_bytes())
        .context("write temp file")?;
    tmp.as_file().sync_all().context("sync_all")?;

    // persist() renames the temp file to the final path; no .tmp stays behind.
    tmp.persist(path)
        .map_err(|e| anyhow::anyhow!("rename temp→target: {}", e.error))?;

    Ok(())
}

/// Read `path` to a `String`; returns `None` if the file does not exist.
pub fn read_to_string_opt(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_agent: Option<AgentKind>,
    pub onboarded: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            root: String::new(),
            selected_agent: None,
            onboarded: false,
        }
    }
}

/// Load the config from `<root>/.freshet/config.json`; returns `Config::default()` if absent.
pub fn load_config(root: &Path) -> Config {
    let path = config_path(root);
    read_to_string_opt(&path)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist `config` to `<root>/.freshet/config.json` atomically.
pub fn save_config(root: &Path, config: &Config) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(config).context("serialize Config")?;
    write_atomic(&config_path(root), &json)
}

// ── Stream descriptions ───────────────────────────────────────────────────────

/// Load a single `StreamDescription` by id.
pub fn load_description(root: &Path, id: &str) -> anyhow::Result<StreamDescription> {
    let path = stream_desc_path(root, id);
    let s = read_to_string_opt(&path)
        .with_context(|| format!("stream description not found: {id}"))?;
    serde_json::from_str(&s).with_context(|| format!("deserialize StreamDescription {id}"))
}

/// Persist a `StreamDescription`; the stream's `id` is used as the filename.
pub fn save_description(root: &Path, desc: &StreamDescription) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(desc).context("serialize StreamDescription")?;
    write_atomic(&stream_desc_path(root, &desc.id), &json)
}

/// Load all `StreamDescription`s from `<root>/.freshet/streams/*.json`.
/// State files (`*.state.json`) are skipped.  Missing or unreadable files are
/// silently ignored.
pub fn list_descriptions(root: &Path) -> Vec<StreamDescription> {
    let dir = streams_dir(root);
    let Ok(entries) = fs::read_dir(&dir) else {
        return vec![];
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // Only plain `<id>.json`, not `<id>.state.json`.
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name.ends_with(".state.json") || !name.ends_with(".json") {
            continue;
        }
        if let Some(s) = read_to_string_opt(&path) {
            if let Ok(desc) = serde_json::from_str::<StreamDescription>(&s) {
                out.push(desc);
            }
        }
    }
    out
}

// ── Stream state ──────────────────────────────────────────────────────────────

/// Load the `StreamState` for `id`; returns `StreamState::default()` if absent.
pub fn load_state(root: &Path, id: &str) -> StreamState {
    let path = state_path(root, id);
    read_to_string_opt(&path)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the `StreamState` for `id`.
pub fn save_state(root: &Path, id: &str, state: &StreamState) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(state).context("serialize StreamState")?;
    write_atomic(&state_path(root, id), &json)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cadence, CadenceMode, StreamStatus};
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    // ── write_atomic / read_to_string_opt ──────────────────────────────────

    #[test]
    fn atomic_write_round_trips() {
        let dir = tmp();
        let path = dir.path().join("hello.txt");
        write_atomic(&path, "hello world").expect("write");
        assert_eq!(read_to_string_opt(&path).unwrap(), "hello world");
    }

    #[test]
    fn atomic_write_overwrite_leaves_no_tmp() {
        let dir = tmp();
        let path = dir.path().join("data.json");
        write_atomic(&path, "first").expect("write 1");
        write_atomic(&path, "second").expect("write 2");

        // The target has the latest content.
        assert_eq!(read_to_string_opt(&path).unwrap(), "second");

        // No leftover .tmp* files.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.ends_with(".tmp") || n.starts_with(".tmp"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(leftovers.is_empty(), "unexpected tmp files: {leftovers:?}");
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = tmp();
        let path = dir.path().join("a").join("b").join("c.txt");
        write_atomic(&path, "deep").expect("write");
        assert_eq!(read_to_string_opt(&path).unwrap(), "deep");
    }

    // ── doc_path sanitization ──────────────────────────────────────────────

    #[test]
    fn doc_path_sanitizes_forward_slash() {
        let root = Path::new("/tmp/freshet-root");
        let p = doc_path(root, "AI/ML Trends");
        let filename = p.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "AI-ML Trends.md");
        // No sub-directory component was created.
        assert_eq!(p.parent().unwrap(), root);
    }

    #[test]
    fn doc_path_sanitizes_backslash() {
        let root = Path::new("/tmp/freshet-root");
        let p = doc_path(root, "Windows\\Path");
        let filename = p.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "Windows-Path.md");
    }

    #[test]
    fn doc_path_trims_whitespace() {
        let root = Path::new("/tmp/freshet-root");
        let p = doc_path(root, "  Rust news  ");
        let filename = p.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "Rust news.md");
    }

    // ── Config round-trip ──────────────────────────────────────────────────

    #[test]
    fn config_default_when_absent() {
        let dir = tmp();
        let cfg = load_config(dir.path());
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn config_round_trips() {
        let dir = tmp();
        let cfg = Config {
            root: "/home/user/notes".into(),
            selected_agent: Some(AgentKind::ClaudeCode),
            onboarded: true,
        };
        save_config(dir.path(), &cfg).expect("save");
        let loaded = load_config(dir.path());
        assert_eq!(cfg, loaded);
    }

    // ── StreamDescription round-trip ───────────────────────────────────────

    fn make_desc(id: &str) -> StreamDescription {
        StreamDescription {
            id: id.into(),
            title: format!("Stream {id}"),
            topic: "test topic".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::Manual,
                interval_minutes: None,
            },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        }
    }

    #[test]
    fn description_round_trips() {
        let dir = tmp();
        let desc = make_desc("s1");
        save_description(dir.path(), &desc).expect("save");
        let loaded = load_description(dir.path(), "s1").expect("load");
        assert_eq!(desc, loaded);
    }

    #[test]
    fn list_descriptions_returns_all_written() {
        let dir = tmp();
        let d1 = make_desc("s1");
        let d2 = make_desc("s2");
        save_description(dir.path(), &d1).expect("save s1");
        save_description(dir.path(), &d2).expect("save s2");

        let mut list = list_descriptions(dir.path());
        list.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], d1);
        assert_eq!(list[1], d2);
    }

    #[test]
    fn list_descriptions_skips_state_files() {
        let dir = tmp();
        let desc = make_desc("s1");
        save_description(dir.path(), &desc).expect("save desc");

        let state = StreamState {
            seen_item_ids: vec!["x".into()],
            ..Default::default()
        };
        save_state(dir.path(), "s1", &state).expect("save state");

        let list = list_descriptions(dir.path());
        assert_eq!(list.len(), 1, "state file must not appear in list_descriptions");
    }

    // ── StreamState round-trip ─────────────────────────────────────────────

    #[test]
    fn state_default_when_absent() {
        let dir = tmp();
        let state = load_state(dir.path(), "missing");
        assert_eq!(state, StreamState::default());
    }

    #[test]
    fn state_round_trips() {
        let dir = tmp();
        let state = StreamState {
            seen_item_ids: vec!["id1".into(), "id2".into()],
            last_checked_at: Some("2026-06-14T10:00:00Z".into()),
            last_changed_at: None,
            doc_digest: Some("abc123".into()),
        };
        save_state(dir.path(), "s1", &state).expect("save");
        let loaded = load_state(dir.path(), "s1");
        assert_eq!(state, loaded);
    }
}
