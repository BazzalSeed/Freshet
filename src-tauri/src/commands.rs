//! Plain, testable command logic behind the TauriBridge.
//!
//! Every function here takes an explicit `root: &Path` (the stream vault) and/or
//! injectable `&dyn Agent` / `&[Box<dyn SourceProvider>]` so it can be unit-tested
//! against a tempdir with `FakeAgent` + `FakeSourceProvider` — no live agent, no
//! network. The `#[tauri::command]` wrappers in `bridge.rs` are thin: they resolve
//! managed state, call into here, and map `anyhow::Error -> String`.
//!
//! App-level config (the root-folder pointer, the selected agent, the `onboarded`
//! flag) lives OUTSIDE any stream root — the chicken-and-egg problem is that we
//! need it *to find* the root. It is persisted at the Tauri app-config dir as
//! `config.json` via [`load_app_config`] / [`save_app_config`], which take the
//! config-dir path explicitly so they are testable.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::agent::{Agent, ResearchInput};
use crate::engine;
use crate::model::{
    AgentKind, AgentStatus, Cadence, StreamDescription, StreamState, StreamStatus, StreamSummary,
    Summary,
};
use crate::sources::SourceProvider;
use crate::store;
use crate::store::document::{read_document, splice_my_notes, write_document};

// ── App-level config ────────────────────────────────────────────────────────

/// App-level config persisted at the Tauri app-config dir (NOT inside a stream
/// root). Holds the pointer to the vault root, the user's selected agent, and
/// whether onboarding has completed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_agent: Option<AgentKind>,
    pub onboarded: bool,
}

/// `<config_dir>/config.json`
fn app_config_path(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join("config.json")
}

/// Load the app config from `<config_dir>/config.json`; returns
/// `AppConfig::default()` if absent or unreadable.
pub fn load_app_config(config_dir: &Path) -> AppConfig {
    store::read_to_string_opt(&app_config_path(config_dir))
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist `cfg` to `<config_dir>/config.json` atomically.
pub fn save_app_config(config_dir: &Path, cfg: &AppConfig) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(cfg).context("serialize AppConfig")?;
    store::write_atomic(&app_config_path(config_dir), &json)
}

// ── Onboarding / config / agents ────────────────────────────────────────────

/// The shape returned by `get_onboarding_state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub onboarded: bool,
    pub has_root: bool,
    /// The status of the currently selected agent, if one is selected AND it is
    /// present in `agents`. `None` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentStatus>,
}

/// Compute the onboarding state from the app config and a freshly-detected list
/// of agent statuses (the caller runs detection with the real runner).
pub fn onboarding_state(cfg: &AppConfig, agents: &[AgentStatus]) -> OnboardingState {
    let agent = cfg.selected_agent.and_then(|k| {
        agents.iter().find(|s| s.kind == k).cloned()
    });
    OnboardingState {
        onboarded: cfg.onboarded,
        has_root: cfg.root.is_some(),
        agent,
    }
}

/// Persist `root` as the vault pointer in the app config, and create the
/// `.freshet/` directory tree under it so subsequent stream writes succeed.
pub fn set_root_folder(config_dir: &Path, root: &Path) -> anyhow::Result<()> {
    // Create the .freshet/ subtree eagerly.
    std::fs::create_dir_all(store::streams_dir(root))
        .with_context(|| format!("create streams dir under {root:?}"))?;
    std::fs::create_dir_all(store::freshet_dir(root).join("history"))
        .with_context(|| format!("create history dir under {root:?}"))?;

    let mut cfg = load_app_config(config_dir);
    cfg.root = Some(root.to_string_lossy().into_owned());
    save_app_config(config_dir, &cfg)
}

/// Persist the user's chosen default agent.
pub fn set_default_agent(config_dir: &Path, kind: AgentKind) -> anyhow::Result<()> {
    let mut cfg = load_app_config(config_dir);
    cfg.selected_agent = Some(kind);
    save_app_config(config_dir, &cfg)
}

/// Mark onboarding complete.
pub fn complete_onboarding(config_dir: &Path) -> anyhow::Result<()> {
    let mut cfg = load_app_config(config_dir);
    cfg.onboarded = true;
    save_app_config(config_dir, &cfg)
}

// ── Streams ─────────────────────────────────────────────────────────────────

/// List all streams in the vault as `StreamSummary`, each enriched with its
/// `last_checked_at` and `changed_since_seen` from its state file.
///
/// `changed_since_seen` is true when the stream has a `last_changed_at` that is
/// strictly newer than its `last_checked_at` would imply has been "seen" — in
/// v1 we model "seen" as: the document changed since the user last opened it.
/// Since opening a stream is a UI concern not yet persisted, we surface
/// `changed_since_seen = last_changed_at.is_some() && last_changed_at == last_checked_at`
/// is NOT a reliable signal; instead we use the simpler, honest rule below.
pub fn list_streams(root: &Path) -> Vec<StreamSummary> {
    let mut descs = store::list_descriptions(root);
    descs.sort_by(|a, b| a.id.cmp(&b.id));
    descs
        .into_iter()
        .map(|d| {
            let state = store::load_state(root, &d.id);
            StreamSummary {
                id: d.id.clone(),
                title: d.title.clone(),
                last_checked_at: state.last_checked_at.clone(),
                changed_since_seen: changed_since_seen(&state),
            }
        })
        .collect()
}

/// A stream is "changed since seen" when it has registered a change
/// (`last_changed_at`) on or after its most recent check — i.e. the latest
/// refresh produced new material the user may not have read yet.
///
/// In v1 there is no persisted "last opened" timestamp, so this is a best-effort
/// signal: a freshly-created or freshly-changed stream reads as changed; a
/// stream whose latest pass was quiet (only `last_checked_at` advanced) reads as
/// not changed.
fn changed_since_seen(state: &StreamState) -> bool {
    match (&state.last_changed_at, &state.last_checked_at) {
        (Some(changed), Some(checked)) => changed >= checked,
        (Some(_), None) => true,
        _ => false,
    }
}

/// The shape returned by `get_stream`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStreamResult {
    pub description: StreamDescription,
    pub document_markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
}

/// Load a single stream: its description, current document markdown (empty
/// string if no document yet), and last-checked timestamp.
pub fn get_stream(root: &Path, id: &str) -> anyhow::Result<GetStreamResult> {
    let description = store::load_description(root, id)?;
    let document_markdown = read_document(root, &description.title).unwrap_or_default();
    let state = store::load_state(root, id);
    Ok(GetStreamResult {
        description,
        document_markdown,
        last_checked_at: state.last_checked_at,
    })
}

/// Replace ONLY the `## My notes` section of a stream's document with
/// `markdown`, leaving the Freshet-owned prefix byte-identical. `markdown` is
/// the full replacement block; if it does not begin with `## My notes`, the
/// header is prepended.
pub fn save_notes(root: &Path, id: &str, markdown: &str) -> anyhow::Result<()> {
    let description = store::load_description(root, id)?;
    let doc = read_document(root, &description.title).unwrap_or_default();

    let block = if markdown.trim_start().starts_with("## My notes") {
        markdown.to_string()
    } else {
        format!("## My notes\n\n{}", markdown)
    };

    let spliced = splice_my_notes(&doc, &block);
    write_document(root, &description.title, &spliced)
}

/// Persist a new `status` for stream `id`.
pub fn set_stream_status(root: &Path, id: &str, status: StreamStatus) -> anyhow::Result<()> {
    let mut desc = store::load_description(root, id)?;
    desc.status = status;
    store::save_description(root, &desc)
}

/// Refresh a single stream: one watch pass via [`engine::refresh`].
///
/// The agent and providers are injected so this is testable; the live wrapper
/// resolves them from managed state. Returns the [`Summary`].
pub fn refresh_stream(
    root: &Path,
    id: &str,
    agent: &dyn Agent,
    providers: &[Box<dyn SourceProvider>],
    now: &str,
) -> anyhow::Result<Summary> {
    log::info!("commands::refresh_stream: id={:?}", id);
    let desc = store::load_description(root, id)?;
    let result = engine::refresh(root, &desc, agent, providers, now);
    match &result {
        Ok(summary) => log::info!(
            "commands::refresh_stream: id={:?} done — changed={} n_new={}",
            id,
            summary.changed,
            summary.n_new,
        ),
        Err(e) => log::error!("commands::refresh_stream: id={:?} failed: {e:#}", id),
    }
    result
}

/// The input to `generate_first_draft` (mirrors the frontend `DraftInput`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInput {
    pub topic: String,
    pub sources: Vec<String>,
    pub cadence: Cadence,
}

/// The result of `generate_first_draft` (mirrors the frontend `DraftResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftResult {
    pub draft_markdown: String,
    pub proposed_description: StreamDescription,
}

/// Turn a free-text `topic` into a URL/file-safe slug id.
///
/// Lowercases, replaces any run of non-alphanumeric characters with a single
/// `-`, and trims leading/trailing `-`. Empty input yields `"stream"`.
pub fn slugify(topic: &str) -> String {
    let mut out = String::with_capacity(topic.len());
    let mut last_dash = false;
    for ch in topic.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "stream".to_string()
    } else {
        trimmed
    }
}

/// Build a `StreamDescription` from a draft input and a creation timestamp.
pub fn description_from_input(input: &DraftInput, now: &str) -> StreamDescription {
    StreamDescription {
        id: slugify(&input.topic),
        title: input.topic.clone(),
        topic: input.topic.clone(),
        sources: input.sources.clone(),
        cadence: input.cadence.clone(),
        status: StreamStatus::Active,
        created_at: now.to_string(),
    }
}

/// Generate the first draft for a proposed stream WITHOUT persisting anything.
///
/// Builds a `StreamDescription` from `input`, fetches its sources, and calls
/// `agent.synthesize` (no prior document — this is the first synthesis). Returns
/// the draft markdown and the proposed description so the frontend can present
/// them before the user commits.
///
/// Returns `Err` when all sources returned 0 items — calling the agent with
/// nothing to say is pointless and would hang the UI on a slow/failing agent.
pub fn generate_first_draft(
    input: &DraftInput,
    agent: &dyn Agent,
    providers: &[Box<dyn SourceProvider>],
    now: &str,
) -> anyhow::Result<DraftResult> {
    log::info!(
        "commands::generate_first_draft: topic={:?} sources={:?}",
        input.topic,
        input.sources,
    );
    let proposed = description_from_input(input, now);

    let items = crate::sources::fetch_all(providers, &proposed.topic, 30);

    if items.is_empty() {
        log::warn!("commands::generate_first_draft: 0 items from all sources — aborting");
        anyhow::bail!(
            "No results from the selected sources. \
             Try another source — some (e.g. Reddit) may be blocked or need setup."
        );
    }

    let draft = agent.synthesize(ResearchInput {
        topic: &proposed.topic,
        items: &items,
        prior_doc: None,
    });

    match &draft {
        Ok(d) => log::info!(
            "commands::generate_first_draft: done — draft_len={} proposed_id={:?}",
            d.len(),
            proposed.id,
        ),
        Err(e) => log::error!("commands::generate_first_draft: synthesis failed: {e:#}"),
    }

    Ok(DraftResult {
        draft_markdown: draft?,
        proposed_description: proposed,
    })
}

/// Persist a new stream and run one refresh pass to produce + write its initial
/// document and state. Returns the resulting summary.
///
/// Fails if all sources return 0 items — we will not create an empty stream
/// whose first doc would be a hallucination with no evidence.
pub fn create_stream(
    root: &Path,
    desc: &StreamDescription,
    agent: &dyn Agent,
    providers: &[Box<dyn SourceProvider>],
    now: &str,
) -> anyhow::Result<StreamSummary> {
    log::info!(
        "commands::create_stream: id={:?} topic={:?} sources={:?}",
        desc.id,
        desc.topic,
        desc.sources,
    );
    // Pre-flight: fetch to see if sources have anything.  This also primes
    // the UI with a useful error rather than hanging on the agent call.
    let preview_items = crate::sources::fetch_all(providers, &desc.topic, 1);
    if preview_items.is_empty() {
        log::warn!("commands::create_stream: 0 items from all sources — aborting");
        anyhow::bail!(
            "No results from the selected sources. \
             Try another source — some (e.g. Reddit) may be blocked or need setup."
        );
    }

    store::save_description(root, desc)?;
    let result = engine::refresh(root, desc, agent, providers, now);
    match &result {
        Ok(summary) => log::info!(
            "commands::create_stream: id={:?} done — changed={} n_new={}",
            desc.id,
            summary.changed,
            summary.n_new,
        ),
        Err(e) => log::error!("commands::create_stream: id={:?} failed: {e:#}", desc.id),
    }
    result?;

    let state = store::load_state(root, &desc.id);
    Ok(StreamSummary {
        id: desc.id.clone(),
        title: desc.title.clone(),
        last_checked_at: state.last_checked_at.clone(),
        changed_since_seen: changed_since_seen(&state),
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::fake::FakeAgent;
    use crate::model::{AgentKind, CadenceMode};
    use crate::sources::{FakeSourceProvider, SourceProvider};
    use crate::model::SourceItem;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    fn item(id: &str, title: &str, score: f64) -> SourceItem {
        SourceItem {
            id: id.into(),
            source: "hackernews".into(),
            url: format!("https://example.com/{id}"),
            title: title.into(),
            score: Some(score),
            snippet: "snippet".into(),
            created_at: None,
        }
    }

    fn providers_with(items: Vec<SourceItem>) -> Vec<Box<dyn SourceProvider>> {
        vec![Box::new(FakeSourceProvider::new("hackernews", items))]
    }

    fn draft_input() -> DraftInput {
        DraftInput {
            topic: "Autonomous AI Agents".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::OnLaunch,
                interval_minutes: None,
            },
        }
    }

    // ── App config round-trip ───────────────────────────────────────────────

    #[test]
    fn app_config_default_when_absent() {
        let dir = tmp();
        assert_eq!(load_app_config(dir.path()), AppConfig::default());
    }

    #[test]
    fn app_config_round_trips() {
        let dir = tmp();
        let cfg = AppConfig {
            root: Some("/home/user/vault".into()),
            selected_agent: Some(AgentKind::ClaudeCode),
            onboarded: true,
        };
        save_app_config(dir.path(), &cfg).expect("save");
        assert_eq!(load_app_config(dir.path()), cfg);
    }

    #[test]
    fn app_config_serializes_camel_case() {
        let cfg = AppConfig {
            root: Some("/x".into()),
            selected_agent: Some(AgentKind::ClaudeCode),
            onboarded: false,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"selectedAgent\""), "got: {json}");
        assert!(json.contains("\"claude_code\""), "got: {json}");
        assert!(!json.contains("selected_agent"), "got: {json}");
    }

    // ── Onboarding state + mutators ─────────────────────────────────────────

    #[test]
    fn set_root_creates_freshet_dirs_and_persists() {
        let cfg_dir = tmp();
        let vault = tmp();
        set_root_folder(cfg_dir.path(), vault.path()).expect("set root");

        assert!(store::streams_dir(vault.path()).is_dir());
        assert!(store::freshet_dir(vault.path()).join("history").is_dir());

        let cfg = load_app_config(cfg_dir.path());
        assert_eq!(cfg.root.as_deref(), Some(vault.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn set_default_agent_and_complete_onboarding_persist() {
        let cfg_dir = tmp();
        set_default_agent(cfg_dir.path(), AgentKind::Codex).expect("set agent");
        complete_onboarding(cfg_dir.path()).expect("complete");

        let cfg = load_app_config(cfg_dir.path());
        assert_eq!(cfg.selected_agent, Some(AgentKind::Codex));
        assert!(cfg.onboarded);
    }

    #[test]
    fn onboarding_state_reflects_config_and_selected_agent() {
        let agents = vec![
            AgentStatus {
                kind: AgentKind::ClaudeCode,
                available: true,
                version: Some("1.0.0".into()),
                path: Some("/bin/claude".into()),
            },
            AgentStatus {
                kind: AgentKind::Codex,
                available: false,
                version: None,
                path: None,
            },
        ];

        // No root, not onboarded, no selected agent.
        let empty = AppConfig::default();
        let st = onboarding_state(&empty, &agents);
        assert!(!st.onboarded);
        assert!(!st.has_root);
        assert!(st.agent.is_none());

        // Fully configured.
        let cfg = AppConfig {
            root: Some("/vault".into()),
            selected_agent: Some(AgentKind::ClaudeCode),
            onboarded: true,
        };
        let st = onboarding_state(&cfg, &agents);
        assert!(st.onboarded);
        assert!(st.has_root);
        assert_eq!(st.agent.as_ref().map(|a| a.kind), Some(AgentKind::ClaudeCode));
        assert_eq!(st.agent.as_ref().map(|a| a.available), Some(true));
    }

    // ── slugify ─────────────────────────────────────────────────────────────

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Autonomous AI Agents"), "autonomous-ai-agents");
        assert_eq!(slugify("  Rust!! news  "), "rust-news");
        assert_eq!(slugify("AI/ML & Trends"), "ai-ml-trends");
        assert_eq!(slugify("!!!"), "stream");
        assert_eq!(slugify(""), "stream");
    }

    // ── create_stream / list_streams / get_stream ───────────────────────────

    #[test]
    fn create_then_list_and_get() {
        let dir = tmp();
        let root = dir.path();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers = providers_with(vec![item("hn:a", "Agent A ships", 10.0)]);

        let desc = description_from_input(&draft_input(), "2026-06-14T10:00:00Z");
        let summary = create_stream(root, &desc, &agent, &providers, "2026-06-14T10:00:00Z")
            .expect("create");
        assert_eq!(summary.id, desc.id);
        assert!(summary.changed_since_seen, "fresh stream should read as changed");

        // list_streams returns it.
        let list = list_streams(root);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, desc.id);
        assert_eq!(list[0].last_checked_at.as_deref(), Some("2026-06-14T10:00:00Z"));

        // get_stream returns the written document.
        let got = get_stream(root, &desc.id).expect("get");
        assert_eq!(got.description, desc);
        assert!(got.document_markdown.contains("Agent A ships"), "doc: {}", got.document_markdown);
        assert_eq!(got.last_checked_at.as_deref(), Some("2026-06-14T10:00:00Z"));
    }

    #[test]
    fn refresh_twice_second_is_unchanged() {
        let dir = tmp();
        let root = dir.path();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);

        let desc = description_from_input(&draft_input(), "2026-06-14T10:00:00Z");
        store::save_description(root, &desc).expect("save desc");

        let providers = providers_with(vec![item("hn:a", "A", 1.0)]);
        let s1 = refresh_stream(root, &desc.id, &agent, &providers, "2026-06-14T10:00:00Z")
            .expect("refresh 1");
        assert!(s1.changed);

        let providers = providers_with(vec![item("hn:a", "A", 1.0)]);
        let s2 = refresh_stream(root, &desc.id, &agent, &providers, "2026-06-14T11:00:00Z")
            .expect("refresh 2");
        assert!(!s2.changed, "second refresh with no new items must be unchanged");
        assert_eq!(s2.n_new, 0);
    }

    #[test]
    fn quiet_pass_reads_as_not_changed_since_seen() {
        let dir = tmp();
        let root = dir.path();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let desc = description_from_input(&draft_input(), "2026-06-14T10:00:00Z");
        store::save_description(root, &desc).expect("save");

        let providers = providers_with(vec![item("hn:a", "A", 1.0)]);
        refresh_stream(root, &desc.id, &agent, &providers, "2026-06-14T10:00:00Z").expect("r1");
        // Quiet pass: no new items, only last_checked_at advances.
        let providers = providers_with(vec![item("hn:a", "A", 1.0)]);
        refresh_stream(root, &desc.id, &agent, &providers, "2026-06-14T12:00:00Z").expect("r2");

        let list = list_streams(root);
        assert_eq!(list.len(), 1);
        assert!(
            !list[0].changed_since_seen,
            "a quiet pass (checked advanced, changed did not) must read as not changed"
        );
        assert_eq!(list[0].last_checked_at.as_deref(), Some("2026-06-14T12:00:00Z"));
    }

    // ── save_notes ──────────────────────────────────────────────────────────

    #[test]
    fn save_notes_updates_only_my_notes() {
        let dir = tmp();
        let root = dir.path();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers = providers_with(vec![item("hn:a", "Agent A ships", 10.0)]);
        let desc = description_from_input(&draft_input(), "2026-06-14T10:00:00Z");
        create_stream(root, &desc, &agent, &providers, "2026-06-14T10:00:00Z").expect("create");

        let before = read_document(root, &desc.title).expect("doc");
        assert!(before.contains("Agent A ships"));

        save_notes(root, &desc.id, "My private thought.").expect("save notes");

        let after = read_document(root, &desc.title).expect("doc after");
        // Freshet-owned content preserved.
        assert!(after.contains("Agent A ships"), "freshet content lost: {after}");
        // Note added under exactly one My notes section.
        assert!(after.contains("My private thought."), "note missing: {after}");
        assert_eq!(after.matches("## My notes").count(), 1, "exactly one notes section: {after}");

        // Saving again replaces only the note, not the freshet content.
        save_notes(root, &desc.id, "## My notes\n\nUpdated thought.\n").expect("save 2");
        let after2 = read_document(root, &desc.title).expect("doc after 2");
        assert!(after2.contains("Agent A ships"), "freshet content lost on 2nd save");
        assert!(after2.contains("Updated thought."));
        assert!(!after2.contains("My private thought."), "old note should be replaced");
        assert_eq!(after2.matches("## My notes").count(), 1);
    }

    // ── set_stream_status ───────────────────────────────────────────────────

    #[test]
    fn set_status_persists() {
        let dir = tmp();
        let root = dir.path();
        let desc = description_from_input(&draft_input(), "2026-06-14T10:00:00Z");
        store::save_description(root, &desc).expect("save");

        set_stream_status(root, &desc.id, StreamStatus::Paused).expect("pause");
        assert_eq!(store::load_description(root, &desc.id).unwrap().status, StreamStatus::Paused);

        set_stream_status(root, &desc.id, StreamStatus::Retired).expect("retire");
        assert_eq!(store::load_description(root, &desc.id).unwrap().status, StreamStatus::Retired);
    }

    // ── generate_first_draft ────────────────────────────────────────────────

    #[test]
    fn first_draft_returns_draft_and_wellformed_description() {
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers = providers_with(vec![item("hn:a", "Big agent news", 9.0)]);

        let input = draft_input();
        let res = generate_first_draft(&input, &agent, &providers, "2026-06-14T10:00:00Z")
            .expect("draft");

        // Draft reflects the fetched item.
        assert!(res.draft_markdown.contains("Big agent news"), "draft: {}", res.draft_markdown);

        // Proposed description is well-formed.
        let pd = &res.proposed_description;
        assert_eq!(pd.id, "autonomous-ai-agents");
        assert_eq!(pd.title, "Autonomous AI Agents");
        assert_eq!(pd.topic, "Autonomous AI Agents");
        assert_eq!(pd.sources, vec!["hackernews".to_string()]);
        assert_eq!(pd.status, StreamStatus::Active);
        assert_eq!(pd.created_at, "2026-06-14T10:00:00Z");
        assert_eq!(pd.cadence.mode, CadenceMode::OnLaunch);

        // Nothing was persisted — generate_first_draft is a preview.
        assert!(store::list_descriptions(std::path::Path::new("/nonexistent-xyz")).is_empty());
    }

    #[test]
    fn first_draft_does_not_persist() {
        let dir = tmp();
        let root = dir.path();
        // Pre-create the streams dir so list works.
        std::fs::create_dir_all(store::streams_dir(root)).unwrap();

        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers = providers_with(vec![item("hn:a", "x", 1.0)]);
        generate_first_draft(&draft_input(), &agent, &providers, "2026-06-14T10:00:00Z")
            .expect("draft");

        // No description should have been written to the vault.
        assert!(store::list_descriptions(root).is_empty(), "draft must not persist a stream");
    }

    // ── 0-items guard (Fix 2) ───────────────────────────────────────────────

    /// generate_first_draft with 0-item providers must return Err with a useful
    /// message and must NOT call synthesize.
    #[test]
    fn first_draft_zero_items_returns_err_no_synth() {
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        // Empty provider list → fetch_all returns 0 items.
        let providers: Vec<Box<dyn SourceProvider>> = vec![];

        let err = generate_first_draft(&draft_input(), &agent, &providers, "2026-06-14T10:00:00Z")
            .unwrap_err();

        assert!(
            err.to_string().contains("No results"),
            "error must mention 'No results': {err}"
        );
        assert_eq!(
            agent.synthesize_calls(),
            0,
            "synthesize must not be called when 0 items fetched"
        );
    }

    /// generate_first_draft with a FakeSourceProvider returning empty Vec also fails.
    #[test]
    fn first_draft_empty_provider_returns_err_no_synth() {
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers: Vec<Box<dyn SourceProvider>> =
            vec![Box::new(FakeSourceProvider::new("hackernews", vec![]))];

        let err = generate_first_draft(&draft_input(), &agent, &providers, "2026-06-14T10:00:00Z")
            .unwrap_err();

        assert!(
            err.to_string().contains("No results"),
            "error must mention 'No results': {err}"
        );
        assert_eq!(agent.synthesize_calls(), 0, "synthesize must not be called");
    }
}
