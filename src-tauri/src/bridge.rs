//! The TauriBridge: thin `#[tauri::command]` wrappers + managed state + events.
//!
//! Each wrapper resolves managed state (the app-config dir, the cached
//! `AppConfig`, the real `CmdRunner` / `HttpClient`), calls the corresponding
//! plain function in [`crate::commands`], and maps `anyhow::Error -> String`
//! (Tauri commands must return `Result<_, E: Serialize>`).
//!
//! The wiring that touches the *live* world — selecting and constructing a real
//! agent, building HTTP-backed source providers, spawning child processes —
//! is marked `// UNVERIFIED: live path`. The plain command fns it calls are all
//! unit-tested with `FakeAgent` + `FakeSourceProvider`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::agent::discovery::CmdRunner;
use crate::agent::{detect_agents, select_agent, Agent};
use crate::commands::{self, AppConfig, DraftInput, DraftResult, GetStreamResult, OnboardingState};
use crate::model::{AgentKind, AgentStatus, StreamDescription, StreamStatus, StreamSummary, Summary};
use crate::scheduler::{due_for_tick, runs_at_startup};
use crate::sources::{registry, HttpClient, SourceProvider};
use crate::store;

// ── Managed state ───────────────────────────────────────────────────────────

/// All long-lived backend state Tauri manages for us.
pub struct BackendState {
    /// Where the app-level `config.json` lives (`app_config_dir()`).
    pub config_dir: PathBuf,
    /// Cached app config, kept in sync with disk under a Mutex.
    pub config: Mutex<AppConfig>,
    /// The real process runner used for agent detection + spawning.
    // UNVERIFIED: live path
    pub runner: Arc<dyn CmdRunner>,
    /// The real HTTP client used to build source providers.
    // UNVERIFIED: live path
    pub http: Arc<dyn HttpClient>,
}

impl BackendState {
    /// Resolve the vault root from the cached config, or error if onboarding
    /// hasn't set one yet.
    fn root(&self) -> anyhow::Result<PathBuf> {
        let cfg = self.config.lock().expect("config mutex poisoned");
        cfg.root
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("no root folder set; complete onboarding first"))
    }

    /// The currently selected agent kind, if any.
    fn selected_agent(&self) -> Option<AgentKind> {
        self.config
            .lock()
            .expect("config mutex poisoned")
            .selected_agent
    }
}

/// Map any `anyhow::Error` into the `String` Tauri commands return on failure.
fn estr<T>(r: anyhow::Result<T>) -> Result<T, String> {
    r.map_err(|e| format!("{e:#}"))
}

/// Current wall-clock as an RFC-3339 string.
// UNVERIFIED: live path (reads the real clock; plain fns take `now` explicitly)
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Live resolution helpers (UNVERIFIED) ────────────────────────────────────

/// Run agent detection with the real runner + real filesystem existence check.
// UNVERIFIED: live path
fn detect_live(runner: &dyn CmdRunner) -> Vec<AgentStatus> {
    let exists = |p: &std::path::Path| p.exists();
    detect_agents(runner, &exists)
}

/// Select and construct a live agent for `desc`'s configured preference.
///
/// When `FRESHET_FAKE_AGENT=1` (or `true`) is set, returns a `FakeAgent` that
/// synthesizes a deterministic living document from the given items without
/// calling any real LLM. This lets the full app run end-to-end with no Claude
/// auth required.
// UNVERIFIED: live path
fn resolve_agent(state: &BackendState) -> anyhow::Result<Box<dyn Agent>> {
    let fake_flag = std::env::var("FRESHET_FAKE_AGENT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if fake_flag {
        log::warn!(
            "FRESHET_FAKE_AGENT active — using deterministic fake agent (no real LLM)"
        );
        use crate::agent::fake::FakeAgent;
        use crate::model::AgentKind;
        return Ok(Box::new(FakeAgent::reflecting(AgentKind::ClaudeCode)));
    }

    let statuses = detect_live(state.runner.as_ref());
    select_agent(state.selected_agent(), &statuses, Arc::clone(&state.runner))
        .ok_or_else(|| anyhow::anyhow!("no usable agent found; check agent installation"))
}

/// Build HTTP-backed providers for the given channel names.
// UNVERIFIED: live path
fn resolve_providers(state: &BackendState, channels: &[String]) -> Vec<Box<dyn SourceProvider>> {
    registry(channels, Arc::clone(&state.http))
}

// ── Config / onboarding / agents ────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, BackendState>) -> AppConfig {
    state.config.lock().expect("config mutex poisoned").clone()
}

#[tauri::command]
pub fn get_onboarding_state(state: State<'_, BackendState>) -> OnboardingState {
    let cfg = state.config.lock().expect("config mutex poisoned").clone();
    // UNVERIFIED: live path — detection runs the real runner.
    let agents = detect_live(state.runner.as_ref());
    commands::onboarding_state(&cfg, &agents)
}

#[tauri::command]
pub fn list_agents(state: State<'_, BackendState>) -> Vec<AgentStatus> {
    // UNVERIFIED: live path
    detect_live(state.runner.as_ref())
}

#[tauri::command]
pub fn recheck_agents(state: State<'_, BackendState>) -> Vec<AgentStatus> {
    // UNVERIFIED: live path
    detect_live(state.runner.as_ref())
}

#[tauri::command]
pub fn set_root_folder(state: State<'_, BackendState>, path: String) -> Result<(), String> {
    let root = PathBuf::from(&path);
    estr(commands::set_root_folder(&state.config_dir, &root))?;
    // Refresh the in-memory cache from disk.
    *state.config.lock().expect("config mutex poisoned") =
        commands::load_app_config(&state.config_dir);
    Ok(())
}

#[tauri::command]
pub fn set_default_agent(state: State<'_, BackendState>, kind: AgentKind) -> Result<(), String> {
    estr(commands::set_default_agent(&state.config_dir, kind))?;
    *state.config.lock().expect("config mutex poisoned") =
        commands::load_app_config(&state.config_dir);
    Ok(())
}

#[tauri::command]
pub fn complete_onboarding(state: State<'_, BackendState>) -> Result<(), String> {
    estr(commands::complete_onboarding(&state.config_dir))?;
    *state.config.lock().expect("config mutex poisoned") =
        commands::load_app_config(&state.config_dir);
    Ok(())
}

// ── Streams ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_streams(state: State<'_, BackendState>) -> Result<Vec<StreamSummary>, String> {
    let root = estr(state.root())?;
    Ok(commands::list_streams(&root))
}

#[tauri::command]
pub fn get_stream(state: State<'_, BackendState>, id: String) -> Result<GetStreamResult, String> {
    let root = estr(state.root())?;
    estr(commands::get_stream(&root, &id))
}

#[tauri::command]
pub fn save_notes(
    state: State<'_, BackendState>,
    id: String,
    markdown: String,
) -> Result<(), String> {
    let root = estr(state.root())?;
    estr(commands::save_notes(&root, &id, &markdown))
}

#[tauri::command]
pub fn set_stream_status(
    state: State<'_, BackendState>,
    id: String,
    status: StreamStatus,
) -> Result<(), String> {
    let root = estr(state.root())?;
    estr(commands::set_stream_status(&root, &id, status))
}

#[tauri::command]
pub fn generate_first_draft(
    state: State<'_, BackendState>,
    input: DraftInput,
) -> Result<DraftResult, String> {
    // UNVERIFIED: live path — resolves a live agent + HTTP providers.
    let agent = estr(resolve_agent(&state))?;
    let providers = resolve_providers(&state, &input.sources);
    estr(commands::generate_first_draft(&input, agent.as_ref(), &providers, &now_iso()))
}

#[tauri::command]
pub fn create_stream(
    state: State<'_, BackendState>,
    description: StreamDescription,
) -> Result<StreamSummary, String> {
    let root = estr(state.root())?;
    // UNVERIFIED: live path
    let agent = estr(resolve_agent(&state))?;
    let providers = resolve_providers(&state, &description.sources);
    estr(commands::create_stream(&root, &description, agent.as_ref(), &providers, &now_iso()))
}

/// Refresh a stream, awaiting the result and returning its [`Summary`].
///
/// The heavy work runs on a blocking task (off the UI thread). Because this is
/// an `async` Tauri command, awaiting it does NOT block the webview — the
/// frontend stays responsive while progress flows via `refresh_progress` and a
/// terminal `stream_updated {streamId, changed}` event, then the command
/// resolves to the `Summary` the caller awaits.
#[tauri::command]
pub async fn refresh_stream(app: AppHandle, id: String) -> Result<Summary, String> {
    // Run on a blocking task so the engine work never blocks the async runtime,
    // then await its result. UNVERIFIED: live path — runs a live agent + HTTP
    // providers in a blocking task.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<BackendState>();
        run_refresh_emitting(&app, &state, &id)
    })
    .await
    .map_err(|e| format!("refresh task panicked: {e}"))?;
    estr(result)
}

/// The body of a refresh: resolve root/agent/providers, run the engine, and
/// emit `refresh_progress` + `stream_updated` events throughout. Returns the
/// [`Summary`] so callers that await it (the command) get a result; the
/// fire-and-forget background tasks ignore it.
// UNVERIFIED: live path
fn run_refresh_emitting(
    app: &AppHandle,
    state: &BackendState,
    id: &str,
) -> anyhow::Result<Summary> {
    let _ = app.emit("refresh_progress", RefreshProgress { stream_id: id.into(), phase: "researching".into() });

    let result = (|| {
        let root = state.root()?;
        let desc = store::load_description(&root, id)?;
        let agent = resolve_agent(state)?;
        let providers = resolve_providers(state, &desc.sources);

        let _ = app.emit("refresh_progress", RefreshProgress { stream_id: id.into(), phase: "synthesizing".into() });

        crate::engine::refresh(&root, &desc, agent.as_ref(), &providers, &now_iso())
    })();

    match &result {
        Ok(summary) => {
            let _ = app.emit("refresh_progress", RefreshProgress { stream_id: id.into(), phase: "done".into() });
            let _ = app.emit("stream_updated", StreamUpdated { stream_id: id.into(), changed: summary.changed });
        }
        Err(e) => emit_error(app, id, &format!("{e:#}")),
    }
    result
}

fn emit_error(app: &AppHandle, id: &str, _msg: &str) {
    let _ = app.emit("refresh_progress", RefreshProgress { stream_id: id.into(), phase: "error".into() });
    let _ = app.emit("stream_updated", StreamUpdated { stream_id: id.into(), changed: false });
}

// ── Event payloads ──────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshProgress {
    stream_id: String,
    phase: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamUpdated {
    stream_id: String,
    changed: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentsChanged {
    agents: Vec<AgentStatus>,
}

// ── Background startup tasks ─────────────────────────────────────────────────

/// Spawn the deferred-detection, startup-refresh, and per-minute-tick tasks.
///
/// All work runs off the UI thread (honoring the non-blocking invariant). This
/// is called once from `setup`.
// UNVERIFIED: live path — every branch here drives the live runner/HTTP/agent.
pub fn spawn_background_tasks(app: AppHandle) {
    // (a) Deferred agent detection shortly after start → emit `agents_changed`.
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let state = app.state::<BackendState>();
            let agents = detect_live(state.runner.as_ref());
            let _ = app.emit("agents_changed", AgentsChanged { agents });
        });
    }

    // (b) Startup refresh pass for active streams whose mode `runs_at_startup`.
    {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let state = app.state::<BackendState>();
            let Ok(root) = state.root() else { return };
            for desc in store::list_descriptions(&root) {
                if desc.status == StreamStatus::Active && runs_at_startup(&desc.cadence.mode) {
                    // Fire-and-forget: events carry the outcome; ignore the Summary.
                    let _ = run_refresh_emitting(&app, &state, &desc.id);
                }
            }
        });
    }

    // (c) Per-minute scheduler tick: refresh due streams in the background.
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let app2 = app.clone();
                // Off-thread blocking work so the timer loop stays responsive.
                tauri::async_runtime::spawn_blocking(move || {
                    let state = app2.state::<BackendState>();
                    let Ok(root) = state.root() else { return };
                    let now = now_iso();
                    for desc in store::list_descriptions(&root) {
                        let st = store::load_state(&root, &desc.id);
                        if due_for_tick(&desc, &st, &now) {
                            // Fire-and-forget: events carry the outcome; ignore the Summary.
                            let _ = run_refresh_emitting(&app2, &state, &desc.id);
                        }
                    }
                });
            }
        });
    }
}
