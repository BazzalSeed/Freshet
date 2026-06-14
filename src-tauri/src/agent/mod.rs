//! Agent layer — agent-agnostic detection + headless invocation.
//!
//! Two concrete adapters (Claude Code, Codex) sit behind one [`Agent`] trait.
//! Every process call is routed through the [`discovery::CmdRunner`] seam so
//! tests exercise the detection-tier logic and argv construction without ever
//! spawning a live agent. Real spawn paths are marked `// UNVERIFIED: live path`.

pub mod claude;
pub mod codex;
pub mod discovery;
pub mod fake;

use std::path::Path;
use std::sync::Arc;

use crate::model::{AgentKind, AgentStatus, SourceItem, StreamDescription};

use discovery::{find_binary, probe_version, CmdRunner};

// ── Trait + I/O types ───────────────────────────────────────────────────────

/// Input for a reconcile/synthesize pass.
pub struct ResearchInput<'a> {
    pub topic: &'a str,
    pub items: &'a [SourceItem],
    pub prior_doc: Option<&'a str>,
}

/// One turn of a creation chat. `role` is "user" or "assistant".
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

/// A chat reply, optionally carrying a structured stream proposal.
#[derive(Clone)]
pub struct ChatReply {
    pub text: String,
    pub proposed_description: Option<StreamDescription>,
}

/// The agent-agnostic contract. Both adapters and the FakeAgent implement this.
pub trait Agent: Send + Sync {
    fn kind(&self) -> AgentKind;
    /// Reconcile new items against the prior document; returns the new markdown.
    fn synthesize(&self, input: ResearchInput) -> anyhow::Result<String>;
    /// A creation-flow chat turn.
    fn chat(&self, system: &str, history: &[ChatTurn]) -> anyhow::Result<ChatReply>;
}

// ── Detection ───────────────────────────────────────────────────────────────

/// Detect both supported agents, returning an [`AgentStatus`] for each.
///
/// Runs `find_binary` + `probe_version` for claude and codex.
/// `available` = the binary was found. Production runs these two probes in
/// parallel; sequential here is fine (detection is one-shot at startup).
pub fn detect_agents(runner: &dyn CmdRunner, exists: &dyn Fn(&Path) -> bool) -> Vec<AgentStatus> {
    let claude = detect_one(
        AgentKind::ClaudeCode,
        "claude",
        &discovery::claude_candidates(),
        runner,
        exists,
    );
    let codex = detect_one(
        AgentKind::Codex,
        "codex",
        &discovery::codex_candidates(),
        runner,
        exists,
    );
    vec![claude, codex]
}

fn detect_one(
    kind: AgentKind,
    name: &str,
    candidates: &[std::path::PathBuf],
    runner: &dyn CmdRunner,
    exists: &dyn Fn(&Path) -> bool,
) -> AgentStatus {
    match find_binary(name, runner, candidates, exists) {
        Some(path) => {
            let version = probe_version(&path, runner);
            AgentStatus {
                kind,
                available: true,
                version,
                path: Some(path.to_string_lossy().into_owned()),
            }
        }
        None => AgentStatus {
            kind,
            available: false,
            version: None,
            path: None,
        },
    }
}

// ── Selection ───────────────────────────────────────────────────────────────

/// Pick a usable agent.
///
/// Preference order: the caller's `preferred` kind if it is available, else the
/// first available status. Returns `None` if nothing is available. The `runner`
/// is shared (`Arc`) into the constructed adapter so it can spawn later.
pub fn select_agent(
    preferred: Option<AgentKind>,
    statuses: &[AgentStatus],
    runner: Arc<dyn CmdRunner>,
) -> Option<Box<dyn Agent>> {
    let chosen = preferred
        .and_then(|p| statuses.iter().find(|s| s.kind == p && s.available))
        .or_else(|| statuses.iter().find(|s| s.available))?;

    let path = std::path::PathBuf::from(chosen.path.as_ref()?);
    match chosen.kind {
        AgentKind::ClaudeCode => Some(Box::new(claude::ClaudeAgent::new(path, runner))),
        AgentKind::Codex => Some(Box::new(codex::CodexAgent::new(path, runner))),
    }
}

// ── Shared prompt construction ──────────────────────────────────────────────

/// Build the reconcile prompt from a [`ResearchInput`].
///
/// NOTE: wording is provisional — tuned with the owner later (unverifiable
/// tonight). The *structure* is the contract: Freshet owns exactly three
/// sections and citations use `[^id]` footnotes.
pub(crate) fn build_reconcile_prompt(input: &ResearchInput) -> String {
    let mut p = String::new();
    p.push_str("You are Freshet's reconciler. Update a living knowledge document.\n\n");
    p.push_str(&format!("TOPIC: {}\n\n", input.topic));

    if let Some(prior) = input.prior_doc {
        p.push_str("PRIOR DOCUMENT (preserve its understanding; do not discard prior facts unless contradicted):\n");
        p.push_str("----- BEGIN PRIOR -----\n");
        p.push_str(prior);
        p.push_str("\n----- END PRIOR -----\n\n");
    } else {
        p.push_str("There is no prior document; this is the first synthesis.\n\n");
    }

    p.push_str("NEW ITEMS (cite by [^id] using the bracketed id):\n");
    if input.items.is_empty() {
        p.push_str("(none)\n");
    } else {
        for item in input.items {
            p.push_str(&claude::render_item_line(item));
            p.push('\n');
        }
    }
    p.push('\n');

    p.push_str(
        "Produce ONLY the Freshet-owned sections, in this exact order:\n\
         ## What changed\n## Current understanding\n## Open questions\n\n\
         Rules:\n\
         - Preserve prior understanding; integrate, do not restate wholesale.\n\
         - Significance over recency: omit items that don't change the picture.\n\
         - If nothing materially changed, say so plainly under \"What changed\".\n\
         - Cite claims with [^id] footnotes and provide matching \
         `[^id]: <source> — <url>` definitions at the end.\n\
         - Output GitHub-flavored markdown only. No preamble, no sign-off.\n",
    );
    p
}

/// Flatten a system prompt + chat history into a single `-p` prompt string.
pub(crate) fn render_chat_prompt(system: &str, history: &[ChatTurn]) -> String {
    let mut p = String::new();
    p.push_str(system);
    p.push_str("\n\n");
    for turn in history {
        let speaker = match turn.role.as_str() {
            "assistant" => "Assistant",
            _ => "User",
        };
        p.push_str(&format!("{speaker}: {}\n", turn.content));
    }
    p.push_str("\nAssistant:");
    p
}

/// Extract a proposed `StreamDescription` from the first fenced ```json block.
/// Defensive: missing block, malformed JSON, or shape mismatch all yield None.
pub(crate) fn extract_proposed_description(reply: &str) -> Option<StreamDescription> {
    let start = reply.find("```json")?;
    let after = &reply[start + "```json".len()..];
    let end = after.find("```")?;
    let json = after[..end].trim();
    serde_json::from_str::<StreamDescription>(json).ok()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cadence, CadenceMode, StreamStatus};
    use discovery::{CmdOutput, FakeCmdRunner};

    fn ok(stdout: &str) -> CmdOutput {
        CmdOutput {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    /// detect_agents: claude present (tier-1), codex absent → mixed availability.
    #[test]
    fn detect_agents_present_claude_absent_codex() {
        let runner = FakeCmdRunner::new()
            .with("which", &["claude"], ok("/usr/local/bin/claude\n"))
            .with(
                "/usr/local/bin/claude",
                &["--version"],
                ok("1.0.0\n"),
            );
        // codex: every lookup misses → absent.
        let exists = |_: &Path| -> bool { false };
        let statuses = detect_agents(&runner, &exists);

        let claude = statuses
            .iter()
            .find(|s| s.kind == AgentKind::ClaudeCode)
            .unwrap();
        assert!(claude.available, "claude should be available");
        assert_eq!(claude.version.as_deref(), Some("1.0.0"));
        assert_eq!(claude.path.as_deref(), Some("/usr/local/bin/claude"));

        let codex = statuses.iter().find(|s| s.kind == AgentKind::Codex).unwrap();
        assert!(!codex.available, "codex should be absent");
        assert!(codex.path.is_none());
        assert!(codex.version.is_none());
    }

    /// select_agent honors the preferred kind when available.
    #[test]
    fn select_agent_prefers_requested_kind() {
        let statuses = vec![
            AgentStatus {
                kind: AgentKind::ClaudeCode,
                available: true,
                version: Some("1.0.0".into()),
                path: Some("/usr/local/bin/claude".into()),
            },
            AgentStatus {
                kind: AgentKind::Codex,
                available: true,
                version: Some("2.0.0".into()),
                path: Some("/usr/local/bin/codex".into()),
            },
        ];
        let runner = Arc::new(FakeCmdRunner::new()) as Arc<dyn CmdRunner>;
        let agent = select_agent(Some(AgentKind::Codex), &statuses, runner).unwrap();
        assert_eq!(agent.kind(), AgentKind::Codex);
    }

    /// select_agent falls back to the first available when preferred is absent.
    #[test]
    fn select_agent_falls_back_to_available() {
        let statuses = vec![
            AgentStatus {
                kind: AgentKind::ClaudeCode,
                available: true,
                version: None,
                path: Some("/usr/local/bin/claude".into()),
            },
            AgentStatus {
                kind: AgentKind::Codex,
                available: false,
                version: None,
                path: None,
            },
        ];
        let runner = Arc::new(FakeCmdRunner::new()) as Arc<dyn CmdRunner>;
        // Prefer Codex, but it's unavailable → fall back to Claude.
        let agent = select_agent(Some(AgentKind::Codex), &statuses, runner).unwrap();
        assert_eq!(agent.kind(), AgentKind::ClaudeCode);
    }

    /// select_agent returns None when nothing is available.
    #[test]
    fn select_agent_none_when_unavailable() {
        let statuses = vec![AgentStatus {
            kind: AgentKind::ClaudeCode,
            available: false,
            version: None,
            path: None,
        }];
        let runner = Arc::new(FakeCmdRunner::new()) as Arc<dyn CmdRunner>;
        assert!(select_agent(None, &statuses, runner).is_none());
    }

    /// extract_proposed_description parses a fenced json StreamDescription.
    #[test]
    fn extract_proposed_parses_fenced_json() {
        let desc = StreamDescription {
            id: "s1".into(),
            title: "Rust news".into(),
            topic: "Rust programming".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::OnLaunch,
                interval_minutes: None,
            },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        };
        let json = serde_json::to_string_pretty(&desc).unwrap();
        let reply = format!("Sure, here's a stream:\n\n```json\n{json}\n```\n\nLet me know!");
        let parsed = extract_proposed_description(&reply).expect("should parse");
        assert_eq!(parsed, desc);
    }

    /// extract_proposed_description returns None when no json block is present.
    #[test]
    fn extract_proposed_none_when_absent() {
        assert!(extract_proposed_description("Just a plain chat reply, no JSON.").is_none());
    }

    /// extract_proposed_description returns None on malformed JSON (defensive).
    #[test]
    fn extract_proposed_none_on_malformed() {
        let reply = "```json\n{ not valid json }\n```";
        assert!(extract_proposed_description(reply).is_none());
    }

    /// The reconcile prompt includes topic, item ids, the three section headers,
    /// and footnote-citation instructions.
    #[test]
    fn reconcile_prompt_has_structure() {
        let items = vec![SourceItem {
            id: "hn:42".into(),
            source: "hackernews".into(),
            url: "https://example.com/x".into(),
            title: "Big release".into(),
            score: Some(99.0),
            snippet: "It shipped.".into(),
            created_at: None,
        }];
        let input = ResearchInput {
            topic: "Rust async",
            items: &items,
            prior_doc: Some("## Current understanding\nAsync is hard."),
        };
        let prompt = build_reconcile_prompt(&input);
        assert!(prompt.contains("Rust async"));
        assert!(prompt.contains("hn:42"));
        assert!(prompt.contains("## What changed"));
        assert!(prompt.contains("## Current understanding"));
        assert!(prompt.contains("## Open questions"));
        assert!(prompt.contains("[^id]"));
        assert!(prompt.contains("Async is hard."), "prior doc must be embedded");
    }
}
