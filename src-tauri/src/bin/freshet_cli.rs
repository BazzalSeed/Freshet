//! `freshet_cli` — a headless harness to drive `engine::refresh` outside Tauri.
//!
//! Usage:
//!   freshet_cli refresh <root> <stream-id>               # real sources + real agent
//!                                                        # (honours FRESHET_FAKE_AGENT env)
//!   freshet_cli refresh <root> <stream-id> --fake-agent  # real sources + fake agent
//!   freshet_cli refresh <root> <stream-id> --fake        # fake sources + fake agent (offline)
//!
//! Set RUST_LOG (e.g. RUST_LOG=info) to control log verbosity; defaults to "info".

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use freshet_tmp_lib::agent::discovery::{CmdRunner, RealCmdRunner};
use freshet_tmp_lib::agent::fake::FakeAgent;
use freshet_tmp_lib::agent::{select_agent, Agent};
use freshet_tmp_lib::engine;
use freshet_tmp_lib::model::{AgentKind, SourceItem};
use freshet_tmp_lib::sources::{registry, FakeSourceProvider, HttpClient, ReqwestClient, SourceProvider};
use freshet_tmp_lib::store;

fn main() -> ExitCode {
    // Fix 1: initialise env_logger so all log::info!/warn!/error! calls reach stderr.
    // Defaults to "info" level when RUST_LOG is not set.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// Three-mode refresh:
/// - `--fake`       → fake sources + fake agent  (fully offline)
/// - `--fake-agent` → real sources + fake agent  (real HTTP, deterministic synthesis)
/// - (default)      → real sources + real agent  (honours FRESHET_FAKE_AGENT env)
fn run(args: &[String]) -> anyhow::Result<()> {
    let cmd = args.get(1).map(String::as_str);
    if cmd != Some("refresh") {
        anyhow::bail!(
            "usage: freshet_cli refresh <root> <stream-id> [--fake-agent | --fake]"
        );
    }

    let root = args
        .get(2)
        .ok_or_else(|| anyhow::anyhow!("missing <root>"))?;
    let stream_id = args
        .get(3)
        .ok_or_else(|| anyhow::anyhow!("missing <stream-id>"))?;

    let full_fake   = args.iter().any(|a| a == "--fake");
    let fake_agent  = args.iter().any(|a| a == "--fake-agent");

    let root = Path::new(root);
    let desc = store::load_description(root, stream_id)?;
    let now = chrono::Utc::now().to_rfc3339();

    // ── Mode: fully offline (fake sources + fake agent) ─────────────────────
    if full_fake {
        log::info!("mode: --fake (fake sources + fake agent, fully offline)");
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let providers: Vec<Box<dyn SourceProvider>> = vec![Box::new(FakeSourceProvider::new(
            "hackernews",
            vec![SourceItem {
                id: format!("fake:{stream_id}:1"),
                source: "hackernews".into(),
                url: "https://example.com/fake".into(),
                title: format!("Fake item for {stream_id}"),
                score: Some(1.0),
                snippet: "Synthetic item produced by --fake.".into(),
                created_at: None,
            }],
        ))];
        let summary = engine::refresh(root, &desc, &agent, &providers, &now)?;
        println!(
            "[--fake] refresh {stream_id}: changed={} nNew={}",
            summary.changed, summary.n_new
        );
        return Ok(());
    }

    // ── Real HTTP providers (shared by --fake-agent and default) ────────────
    let http: Arc<dyn HttpClient> = Arc::new(ReqwestClient::new()?);
    let providers = registry(&desc.sources, http);

    // ── Mode: real sources + fake agent ─────────────────────────────────────
    if fake_agent {
        log::info!("mode: --fake-agent (real sources + deterministic fake agent)");
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let summary = engine::refresh(root, &desc, &agent, &providers, &now)?;
        println!(
            "[--fake-agent] refresh {stream_id}: changed={} nNew={}",
            summary.changed, summary.n_new
        );
        return Ok(());
    }

    // ── Mode: real sources + real agent (default) ────────────────────────────
    // Honour FRESHET_FAKE_AGENT env the same way the Tauri app does.
    if std::env::var("FRESHET_FAKE_AGENT").is_ok() {
        log::info!("mode: default (real sources + fake agent via FRESHET_FAKE_AGENT env)");
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let summary = engine::refresh(root, &desc, &agent, &providers, &now)?;
        println!(
            "[FRESHET_FAKE_AGENT] refresh {stream_id}: changed={} nNew={}",
            summary.changed, summary.n_new
        );
        return Ok(());
    }

    log::info!("mode: default (real sources + real agent)");
    let runner: Arc<dyn CmdRunner> = Arc::new(RealCmdRunner);
    let exists = |p: &Path| p.exists();
    let statuses = freshet_tmp_lib::agent::detect_agents(runner.as_ref(), &exists);
    let agent: Box<dyn Agent> = select_agent(None, &statuses, Arc::clone(&runner))
        .ok_or_else(|| anyhow::anyhow!("no usable agent found (is 'claude' installed?)"))?;

    let summary = engine::refresh(root, &desc, agent.as_ref(), &providers, &now)?;
    println!(
        "refresh {stream_id}: changed={} nNew={}",
        summary.changed, summary.n_new
    );
    Ok(())
}
