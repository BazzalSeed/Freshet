//! The watch engine — the heart of Freshet's "push, not pull" model.
//!
//! [`refresh`] is one pass of the watch loop for a single stream: fetch from
//! sources, dedup against what's already been seen, and — only when there is
//! genuinely new material — reconcile the living document with the agent.
//!
//! Invariants honored here (vision §9):
//! - **Quiet by default**: when nothing is new we do *not* call the agent. No
//!   synthesis, no document rewrite, no manufactured novelty.
//! - **My-notes is sacred**: the user-owned `## My notes` block is split off
//!   before the prior document reaches the agent and re-attached byte-for-byte
//!   afterward. The agent never sees it.
//! - **History before overwrite**: a snapshot of the prior document is taken
//!   before the new one is written.
//! - **Deterministic**: the caller supplies `now`; the engine never reads the clock.

pub mod reconcile;

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::agent::{Agent, ResearchInput};
use crate::model::{StreamDescription, Summary};
use crate::sources::{fetch_all, SourceProvider};
use crate::store;
use crate::store::document::{read_document, splice_my_notes, write_document};

/// How many items each provider is asked for per pass.
const FETCH_LIMIT: usize = 30;

/// Hex-encoded SHA-256 of `s`.
fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Run one watch pass for `desc`.
///
/// `now` is an ISO-8601 string supplied by the caller (no `Utc::now()` inside,
/// for deterministic tests). Returns a [`Summary`] describing whether the
/// document changed and how many new items drove the change.
pub fn refresh(
    root: &Path,
    desc: &StreamDescription,
    agent: &dyn Agent,
    providers: &[Box<dyn SourceProvider>],
    now: &str,
) -> anyhow::Result<Summary> {
    // 1. Load prior state + prior document.
    let mut state = store::load_state(root, &desc.id);
    let prior_doc = read_document(root, &desc.title);

    // 2. Fetch + rank by score desc (stable).
    let mut items = fetch_all(providers, &desc.topic, FETCH_LIMIT);
    items.sort_by(|a, b| {
        let sa = a.score.unwrap_or(f64::NEG_INFINITY);
        let sb = b.score.unwrap_or(f64::NEG_INFINITY);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 3. Dedup against already-seen ids.
    let new: Vec<_> = items
        .into_iter()
        .filter(|it| !state.seen_item_ids.contains(&it.id))
        .collect();

    // 4. Nothing new → quiet. Touch last_checked_at, never call the agent.
    if new.is_empty() {
        state.last_checked_at = Some(now.to_string());
        store::save_state(root, &desc.id, &state)?;
        return Ok(Summary { changed: false, n_new: 0 });
    }

    // 5. New material → reconcile.
    //    Split the prior doc so the agent never sees the My-notes block.
    let (freshet_owned, my_notes_block) = match prior_doc.as_deref() {
        Some(doc) => {
            let (owned, block) = reconcile::extract_my_notes(doc);
            (Some(owned), block)
        }
        None => (None, None),
    };

    let synthesized = agent.synthesize(ResearchInput {
        topic: &desc.topic,
        items: &new,
        prior_doc: freshet_owned.as_deref(),
    })?;

    // Re-attach the user-owned My-notes block byte-for-byte.
    let final_doc = match &my_notes_block {
        Some(block) => splice_my_notes(&synthesized, block),
        None => synthesized,
    };

    // History: snapshot the prior document BEFORE overwriting it.
    if let Some(prior) = &prior_doc {
        store::history::snapshot(root, &desc.id, prior, now)?;
    }

    // Atomic write of the new living document.
    write_document(root, &desc.title, &final_doc)?;

    // Update state.
    for it in &new {
        state.seen_item_ids.push(it.id.clone());
    }
    state.last_checked_at = Some(now.to_string());
    state.last_changed_at = Some(now.to_string());
    state.doc_digest = Some(sha256_hex(&final_doc));
    store::save_state(root, &desc.id, &state)?;

    Ok(Summary { changed: true, n_new: new.len() as u32 })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::fake::FakeAgent;
    use crate::model::{
        AgentKind, Cadence, CadenceMode, SourceItem, StreamStatus,
    };
    use crate::sources::FakeSourceProvider;
    use crate::store::document::splice_my_notes;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    fn desc() -> StreamDescription {
        StreamDescription {
            id: "ai-agents".into(),
            title: "AI Agents".into(),
            topic: "autonomous AI agents".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::OnLaunch,
                interval_minutes: None,
            },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        }
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

    /// The core watch-loop behavior across three passes:
    /// fresh → nothing-new (no synth) → one new item.
    #[test]
    fn refresh_full_lifecycle_with_my_notes_preservation() {
        let dir = tmp();
        let root = dir.path();
        let d = desc();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);

        let item_a = item("hn:a", "Agent A ships", 10.0);
        let item_b = item("hn:b", "Agent B ships", 9.0);
        let item_c = item("hn:c", "Agent C ships", 8.0);

        // ── Run 1: A, B are both new. ──────────────────────────────────────
        let providers = providers_with(vec![item_a.clone(), item_b.clone()]);
        let s1 = refresh(root, &d, &agent, &providers, "2026-06-14T10:00:00Z").expect("run 1");
        assert!(s1.changed, "run 1 should change");
        assert_eq!(s1.n_new, 2, "run 1: A and B are new");
        assert_eq!(agent.synthesize_calls(), 1, "run 1: one synthesize");

        let doc = read_document(root, &d.title).expect("doc exists after run 1");
        assert!(doc.contains("Agent A ships"), "doc must mention A: {doc}");
        assert!(doc.contains("Agent B ships"), "doc must mention B: {doc}");

        // State updated.
        let state = store::load_state(root, &d.id);
        assert_eq!(state.last_checked_at.as_deref(), Some("2026-06-14T10:00:00Z"));
        assert_eq!(state.last_changed_at.as_deref(), Some("2026-06-14T10:00:00Z"));
        assert!(state.doc_digest.is_some(), "digest must be set");

        // ── Run 2: identical providers → nothing new, NO synthesize. ───────
        let providers = providers_with(vec![item_a.clone(), item_b.clone()]);
        let s2 = refresh(root, &d, &agent, &providers, "2026-06-14T11:00:00Z").expect("run 2");
        assert!(!s2.changed, "run 2 should be quiet");
        assert_eq!(s2.n_new, 0, "run 2: nothing new");
        assert_eq!(
            agent.synthesize_calls(),
            1,
            "run 2: synthesize MUST NOT be called on nothing-new"
        );
        // last_checked_at advanced even though nothing changed.
        let state = store::load_state(root, &d.id);
        assert_eq!(state.last_checked_at.as_deref(), Some("2026-06-14T11:00:00Z"));
        assert_eq!(
            state.last_changed_at.as_deref(),
            Some("2026-06-14T10:00:00Z"),
            "last_changed_at must NOT advance on a quiet pass"
        );

        // ── Inject a user-owned My-notes block before run 3. ───────────────
        let current = read_document(root, &d.title).expect("doc present");
        let with_notes = splice_my_notes(&current, "## My notes\n\n- mine\n");
        write_document(root, &d.title, &with_notes).expect("write notes");

        // ── Run 3: providers now also return C → one new item. ─────────────
        let providers = providers_with(vec![item_a.clone(), item_b.clone(), item_c.clone()]);
        let s3 = refresh(root, &d, &agent, &providers, "2026-06-14T12:00:00Z").expect("run 3");
        assert!(s3.changed, "run 3 should change");
        assert_eq!(s3.n_new, 1, "run 3: only C is new");
        assert_eq!(agent.synthesize_calls(), 2, "run 3: second synthesize");

        let doc = read_document(root, &d.title).expect("doc present after run 3");
        assert!(doc.contains("Agent C ships"), "doc must now mention C: {doc}");

        // My-notes survived byte-wise.
        assert!(doc.contains("- mine"), "user notes must survive reconcile: {doc}");
        assert_eq!(
            doc.matches("## My notes").count(),
            1,
            "exactly one My notes section: {doc}"
        );

        // The agent NEVER received the My-notes block.
        let prior = agent.last_prior_doc().expect("agent saw a prior doc on run 3");
        assert!(
            !prior.contains("## My notes"),
            "agent must never see My notes; got prior: {prior}"
        );
        assert!(
            !prior.contains("- mine"),
            "agent must never see the user's note text; got prior: {prior}"
        );
    }

    /// First-ever pass: no prior doc, so the agent receives `prior_doc: None`
    /// and no history snapshot is written.
    #[test]
    fn refresh_first_pass_has_no_prior_and_no_snapshot() {
        let dir = tmp();
        let root = dir.path();
        let d = desc();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);

        let providers = providers_with(vec![item("hn:a", "First", 1.0)]);
        let s = refresh(root, &d, &agent, &providers, "2026-06-14T10:00:00Z").expect("run");
        assert!(s.changed);
        assert!(agent.last_prior_doc().is_none(), "no prior doc on first pass");

        // No history snapshot yet (nothing to snapshot before first write).
        assert!(
            store::history::list_history(root, &d.id).is_empty(),
            "first pass must not snapshot a non-existent prior doc"
        );
    }

    /// A history snapshot of the prior document is taken before overwrite.
    #[test]
    fn refresh_snapshots_prior_before_overwrite() {
        let dir = tmp();
        let root = dir.path();
        let d = desc();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);

        // Run 1 writes the first doc.
        let p1 = providers_with(vec![item("hn:a", "First", 1.0)]);
        refresh(root, &d, &agent, &p1, "2026-06-14T10:00:00Z").expect("run 1");
        let v1 = read_document(root, &d.title).expect("v1");

        // Run 2 with a new item triggers reconcile → snapshot of v1.
        let p2 = providers_with(vec![item("hn:a", "First", 1.0), item("hn:b", "Second", 2.0)]);
        refresh(root, &d, &agent, &p2, "2026-06-14T11:00:00Z").expect("run 2");

        let history = store::history::list_history(root, &d.id);
        assert_eq!(history.len(), 1, "exactly one snapshot taken");
        let snap_path = store::history_dir(root, &d.id).join(&history[0].file);
        let snapped = std::fs::read_to_string(snap_path).expect("read snapshot");
        assert_eq!(snapped, v1, "snapshot must be the prior doc, byte-for-byte");
    }

    /// Empty providers → nothing new, quiet, no synth, no doc.
    #[test]
    fn refresh_empty_providers_is_quiet() {
        let dir = tmp();
        let root = dir.path();
        let d = desc();
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);

        let providers: Vec<Box<dyn SourceProvider>> = vec![];
        let s = refresh(root, &d, &agent, &providers, "2026-06-14T10:00:00Z").expect("run");
        assert!(!s.changed);
        assert_eq!(s.n_new, 0);
        assert_eq!(agent.synthesize_calls(), 0, "no synth when nothing fetched");
        assert!(read_document(root, &d.title).is_none(), "no doc written");
    }
}
