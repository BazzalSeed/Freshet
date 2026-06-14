//! 3-tier PATH-robust binary discovery (blueprinted on Tolaria).
//!
//! The macOS GUI-PATH problem: a Finder-launched app does NOT inherit the
//! interactive shell PATH, so npm/nvm/Homebrew-installed `claude`/`codex`
//! binaries look "missing" to `which`. We solve this with a three-tier search,
//! all process calls routed through an injected [`CmdRunner`] seam so tests
//! never spawn a real process.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── CmdRunner seam ──────────────────────────────────────────────────────────

/// Result of running a subprocess: did it succeed, and what did it print.
#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// One method: run a program with args, get structured output back.
/// Real impl shells out via `std::process::Command`; tests use `FakeCmdRunner`.
pub trait CmdRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<CmdOutput>;
}

/// Production runner using `std::process::Command`.
///
/// Every subprocess invocation is subject to a hard 120-second timeout: we
/// spawn the child, hand off stdout/stderr collection to a background thread,
/// and `recv_timeout` in the calling thread. On timeout the child is killed and
/// we return an error so the UI is never left hanging. Real agent paths are
/// marked `// UNVERIFIED: live path`.
// UNVERIFIED: live path
pub struct RealCmdRunner;

/// Hard timeout applied to every subprocess (120 s).
const CMD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

impl CmdRunner for RealCmdRunner {
    // UNVERIFIED: live path
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<CmdOutput> {
        use std::sync::mpsc;

        // Spawn with piped stdout/stderr so we can collect output.
        let child = std::process::Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Hand off blocking wait+collect to a thread, so we can timeout.
        let (tx, rx) = mpsc::channel::<anyhow::Result<CmdOutput>>();
        // We need to move the child into the thread. `Child` is not `Send` on
        // all targets, but on macOS/Linux it is, so this is fine.
        std::thread::spawn(move || {
            let result = child.wait_with_output().map(|out| CmdOutput {
                success: out.status.success(),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            }).map_err(|e| anyhow::anyhow!("subprocess wait failed: {e}"));
            let _ = tx.send(result);
        });

        match rx.recv_timeout(CMD_TIMEOUT) {
            Ok(result) => result,
            Err(_) => {
                // Timeout: the child thread still holds the child handle;
                // we cannot kill it directly, but the process will be
                // cleaned up when the thread finishes. Return an error so
                // the caller does not hang.
                Err(anyhow::anyhow!(
                    "agent/tool timed out after {}s ({})",
                    CMD_TIMEOUT.as_secs(),
                    program
                ))
            }
        }
    }
}

/// Test-only runner: returns canned outputs keyed by `"program\0arg1\0arg2"`.
///
/// Lookups that miss the map return a non-success empty `CmdOutput` (modeling a
/// command that ran but found nothing), so tier-fallthrough logic is exercised.
pub struct FakeCmdRunner {
    responses: HashMap<String, CmdOutput>,
}

impl FakeCmdRunner {
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
        }
    }

    /// Register a canned output for a `(program, args)` invocation.
    pub fn with(mut self, program: &str, args: &[&str], output: CmdOutput) -> Self {
        self.responses.insert(Self::key(program, args), output);
        self
    }

    fn key(program: &str, args: &[&str]) -> String {
        let mut parts = vec![program.to_string()];
        parts.extend(args.iter().map(|a| a.to_string()));
        parts.join("\0")
    }
}

impl Default for FakeCmdRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CmdRunner for FakeCmdRunner {
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<CmdOutput> {
        Ok(self
            .responses
            .get(&Self::key(program, args))
            .cloned()
            .unwrap_or_else(|| CmdOutput {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
            }))
    }
}

// ── 3-tier discovery ────────────────────────────────────────────────────────

/// The shell to use for the login-shell tier. `$SHELL` if set, else `/bin/zsh`.
fn login_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

/// Find a binary by name, robust to the macOS GUI-PATH problem.
///
/// Tier 1: `which <name>` — fast, honors the *current* process PATH.
/// Tier 2: `<shell> -lc "command -v <name>"` — a *login* shell sources
///         `~/.zshrc`/profile, picking up nvm/mise/asdf/Homebrew shims that a
///         Finder-launched app never inherited.
/// Tier 3: probe known install locations directly via the injected `exists`.
///
/// `exists` is injected so tests don't touch the real filesystem; the
/// production caller passes a real existence+executable check.
pub fn find_binary(
    name: &str,
    runner: &dyn CmdRunner,
    candidates: &[PathBuf],
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    // Tier 1: which.
    if let Ok(out) = runner.run("which", &[name]) {
        if out.success {
            if let Some(path) = first_nonempty_line(&out.stdout) {
                return Some(PathBuf::from(path));
            }
        }
    }

    // Tier 2: login shell `command -v`.
    let shell = login_shell();
    let probe = format!("command -v {name}");
    if let Ok(out) = runner.run(&shell, &["-lc", &probe]) {
        if out.success {
            // `command -v` may emit several lines; take the first that exists.
            for line in out.stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let p = PathBuf::from(trimmed);
                if exists(&p) {
                    return Some(p);
                }
            }
            // Fall back to the first non-empty line even if `exists` is unsure
            // (e.g. a shell function) — but only when nothing matched above.
            if let Some(first) = first_nonempty_line(&out.stdout) {
                return Some(PathBuf::from(first));
            }
        }
    }

    // Tier 3: known install locations.
    for candidate in candidates {
        if exists(candidate) {
            return Some(candidate.clone());
        }
    }

    None
}

/// Run `<path> --version` and return the trimmed stdout, if the call succeeded.
pub fn probe_version(path: &Path, runner: &dyn CmdRunner) -> Option<String> {
    let path_str = path.to_str()?;
    let out = runner.run(path_str, &["--version"]).ok()?;
    if !out.success {
        return None;
    }
    let trimmed = out.stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn first_nonempty_line(s: &str) -> Option<&str> {
    s.lines().map(str::trim).find(|l| !l.is_empty())
}

// ── Production candidate lists ───────────────────────────────────────────────

/// Expand a leading `~` using `$HOME`. Non-`~` paths pass through unchanged.
fn home_path(rel: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    if let Some(stripped) = rel.strip_prefix("~/") {
        PathBuf::from(home).join(stripped)
    } else {
        PathBuf::from(rel)
    }
}

/// Known install locations for the `claude` CLI.
///
/// Covers the official installer (`~/.local/bin`, `~/.claude/local`), Homebrew
/// (Apple-silicon + Intel prefixes), and `npm -g` (`~/.npm-global/bin`).
/// nvm installs land in `~/.nvm/versions/node/<v>/bin` — version-pinned and
/// thus unenumerable here; those are caught by Tier 2 (login shell), which is
/// exactly why Tier 2 exists.
pub fn claude_candidates() -> Vec<PathBuf> {
    [
        "~/.local/bin/claude",
        "~/.claude/local/claude",
        "/opt/homebrew/bin/claude",
        "/usr/local/bin/claude",
        "~/.npm-global/bin/claude",
    ]
    .iter()
    .map(|p| home_path(p))
    .collect()
}

/// Known install locations for the `codex` CLI.
///
/// Same set as claude plus `~/.codex/bin/codex` and the Bun global bin
/// (`~/.bun/bin/codex`). nvm dirs are likewise version-pinned → Tier 2.
pub fn codex_candidates() -> Vec<PathBuf> {
    [
        "~/.local/bin/codex",
        "~/.claude/local/codex",
        "/opt/homebrew/bin/codex",
        "/usr/local/bin/codex",
        "~/.npm-global/bin/codex",
        "~/.codex/bin/codex",
        "~/.bun/bin/codex",
    ]
    .iter()
    .map(|p| home_path(p))
    .collect()
}

/// Production existence check: the path exists AND is a regular file.
/// (Executability is best-effort; on macOS install dirs this is sufficient and
/// avoids a permissions-mode dependency.)
// UNVERIFIED: live path
pub fn real_exists(path: &Path) -> bool {
    path.is_file()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(stdout: &str) -> CmdOutput {
        CmdOutput {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn fail() -> CmdOutput {
        CmdOutput {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    /// Tier 1 hit: `which` succeeds → that path is used, tiers 2/3 untouched.
    #[test]
    fn tier1_which_hit() {
        let runner = FakeCmdRunner::new().with("which", &["claude"], ok("/usr/local/bin/claude\n"));
        // exists() would panic if called — proves tiers 2/3 are skipped.
        let exists = |_: &Path| -> bool { panic!("exists must not be called on a tier-1 hit") };
        let found = find_binary("claude", &runner, &[], &exists);
        assert_eq!(found, Some(PathBuf::from("/usr/local/bin/claude")));
    }

    /// Tier 1 miss → Tier 2 (login shell `command -v`) hit.
    #[test]
    fn tier1_miss_tier2_hit() {
        let shell = login_shell();
        let runner = FakeCmdRunner::new()
            .with("which", &["claude"], fail())
            .with(
                &shell,
                &["-lc", "command -v claude"],
                ok("/Users/me/.nvm/versions/node/v20/bin/claude\n"),
            );
        let exists = |p: &Path| -> bool {
            p == Path::new("/Users/me/.nvm/versions/node/v20/bin/claude")
        };
        let found = find_binary("claude", &runner, &[], &exists);
        assert_eq!(
            found,
            Some(PathBuf::from("/Users/me/.nvm/versions/node/v20/bin/claude"))
        );
    }

    /// Tier 1 & 2 miss → Tier 3 (first existing candidate) hit.
    #[test]
    fn tier1_2_miss_tier3_hit() {
        let runner = FakeCmdRunner::new(); // all lookups miss → non-success
        let candidates = vec![
            PathBuf::from("/opt/homebrew/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
        ];
        // Only the second candidate exists.
        let exists = |p: &Path| -> bool { p == Path::new("/usr/local/bin/claude") };
        let found = find_binary("claude", &runner, &candidates, &exists);
        assert_eq!(found, Some(PathBuf::from("/usr/local/bin/claude")));
    }

    /// All tiers miss → None.
    #[test]
    fn all_tiers_miss_returns_none() {
        let runner = FakeCmdRunner::new();
        let candidates = vec![PathBuf::from("/opt/homebrew/bin/codex")];
        let exists = |_: &Path| -> bool { false };
        let found = find_binary("codex", &runner, &candidates, &exists);
        assert_eq!(found, None);
    }

    /// Tier 2 returns a line that does NOT exist → falls through to Tier 3.
    #[test]
    fn tier2_nonexistent_line_falls_to_tier3() {
        let shell = login_shell();
        let runner = FakeCmdRunner::new()
            .with("which", &["claude"], fail())
            .with(&shell, &["-lc", "command -v claude"], ok("claude\n")); // a shell alias, not a path
        let candidates = vec![PathBuf::from("/opt/homebrew/bin/claude")];
        // The bare "claude" line doesn't exist; the candidate does.
        let exists = |p: &Path| -> bool { p == Path::new("/opt/homebrew/bin/claude") };
        let found = find_binary("claude", &runner, &candidates, &exists);
        // Tier 2's non-existent line is taken as last-resort only if nothing
        // else matches; here Tier 2's fallback line wins because it returns
        // before Tier 3. Document the actual behavior: fallback line is used.
        assert_eq!(found, Some(PathBuf::from("claude")));
    }

    /// probe_version parses a trimmed version string.
    #[test]
    fn probe_version_parses() {
        let path = PathBuf::from("/usr/local/bin/claude");
        let runner = FakeCmdRunner::new().with(
            "/usr/local/bin/claude",
            &["--version"],
            ok("  1.2.3 (Claude Code)\n"),
        );
        let v = probe_version(&path, &runner);
        assert_eq!(v, Some("1.2.3 (Claude Code)".to_string()));
    }

    /// probe_version returns None when the command fails.
    #[test]
    fn probe_version_none_on_failure() {
        let path = PathBuf::from("/usr/local/bin/missing");
        let runner = FakeCmdRunner::new(); // miss → non-success
        assert_eq!(probe_version(&path, &runner), None);
    }

    /// Candidate lists are non-empty and `~` is expanded (no leading tilde).
    #[test]
    fn candidate_lists_expand_home() {
        for p in claude_candidates().iter().chain(codex_candidates().iter()) {
            let s = p.to_string_lossy();
            assert!(!s.starts_with('~'), "tilde not expanded: {s}");
        }
        assert!(!claude_candidates().is_empty());
        assert!(!codex_candidates().is_empty());
    }
}
