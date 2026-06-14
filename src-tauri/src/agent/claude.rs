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
    // Synthesis is tool-less text generation; no --permission-mode needed.
    vec![
        "-p".to_string(),
        prompt.to_string(),
        "--bare".to_string(),
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
        // Log the argv so failed invocations (e.g. "Not logged in") are traceable.
        log::info!(
            "claude synthesize: invoking {} with flags {:?}",
            path,
            &arg_refs[..arg_refs.len().min(4)], // first flags only, not the full prompt
        );
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            // Include both stderr and stdout so messages like
            // "Not logged in · Please run /login" (which claude may emit on
            // stdout) are surfaced to the UI rather than being swallowed.
            let detail = non_empty_detail(&out.stdout, &out.stderr);
            log::error!(
                "claude synthesize failed: {} | stderr={:?} | stdout={:?}",
                detail,
                out.stderr.trim(),
                out.stdout.trim(),
            );
            anyhow::bail!("claude synthesize failed: {detail}");
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
        log::info!(
            "claude chat: invoking {} with flags {:?} (turns={})",
            path,
            &arg_refs[..arg_refs.len().min(2)],
            history.len(),
        );
        // UNVERIFIED: live path
        let out = self.runner.run(path, &arg_refs)?;
        if !out.success {
            let detail = non_empty_detail(&out.stdout, &out.stderr);
            log::error!(
                "claude chat failed: {} | stderr={:?} | stdout={:?}",
                detail,
                out.stderr.trim(),
                out.stdout.trim(),
            );
            anyhow::bail!("claude chat failed: {detail}");
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
/// claude sometimes emits the human-readable error on stdout (e.g. "Not logged
/// in · Please run /login") rather than stderr, so we check both.
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
    use crate::agent::discovery::CmdOutput;
    use std::sync::Arc;

    /// A non-success CmdOutput with stderr "Not logged in" must surface that
    /// message in the synthesize Err, not swallow it.
    #[test]
    fn synthesize_non_zero_surfaces_stderr() {
        use crate::agent::ResearchInput;
        // We match on the program path + args prefix; just match the path key.
        // FakeCmdRunner matches exact keys, so we use a canned non-success response
        // that the ClaudeAgent will hit when it calls runner.run(path, args).
        // Rather than reproducing the exact prompt, wire a runner that returns
        // failure for ANY call (miss → default non-success empty CmdOutput has
        // empty stderr). Instead we register the exact key for our fake path.
        //
        // Build a minimal synthesize call: topic with no items so the prompt is
        // short, then key on path + first two args ("-p", prompt).
        // FakeCmdRunner.key = program \0 arg1 \0 arg2 ... so we can't easily
        // pre-register the full prompt. Use a separate approach: build a runner
        // that always returns the failure output regardless of key.
        struct AlwaysFailRunner {
            stdout: String,
            stderr: String,
        }
        impl CmdRunner for AlwaysFailRunner {
            fn run(&self, _program: &str, _args: &[&str]) -> anyhow::Result<CmdOutput> {
                Ok(CmdOutput {
                    success: false,
                    stdout: self.stdout.clone(),
                    stderr: self.stderr.clone(),
                })
            }
        }

        let runner = AlwaysFailRunner {
            stdout: String::new(),
            stderr: "Not logged in · Please run /login".to_string(),
        };
        let agent = ClaudeAgent::new(
            std::path::PathBuf::from("/fake/claude"),
            Arc::new(runner) as Arc<dyn CmdRunner>,
        );

        let items = vec![];
        let err = agent
            .synthesize(ResearchInput {
                topic: "rust",
                items: &items,
                prior_doc: None,
            })
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Not logged in"),
            "error must contain agent's stderr: {msg}"
        );
    }

    /// When stderr is empty but stdout carries the auth message, it must surface.
    #[test]
    fn synthesize_non_zero_surfaces_stdout_when_stderr_empty() {
        use crate::agent::ResearchInput;
        struct AlwaysFailRunner {
            stdout: String,
            stderr: String,
        }
        impl CmdRunner for AlwaysFailRunner {
            fn run(&self, _program: &str, _args: &[&str]) -> anyhow::Result<CmdOutput> {
                Ok(CmdOutput {
                    success: false,
                    stdout: self.stdout.clone(),
                    stderr: self.stderr.clone(),
                })
            }
        }

        let runner = AlwaysFailRunner {
            stdout: "Not logged in · Please run /login".to_string(),
            stderr: String::new(),
        };
        let agent = ClaudeAgent::new(
            std::path::PathBuf::from("/fake/claude"),
            Arc::new(runner) as Arc<dyn CmdRunner>,
        );

        let items = vec![];
        let err = agent
            .synthesize(ResearchInput {
                topic: "rust",
                items: &items,
                prior_doc: None,
            })
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Not logged in"),
            "error must contain agent's stdout when stderr empty: {msg}"
        );
    }

    #[test]
    fn synthesize_args_have_expected_flags() {
        let args = build_synthesize_args("PROMPT");
        assert!(args.contains(&"-p".to_string()), "missing -p: {args:?}");
        assert!(args.contains(&"PROMPT".to_string()));
        assert!(args.contains(&"--bare".to_string()), "missing --bare: {args:?}");
        // Synthesis is tool-less; --permission-mode must NOT appear (it was
        // previously passed without its required value, which claude rejects).
        assert!(
            !args.contains(&"--permission-mode".to_string()),
            "--permission-mode must not appear in synthesize argv: {args:?}"
        );
        // Verify no dangling value-expecting flag at the end of the argv.
        let last = args.last().map(String::as_str).unwrap_or("");
        assert!(
            !last.starts_with("--") || last == "--bare",
            "argv must not end on a value-expecting flag: {args:?}"
        );
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
        // Neither synthesize nor chat should carry --permission-mode.
        assert!(
            !args.contains(&"--permission-mode".to_string()),
            "--permission-mode must not appear in chat argv: {args:?}"
        );
    }
}
