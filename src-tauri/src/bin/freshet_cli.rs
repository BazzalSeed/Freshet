//! `freshet_cli` — a headless harness to drive `engine::refresh` outside Tauri.
//!
//! Usage:
//!   freshet_cli refresh <root> <stream-id>           # live agent + HTTP providers
//!   freshet_cli refresh <root> <stream-id> --fake    # FakeAgent + FakeSourceProvider
//!
//! The `--fake` path uses no network and no agent process, so it runs in CI /
//! headless. The live path (no flag) is marked UNVERIFIED.

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
    let args: Vec<String> = std::env::args().collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> anyhow::Result<()> {
    // args[0] is the binary name.
    let cmd = args.get(1).map(String::as_str);
    if cmd != Some("refresh") {
        anyhow::bail!(
            "usage: freshet_cli refresh <root> <stream-id> [--fake]"
        );
    }

    let root = args
        .get(2)
        .ok_or_else(|| anyhow::anyhow!("missing <root>"))?;
    let stream_id = args
        .get(3)
        .ok_or_else(|| anyhow::anyhow!("missing <stream-id>"))?;
    let fake = args.iter().any(|a| a == "--fake");

    let root = Path::new(root);
    let desc = store::load_description(root, stream_id)?;
    let now = chrono::Utc::now().to_rfc3339();

    if fake {
        // Headless, deterministic path: no network, no agent process.
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

    // ── Live path ───────────────────────────────────────────────────────────
    // UNVERIFIED: live path — spawns a real agent + real HTTP fetches.
    let runner: Arc<dyn CmdRunner> = Arc::new(RealCmdRunner);
    let exists = |p: &Path| p.exists();
    let statuses = freshet_tmp_lib::agent::detect_agents(runner.as_ref(), &exists);
    let agent: Box<dyn Agent> = select_agent(None, &statuses, Arc::clone(&runner))
        .ok_or_else(|| anyhow::anyhow!("no usable agent found"))?;

    let http: Arc<dyn HttpClient> = Arc::new(ReqwestClient::new()?);
    let providers = registry(&desc.sources, http);

    let summary = engine::refresh(root, &desc, agent.as_ref(), &providers, &now)?;
    println!(
        "refresh {stream_id}: changed={} nNew={}",
        summary.changed, summary.n_new
    );
    Ok(())
}
