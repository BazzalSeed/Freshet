//! Codex adapter: headless invocation via `codex exec`.

use std::path::PathBuf;
use std::sync::Arc;

use crate::model::{AgentKind, StreamDescription};

use super::discovery::CmdRunner;
use super::{build_reconcile_prompt, extract_proposed_description, finalize_synthesis, render_chat_prompt, Agent, ChatReply, ChatTurn, ResearchInput};

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
        // Log the argv so failed invocations are traceable.
        log::info!(
            "codex synthesize: invoking {} with flags {:?}",
            path,
            &arg_refs[..arg_refs.len().min(4)],
        );
        // Spawn in a neutral directory (temp) so codex does not pick up the
        // app's CLAUDE.md or project hooks from the Freshet source tree.
        // UNVERIFIED: live path
        let out = self.runner.run_in(path, &arg_refs, Some(&std::env::temp_dir()))?;
        if !out.success {
            // Include both stderr and stdout so auth errors (often on stdout)
            // are surfaced to the UI rather than being swallowed.
            let detail = non_empty_detail(&out.stdout, &out.stderr);
            log::error!(
                "codex synthesize failed: {} | stderr={:?} | stdout={:?}",
                detail,
                out.stderr.trim(),
                out.stdout.trim(),
            );
            anyhow::bail!("codex synthesize failed: {detail}");
        }
        // Append canonical footnote definitions for the items the agent cited.
        Ok(finalize_synthesis(&out.stdout, input.items))
    }

    fn chat(&self, system: &str, history: &[ChatTurn]) -> anyhow::Result<ChatReply> {
        let prompt = render_chat_prompt(system, history);
        let args = build_chat_args(&prompt);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let path = self
            .path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("codex path is not valid UTF-8"))?;
        log::info!(
            "codex chat: invoking {} with flags {:?} (turns={})",
            path,
            &arg_refs[..arg_refs.len().min(2)],
            history.len(),
        );
        // Spawn in a neutral directory (temp) — same rationale as synthesize.
        // UNVERIFIED: live path
        let out = self.runner.run_in(path, &arg_refs, Some(&std::env::temp_dir()))?;
        if !out.success {
            let detail = non_empty_detail(&out.stdout, &out.stderr);
            log::error!(
                "codex chat failed: {} | stderr={:?} | stdout={:?}",
                detail,
                out.stderr.trim(),
                out.stdout.trim(),
            );
            anyhow::bail!("codex chat failed: {detail}");
        }
        let proposed = extract_proposed(&out.stdout);
        Ok(ChatReply {
            text: out.stdout,
            proposed_description: proposed,
        })
    }
}

/// Return the most useful non-empty detail string from a failed subprocess.
///
/// Codex sometimes emits auth errors on stdout rather than stderr.
fn non_empty_detail<'a>(stdout: &'a str, stderr: &'a str) -> &'a str {
    let s = stderr.trim();
    let o = stdout.trim();
    if !s.is_empty() {
        s
    } else if !o.is_empty() {
        o
    } else {
        "(no output)"
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
