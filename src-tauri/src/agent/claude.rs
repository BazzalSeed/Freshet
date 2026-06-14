//! Claude Code adapter: headless invocation via `claude -p`.

use std::path::PathBuf;
use std::sync::Arc;

use crate::model::{AgentKind, SourceItem, StreamDescription};

use super::discovery::CmdRunner;
use super::{build_reconcile_prompt, extract_proposed_description, render_chat_prompt, Agent, ChatReply, ChatTurn, ResearchInput};

pub struct ClaudeAgent {
    pub path: PathBuf,
    pub runner: Arc<dyn CmdRunner>,
}

impl ClaudeAgent {
    pub fn new(path: PathBuf, runner: Arc<dyn CmdRunner>) -> Self {
        Self { path, runner }
    }
}

/// Build the argv (after the program) for a synthesize invocation.
/// Kept pure so the flag set is unit-testable without spawning anything.
pub fn build_synthesize_args(prompt: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        prompt.to_string(),
        "--bare".to_string(),
        "--permission-mode".to_string(),
        "dontAsk".to_string(),
    ]
}

/// Build the argv (after the program) for a chat invocation.
pub fn build_chat_args(prompt: &str) -> Vec<String> {
    vec!["-p".to_string(), prompt.to_string(), "--bare".to_string()]
}

/// Parse an optional proposed `StreamDescription` from a fenced ```json block.
pub fn extract_proposed(reply: &str) -> Option<StreamDescription> {
    extract_proposed_description(reply)
}

impl Agent for ClaudeAgent {
    fn kind(&self) -> AgentKind {
        AgentKind::ClaudeCode
    }

    fn synthesize(&self, input: ResearchInput) -> anyhow::Result<String> {
        let prompt = build_reconcile_prompt(&input);
        let args = build_synthesize_args(&prompt);
        // UNVERIFIED: live path
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let path = self
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("claude path is not valid UTF-8"))?;
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            anyhow::bail!("claude synthesize failed: {}", out.stderr.trim());
        }
        Ok(out.stdout)
    }

    fn chat(&self, system: &str, history: &[ChatTurn]) -> anyhow::Result<ChatReply> {
        let prompt = render_chat_prompt(system, history);
        let args = build_chat_args(&prompt);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let path = self
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("claude path is not valid UTF-8"))?;
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            anyhow::bail!("claude chat failed: {}", out.stderr.trim());
        }
        let proposed = extract_proposed(&out.stdout);
        Ok(ChatReply {
            text: out.stdout,
            proposed_description: proposed,
        })
    }
}

/// Render a SourceItem as a prompt list line. Shared shape used by both adapters.
pub(super) fn render_item_line(item: &SourceItem) -> String {
    let score = item
        .score
        .map(|s| format!(" (score {s})"))
        .unwrap_or_default();
    format!(
        "- [{}] {} — {}{}\n  {}",
        item.id, item.title, item.url, score, item.snippet
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_args_have_expected_flags() {
        let args = build_synthesize_args("PROMPT");
        assert!(args.contains(&"-p".to_string()), "missing -p: {args:?}");
        assert!(args.contains(&"PROMPT".to_string()));
        assert!(args.contains(&"--bare".to_string()), "missing --bare: {args:?}");
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"dontAsk".to_string()));
        // -p must immediately precede the prompt.
        let p_idx = args.iter().position(|a| a == "-p").unwrap();
        assert_eq!(args[p_idx + 1], "PROMPT");
    }

    #[test]
    fn chat_args_have_expected_flags() {
        let args = build_chat_args("HELLO");
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"--bare".to_string()));
        assert!(args.contains(&"HELLO".to_string()));
        // chat does NOT force a permission mode.
        assert!(!args.contains(&"--permission-mode".to_string()));
    }
}
