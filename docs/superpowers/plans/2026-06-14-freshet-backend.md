# Freshet Backend Implementation Plan

> **For agentic workers:** execute with superpowers:subagent-driven-development (controller dispatches a fresh subagent per task + reviews). Steps use `- [ ]`.
>
> **Verifiability rule for tonight:** everything is built TDD against **fakes + recorded fixtures** — NO live agent, NO live network in tests/CI. The real `claude`/`codex` invocation and real channel HTTP are written but marked `// UNVERIFIED: live path` and exercised only by the owner later. Don't fire real subprocesses/network in automated runs.

**Goal:** Replace `MockBridge` with a real Rust backend in the native app — Store, free-channel `SourceProvider`s, agent detection + headless adapters, the watch engine, the scheduler, and the `TauriBridge` — so streams really refresh, create, and persist. UI keeps using `MockBridge` in the browser.

**Architecture:** Three seams (`Bridge`, `Agent`, `SourceProvider`); the engine depends on traits, faked in tests. See the [Backend & Core Features spec](../specs/2026-06-14-backend-and-core-features-design.md) (§3 crate layout, §4 modules, §7 bridge). Agent detection blueprinted on Tolaria (3-tier PATH-robust + macOS GUI-PATH fix).

**Tech Stack:** Rust (serde, serde_json, anyhow, thiserror, tempfile, chrono, tokio, reqwest, sha2), Tauri 2 (+ plugins: dialog, store, fs, window-state), React/TS frontend (existing).

**Decisions (owner, 2026-06-14):** fetch free channels ourselves (agent-agnostic); no-agent → guide to install; full build, live path unverified.

---

## Phase 0 — Rust deps, types, plugins

### Task 0.1: Dependencies + Tauri plugins
- [ ] Add crates: `cargo add serde --features derive; cargo add serde_json anyhow thiserror tempfile chrono sha2 reqwest --features reqwest/json,reqwest/blocking; cargo add tokio --features rt-multi-thread,macros,time,process; cargo add fix-path-env --git https://github.com/tauri-apps/fix-path-env-rs`.
- [ ] Add Tauri plugins (JS + Rust): `npm i @tauri-apps/plugin-dialog @tauri-apps/plugin-store @tauri-apps/plugin-fs @tauri-apps/plugin-window-state` and the matching `tauri-plugin-*` crates; register them in `main.rs`.
- [ ] `cargo build` clean; commit `chore(backend): deps + tauri plugins`.

### Task 0.2: `model.rs` — shared Rust types (mirror the bridge contract)
- [ ] Define (serde, `#[serde(rename_all="snake_case")]` on enums): `CadenceMode`, `Cadence`, `StreamStatus`, `StreamDescription`, `StreamSummary`, `SourceItem`, `StreamState`, `Summary`, `AgentKind`, `AgentStatus`. Field names match the TS contract (snake_case across the bridge — the TS side maps to camelCase or we set serde rename; **pick snake_case on the wire and update `src/bridge/types.ts` to match, or add `#[serde(rename_all="camelCase")]` — choose one and document it**).
- [ ] serde round-trip tests for each. `cargo test model::` green. Commit `feat(backend): shared model types`.

---

## Phase 1 — Store

### Task 1.1: paths + atomic writes (`store/mod.rs`)
- [ ] `write_atomic(path,&str)` (temp+fsync+rename; no `.tmp` left), `read_opt`, path helpers (`doc_path(root,title)`, `streams_dir`, `state_path`, `config_path`, `history_dir`). Tests via `tempfile::tempdir`. Commit.

### Task 1.2: config + descriptions + state load/save
- [ ] `Config { root, selected_agent: Option<AgentKind>, onboarded: bool }`; `load/save_config`, `load/save_description`, `load/save_state` (serde_json + helpers), each round-trip tested through a tempdir. Commit.

### Task 1.3: document I/O — model B My-notes splice (`store/document.rs`)
- [ ] `read_document`, `write_document` (atomic), and `splice_my_notes(doc, new_block) -> String` that replaces ONLY the `## My notes` block (the last block; footnote defs live above it) leaving the Freshet-owned prefix **byte-identical**. Mirror the proven frontend logic.
- [ ] Tests: splice preserves prefix byte-for-byte incl. `[^id]:` defs; if `## My notes` absent, append it. Commit `feat(backend): store — atomic IO, config, document my-notes splice`.

### Task 1.4: history snapshots (`store/history.rs`)
- [ ] `snapshot(root,id,doc)` → write `.freshet/history/<id>/<iso>.md` + update a small JSON index; `list_history(root,id)`. (Use a timestamp passed in — no `Date::now` in tested code paths; the caller stamps.) Tests in tempdir. Commit.

---

## Phase 2 — SourceProvider (free channels)

### Task 2.1: trait + types + registry (`sources/mod.rs`)
- [ ] `SourceItem` (source-qualified `id`, source, url, title, score, snippet, created_at); `trait SourceProvider { fn channel(&self)->&str; fn fetch(&self,topic:&str,limit:usize)->anyhow::Result<Vec<SourceItem>>; }`; a `registry(channels:&[String]) -> Vec<Box<dyn SourceProvider>>`; `fetch_all(providers, topic) -> Vec<SourceItem>` that runs providers concurrently and **degrades gracefully** (one channel erroring is skipped, not fatal). `FakeSourceProvider` returning fixture items. Tests for registry mapping + graceful-degrade with a failing fake. Commit.

### Task 2.2: four fetchers (parse fixtures; live HTTP behind a seam)
- [ ] `hackernews.rs reddit.rs github.rs polymarket.rs`. Each: a pure `parse_<channel>(json:&str)->Vec<SourceItem>` (source-qualified ids, ranked by the channel signal) tested against a **recorded fixture** in `src-tauri/tests/fixtures/`, plus a `fetch` that builds the URL and calls an injected HTTP client (real `reqwest` in prod marked `// UNVERIFIED: live path`, a fake in tests). No live network in tests.
- [ ] Commit `feat(backend): source providers (HN/Reddit/GitHub/Polymarket) over fixtures`.

---

## Phase 3 — Agent layer

### Task 3.1: discovery (`agent/discovery.rs`) — 3-tier, injectable
- [ ] `find_binary(name, runner:&dyn CmdRunner) -> Option<PathBuf>` implementing tier1 `which`, tier2 `$SHELL -lc "command -v"`, tier3 hardcoded candidates (verify exists+executable). `CmdRunner` trait injected so tests use a fake (no real processes). `probe_version`. Tests cover each tier via the fake runner. Commit.

### Task 3.2: Agent trait + detection + adapters (`agent/`)
- [ ] `trait Agent { fn kind(&self)->AgentKind; fn synthesize(&self,input:ResearchInput)->anyhow::Result<String>; fn chat(&self,system:&str,history:&[ChatTurn])->anyhow::Result<ChatReply>; }`; `detect_agents(runner)->Vec<AgentStatus>` (parallel in prod; sequential w/ fake runner in tests); `select_agent`.
- [ ] `claude.rs`/`codex.rs`: build the headless argv (`claude -p … --bare --output-format stream-json …`; `codex exec --json …`) and drive via an injected `CmdRunner`; `// UNVERIFIED: live path` on the real spawn. `FakeAgent` (canned synthesize/chat) for engine tests.
- [ ] `main.rs`: `fix_path_env::fix()` at top.
- [ ] Tests: discovery tiers; argv construction; FakeAgent. Commit `feat(backend): agent discovery + claude/codex adapters (fake-runner tested)`.

---

## Phase 4 — Engine

### Task 4.1: refresh loop (`engine/mod.rs` + `reconcile.rs`)
- [ ] `refresh(root,&StreamDescription,&dyn Agent,&[Box<dyn SourceProvider>],now:&str)->anyhow::Result<Summary>` per spec §4.4: fetch_all → dedup (source-qualified vs `seen_item_ids`) → if none new: record `last_checked_at`, return `{changed:false}` (no agent call) → else: extract My-notes, `agent.synthesize`, reconcile, snapshot history, atomic-write doc, update state, return `{changed:true,n_new}`. `reconcile.rs` builds the prompt + assembles the doc preserving My-notes verbatim.
- [ ] Integration test (FakeAgent+FakeSource): refresh twice → run1 populates, identical run2 = nothing-new + **zero synthesize calls**, run2+1 extra item → appears under What-changed; `## My notes` survives all. Commit `feat(backend): watch engine (refresh loop)`.

---

## Phase 5 — Scheduler
- [ ] `is_due(mode,interval_minutes,last_checked_at,now)->bool` (pure, tested: interval elapsed, on_launch when never checked, manual never auto). A `tokio` tick task (wired in Phase 6) spawning background refreshes for due active streams. Commit `feat(backend): scheduler is_due + tick`.

---

## Phase 6 — TauriBridge + CLI harness

### Task 6.1: commands + events (`bridge.rs`, `main.rs`)
- [ ] Implement the full §7 command set over store/engine/sources/agent/scheduler; `refresh_stream`/creation run on background tasks; emit `refresh_progress`/`stream_updated`/`agents_changed`; map `anyhow→String`. Start the scheduler tick + deferred agent detection in `setup`.
- [ ] Test command bodies directly (call the Rust fns with a tempdir + FakeAgent/FakeSource), not through the JS layer. Commit.

### Task 6.2: CLI harness (`bin/freshet_cli.rs`)
- [ ] `freshet_cli refresh <root> <stream-id> [--fake]` driving `engine::refresh` headlessly (the integration-test driver). Commit.

---

## Phase 7 — Frontend wiring

### Task 7.1: `TauriBridge` + auto-select
- [ ] `src/bridge/TauriBridge.ts` implementing `Bridge` over `@tauri-apps/api` invoke/listen (+ the new onboarding/creation commands). `BridgeProvider` auto-selects `TauriBridge` when `window.__TAURI__` (or `@tauri-apps/api` import) is present, else `MockBridge`. Reconcile field casing with `model.rs` (Phase 0.2 decision). Tests: provider picks Mock in jsdom. Commit.
- [ ] Wire the existing creation **form** + desk refresh to the real bridge (works once native). Confirm `npm run test` (50 + new) green and `npm run tauri dev` runs against the real backend (manual, owner-verified).

---

## Phase 8 — Onboarding surface (stretch)
- [ ] Per spec §5: a dedicated onboarding view gated by `get_onboarding_state` — Welcome → folder picker (`set_root_folder`) → agent state (found/none/unreachable, `recheck_agents`) → first stream. Calm empty-states. Build against `MockBridge` first (mock returns onboarding states), then real. Component tests for each state. Commit.

---

## Phase 9 — Chat creation surface (stretch)
- [ ] Per spec §6: a creation chat view using `start_creation`/`creation_message`/`creation_preview`/`creation_commit`, streaming via `creation_progress`; emits an editable proposed-description card (the existing form) + a first-draft preview. Mock-first, then real. Tests against the mock. Commit.

---

## Phase 10 — NIGHT-SUMMARY
- [ ] Append `docs/NIGHT-SUMMARY.md`: what landed, test counts, what's verified vs `// UNVERIFIED: live path`, decisions made, where I stopped, and the exact next steps for the owner (esp. running the live agent/network path). Commit.

---

## Acceptance (backend phase)
- [ ] `cargo test` green (model, store, sources-over-fixtures, discovery, engine integration, scheduler); `npm run test` still green.
- [ ] `npm run tauri build`/`tauri dev` compiles with the real bridge.
- [ ] The engine's two-run "what changed / nothing new" sequence passes against FakeAgent+FakeSource, My-notes preserved.
- [ ] Live agent/network paths exist, compile, and are clearly marked unverified.

## Self-review notes
- **Seam discipline:** engine depends on `Agent`/`SourceProvider` traits only; both faked in tests — the whole core is verifiable without a live agent/network.
- **Contract parity:** `model.rs` ↔ `src/bridge/types.ts` field casing reconciled once (Phase 0.2 / 7.1).
- **Unverified marks:** every real subprocess/HTTP call carries `// UNVERIFIED: live path`; the NIGHT-SUMMARY lists them.
