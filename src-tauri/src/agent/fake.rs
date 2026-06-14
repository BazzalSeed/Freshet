//! Test-only fake Agent with a synthesize call counter.
//!
//! The counter lets the engine phase assert "no synthesize on nothing-new"
//! (the calm-by-default invariant) without spawning a real agent.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::model::AgentKind;

use super::{Agent, ChatReply, ChatTurn, ResearchInput};

pub struct FakeAgent {
    kind: AgentKind,
    canned_doc: String,
    canned_reply: ChatReply,
    /// When true, `synthesize` builds a document that reflects its input items
    /// (instead of returning `canned_doc`), so engine tests can assert that new
    /// items actually flowed through reconciliation.
    reflect_items: bool,
    synthesize_calls: Arc<AtomicUsize>,
    chat_calls: Arc<AtomicUsize>,
    /// The `prior_doc` captured on the most recent `synthesize` call. Lets
    /// engine tests assert the agent never received the `## My notes` block.
    last_prior_doc: Arc<Mutex<Option<String>>>,
}

impl FakeAgent {
    /// A fake that returns `canned_doc` from `synthesize` and an empty chat reply.
    pub fn new(kind: AgentKind, canned_doc: impl Into<String>) -> Self {
        Self {
            kind,
            canned_doc: canned_doc.into(),
            canned_reply: ChatReply {
                text: String::new(),
                proposed_description: None,
            },
            reflect_items: false,
            synthesize_calls: Arc::new(AtomicUsize::new(0)),
            chat_calls: Arc::new(AtomicUsize::new(0)),
            last_prior_doc: Arc::new(Mutex::new(None)),
        }
    }

    /// A fake whose `synthesize` reflects its input: it returns a Freshet-owned
    /// document that lists each input item's title under `## What changed`. The
    /// returned doc never contains a `## My notes` section.
    pub fn reflecting(kind: AgentKind) -> Self {
        let mut f = Self::new(kind, String::new());
        f.reflect_items = true;
        f
    }

    /// The `prior_doc` passed to the most recent `synthesize` call, if any.
    pub fn last_prior_doc(&self) -> Option<String> {
        self.last_prior_doc.lock().unwrap().clone()
    }

    /// A fake whose `chat` returns the given canned reply (incl. a proposed
    /// description) — used by stream-creation tests.
    pub fn with_chat_reply(kind: AgentKind, reply: ChatReply) -> Self {
        Self {
            kind,
            canned_doc: String::new(),
            canned_reply: reply,
            reflect_items: false,
            synthesize_calls: Arc::new(AtomicUsize::new(0)),
            chat_calls: Arc::new(AtomicUsize::new(0)),
            last_prior_doc: Arc::new(Mutex::new(None)),
        }
    }

    /// How many times `synthesize` has been called.
    pub fn synthesize_calls(&self) -> usize {
        self.synthesize_calls.load(Ordering::SeqCst)
    }

    /// How many times `chat` has been called.
    pub fn chat_calls(&self) -> usize {
        self.chat_calls.load(Ordering::SeqCst)
    }
}

impl Agent for FakeAgent {
    fn kind(&self) -> AgentKind {
        self.kind
    }

    fn synthesize(&self, input: ResearchInput) -> anyhow::Result<String> {
        self.synthesize_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_prior_doc.lock().unwrap() = input.prior_doc.map(|s| s.to_string());

        if self.reflect_items {
            // Build a Freshet-owned document that reflects the input items.
            // Never emits a `## My notes` section — that is user-owned.
            let mut doc = String::from("# Reflected\n\n## What changed\n\n");
            for item in input.items {
                doc.push_str(&format!("- {}[^{}]\n", item.title, item.id));
            }
            doc.push_str("\n## Current understanding\n\nState reflects the items above.\n\n");
            doc.push_str("## Open questions\n\n- None.\n\n");
            for item in input.items {
                doc.push_str(&format!("[^{}]: {}\n", item.id, item.url));
            }
            return Ok(doc);
        }

        Ok(self.canned_doc.clone())
    }

    fn chat(&self, _system: &str, _history: &[ChatTurn]) -> anyhow::Result<ChatReply> {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.canned_reply.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_synthesize_calls() {
        let agent = FakeAgent::new(AgentKind::ClaudeCode, "DOC");
        assert_eq!(agent.synthesize_calls(), 0);

        let items = vec![];
        let _ = agent.synthesize(ResearchInput {
            topic: "rust",
            items: &items,
            prior_doc: None,
        });
        assert_eq!(agent.synthesize_calls(), 1);

        let _ = agent.synthesize(ResearchInput {
            topic: "rust",
            items: &items,
            prior_doc: None,
        });
        assert_eq!(agent.synthesize_calls(), 2);
    }

    #[test]
    fn returns_canned_chat_reply_with_proposed() {
        use crate::model::{Cadence, CadenceMode, StreamDescription, StreamStatus};
        let desc = StreamDescription {
            id: "s1".into(),
            title: "Rust".into(),
            topic: "Rust lang".into(),
            sources: vec!["hackernews".into()],
            cadence: Cadence {
                mode: CadenceMode::OnLaunch,
                interval_minutes: None,
            },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        };
        let agent = FakeAgent::with_chat_reply(
            AgentKind::ClaudeCode,
            ChatReply {
                text: "sure".into(),
                proposed_description: Some(desc.clone()),
            },
        );
        let reply = agent.chat("system", &[]).expect("chat");
        assert_eq!(reply.proposed_description, Some(desc));
        assert_eq!(agent.chat_calls(), 1);
    }
}
