//! Codex adapter: headless invocation via `codex exec`.

use std::path::PathBuf;
use std::sync::Arc;

use crate::model::{AgentKind, StreamDescription};

use super::discovery::CmdRunner;
use super::{build_reconcile_prompt, extract_proposed_description, render_chat_prompt, Agent, ChatReply, ChatTurn, ResearchInput};

pub struct CodexAgent {
    pub path: PathBuf,
    pub runner: Arc<dyn CmdRunner>,
}

impl CodexAgent {
    pub fn new(path: PathBuf, runner: Arc<dyn CmdRunner>) -> Self {
        Self { path, runner }
    }
}

/// Build the argv (after the program) for a synthesize invocation.
pub fn build_synthesize_args(prompt: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        prompt.to_string(),
        "--json".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
    ]
}

/// Build the argv (after the program) for a chat invocation.
pub fn build_chat_args(prompt: &str) -> Vec<String> {
    vec![
        "exec".to_string(),
        prompt.to_string(),
        "--json".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
    ]
}

/// Parse an optional proposed `StreamDescription` from a fenced ```json block.
pub fn extract_proposed(reply: &str) -> Option<StreamDescription> {
    extract_proposed_description(reply)
}

impl Agent for CodexAgent {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn synthesize(&self, input: ResearchInput) -> anyhow::Result<String> {
        let prompt = build_reconcile_prompt(&input);
        let args = build_synthesize_args(&prompt);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let path = self
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("codex path is not valid UTF-8"))?;
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            anyhow::bail!("codex synthesize failed: {}", out.stderr.trim());
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
            .ok_or_else(|| anyhow::anyhow!("codex path is not valid UTF-8"))?;
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            anyhow::bail!("codex chat failed: {}", out.stderr.trim());
        }
        let proposed = extract_proposed(&out.stdout);
        Ok(ChatReply {
            text: out.stdout,
            proposed_description: proposed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_args_have_expected_flags() {
        let args = build_synthesize_args("PROMPT");
        assert_eq!(args.first().map(String::as_str), Some("exec"), "exec must be first: {args:?}");
        assert!(args.contains(&"PROMPT".to_string()));
        assert!(args.contains(&"--json".to_string()), "missing --json: {args:?}");
        assert!(args.contains(&"--sandbox".to_string()));
        assert!(args.contains(&"read-only".to_string()));
    }

    #[test]
    fn chat_args_have_expected_flags() {
        let args = build_chat_args("HELLO");
        assert_eq!(args.first().map(String::as_str), Some("exec"));
        assert!(args.contains(&"--json".to_string()));
        assert!(args.contains(&"HELLO".to_string()));
    }
}
