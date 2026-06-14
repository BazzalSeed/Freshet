# Freshet v1 — Backend & Core Features Design

*Status: design (2026-06-14), written autonomously overnight per the owner's request. Extends the
[v1 design spec](./2026-06-12-freshet-v1-design.md) and the [frontend feel brief](./2026-06-13-frontend-feel-design.md).
The frontend (desk · reading view · creation form · theming) is **built and mock-backed**; this
spec covers the **remaining work to a real, shippable v1**: the light Rust backend, plus the two
upgrades the owner asked for — **first-run onboarding** and **chat-based stream creation**.*

> **Decisions locked with the owner (2026-06-14):**
> 1. **Sourcing = fetch the free channels ourselves** (agent-agnostic, verifiable), behind a
>    pluggable `SourceProvider`. `last30days`/richer polling is a future provider (todo).
> 2. **No agent detected → guide the user to install one** (Claude Code / Codex), then re-detect.
>    No API-key handling in v1.
> 3. **Build scope tonight = full build**; the **live-agent + live-network path ships UNVERIFIED**
>    (can't be safely run unattended) — marked clearly, to be debugged with the owner.

---

## 1. What this adds

The frontend talks only to a `Bridge`. Today there's one implementation, `MockBridge` (sample
data). This spec defines the **real backend** that replaces it in the native app, plus two surfaces
the v1 spec under-specified:

| # | Feature | This spec |
| :--- | :--- | :--- |
| A | **First-run onboarding** | §5 — welcome → root folder → agent detection → first stream |
| B | **The light Rust backend** | §4 — Store · Agent layer · SourceProvider · Engine · Scheduler · TauriBridge |
| C | **Stream creation as a chat** | §6 — collaborative scoping → description + first draft |
| D | **Refresh each stream** | §4.4 — the watch loop, wired end-to-end |
| E | **Config / agent selection** | §4.1, §4.2, §7 |

Heavy lifting (research synthesis, the creation conversation) is the **agent's**; Rust is **files +
state + scheduling + spawning the agent + small channel fetchers**. That's the "light backend."

## 2. Core architecture — three parts, three seams

> **React = the face · Rust = the hands · the agent = the brain.**

The three seams that keep this decoupled and testable (all faked in tests):

- **`Bridge`** (Tauri commands/events) — React ↔ Rust. Already defined (v1 §5.5), extended here (§7).
- **`Agent`** (trait) — Rust ↔ the local coding agent. Detection + headless invocation (§4.2).
- **`SourceProvider`** (trait) — Rust ↔ the channels. Free-channel fetchers in v1 (§4.3).

The engine depends on `Agent` + `SourceProvider` traits, never concrete types — so it runs against
fakes in tests and real impls in the app.

## 3. Rust crate layout

```
src-tauri/src/
  main.rs                 # Tauri entry; fix-path-env; register commands; start scheduler
  bridge.rs               # #[tauri::command] fns + event emit (the real Bridge)
  model.rs                # shared types (StreamDescription, StreamState, SourceItem, …)
  store/
    mod.rs                # paths, atomic writes, config
    document.rs           # living-doc read/write + the `## My notes` splice (model B)
    history.rs            # timestamped snapshots (.freshet/history/)
  agent/
    mod.rs                # Agent trait, AgentKind, AgentStatus, detect_agents()
    discovery.rs          # three-tier PATH-robust binary discovery + shell-env probe
    claude.rs             # Claude Code adapter (claude -p, stream-json)
    codex.rs              # Codex adapter (codex exec --json)
    fake.rs               # FakeAgent for tests
  sources/
    mod.rs                # SourceProvider trait, SourceItem; registry by channel
    hackernews.rs reddit.rs github.rs polymarket.rs
    fake.rs               # fixture-backed provider for tests
  engine/
    mod.rs                # refresh() watch loop: research → dedup → reconcile
    reconcile.rs          # prompt construction + My-notes-preserving merge
  scheduler.rs            # manual / on_launch / interval (tokio)
  onboarding.rs           # first-run state + readiness checks
  creation.rs             # chat-based stream creation sessions
  bin/freshet_cli.rs      # headless harness (also the integration-test driver)
```

The Rust core is a **library + a thin CLI harness**, so the engine/agent/sources are exercisable
headlessly (and in CI) without the window.

## 4. The light Rust backend

### 4.1 Store — files, config, state, history

The user configures one **root folder**; Freshet maintains structure inside it (v1 spec §10):
visible `<root>/<Title>.md` documents; hidden `<root>/.freshet/` for config, stream descriptions,
per-stream state, and history snapshots.

- **Atomic writes** — temp file → `fsync` → rename, so a crash never leaves a torn document/state.
- **Config** (`.freshet/config.json` + a `tauri-plugin-store` key for onboarding/window state):
  `{ root, selected_agent: AgentKind?, onboarded: bool, prefs }`.
- **Root-folder access (macOS)** — persist the chosen folder as a **security-scoped bookmark** and
  re-activate it each launch; a sandboxed app can't reach an arbitrary path otherwise (the `NSOpenPanel`
  grant expires on quit). Build for the bookmark from the start even if dev runs un-sandboxed.
- **Document I/O** preserves model B: read the doc, splice **only** the `## My notes` block on
  `save_notes`, keep all Freshet-owned content (incl. footnote citation defs) byte-identical. (The
  frontend already proved this contract; the Rust side mirrors it — `## My notes` is the last block.)
- **History** (`.freshet/history/<id>/<iso>.md` + an index) — snapshot the document before each
  overwrite; supports a future "history" view, refresh-to-refresh diff, and revert (v1 §10).

### 4.2 Agent layer — detection + headless invocation (agent-agnostic)

Blueprinted on Tolaria (same stack). The **macOS GUI-PATH problem is solved first**: a
Finder-launched app doesn't inherit the user's shell PATH, so `claude`/`codex` installed via
npm/nvm/Homebrew appear "missing." Fix at startup with **`fix-path-env`** (`fix_path_env::fix()` at
the top of `main()`), and additionally a **shell-env probe** to pull provider env vars (e.g.
`ANTHROPIC_API_KEY`) from `~/.zshrc` only if absent.

**Three-tier binary discovery** per agent (`discovery.rs`):
1. `which <bin>` on the current PATH.
2. Login-shell fallback: `$SHELL -lc "command -v <bin>"` (sources `~/.zshrc`/`~/.bash_profile`;
   catches nvm/mise/asdf installs). ~200–500 ms — acceptable because detection is **deferred + parallel**.
3. Hardcoded candidate paths (`/opt/homebrew/bin`, `~/.local/bin`, `~/.claude/local/`, nvm node
   dirs, etc.); verify `.exists()` + executable bit.

Then a `--version` probe. **Run all agents' probes concurrently** (`tokio::spawn_blocking` +
`join`), each under a ~5 s timeout, **after first paint** (never on the cold-start path).

```rust
enum AgentKind { ClaudeCode, Codex }            // extensible
struct AgentStatus { kind: AgentKind, available: bool, version: Option<String>, path: Option<PathBuf> }

#[derive(Clone)]
struct ResearchInput<'a> { topic: &'a str, items: &'a [SourceItem], prior_doc: Option<&'a str> }

trait Agent: Send + Sync {
    fn kind(&self) -> AgentKind;
    /// Synthesize/reconcile a living document from fetched items + the prior doc. (Brain.)
    fn synthesize(&self, input: ResearchInput) -> anyhow::Result<String>;
    /// Free-form turn for the creation chat (§6); streams via the event bus.
    fn chat(&self, system: &str, history: &[ChatTurn]) -> anyhow::Result<ChatReply>;
}

fn detect_agents() -> Vec<AgentStatus>;
fn select_agent(prefs: &Config, available: &[AgentStatus]) -> Option<Box<dyn Agent>>;
```

> **Note the seam shift from v1 §5.4:** the `Agent` no longer does `research()` — sourcing moved to
> `SourceProvider` (decision 1). The agent's job is **synthesize** (and **chat**). This is what makes
> it truly agent-agnostic: any LLM agent can synthesize from given items.

**Headless invocation** (`claude.rs`): `claude -p <prompt> --bare --output-format stream-json
--verbose --include-partial-messages --permission-mode dontAsk --allowedTools "Read"` — `--bare` for
a hermetic call (skip CLAUDE.md/hooks/skills), stream-json for token-by-token events relayed to the
UI as `refresh_progress`/chat events. `codex.rs`: `codex exec <prompt> --json --sandbox read-only`.
A `fallback_args` retry handles older `claude` flag differences. **Synthesis needs no tools/network
from the agent** — we hand it the items as text; it just writes prose. (Lower risk + works offline-ish.)

**Fallback ladder (decision 3):** detected agent → (future: local model → API key) → **none →
onboarding "install an agent" prompt**. v1 implements the detected-agent rung only.

### 4.3 SourceProvider — the free channels (agent-agnostic sourcing)

```rust
struct SourceItem {            // the dedup + citation contract (source-qualified id)
    id: String,                // "hackernews:123", "reddit:abc", "github:owner/repo#42"
    source: String, url: String, title: String,
    score: Option<f64>, snippet: String, created_at: Option<String>,
}
trait SourceProvider: Send + Sync {
    fn channel(&self) -> &str;                                   // "hackernews" | …
    fn fetch(&self, topic: &str, limit: usize) -> anyhow::Result<Vec<SourceItem>>;
}
```

v1 ships four providers over **public, no-key APIs**, each returning **source-qualified items** ranked
by the channel's own engagement signal (significance-over-recency):

| Channel | Endpoint (no auth) | Signal |
| :--- | :--- | :--- |
| Hacker News | Algolia search `https://hn.algolia.com/api/v1/search?query=…` | points, comments |
| Reddit | `https://www.reddit.com/search.json?q=…&sort=top` (+ a `User-Agent`) | upvotes |
| GitHub | `https://api.github.com/search/repositories?q=…` (+ UA; 60 req/hr unauth) | stars, recency |
| Polymarket | public markets/search API | market odds |

A registry maps a stream's `sources: [..]` to active providers; fetches run **concurrently**. HTTP
via `reqwest`. Network failures of one channel degrade gracefully (skip it, note it) — never fail the
whole refresh. **Tested against recorded JSON fixtures** (no live network in CI). *Live network is the
unverified mile.* `last30days` + credentialed channels are future `SourceProvider`s (todo, §11).

### 4.4 Engine — the watch loop (the heart)

`engine::refresh(root, &StreamDescription, &dyn Agent, &[Box<dyn SourceProvider>]) -> Summary`:

1. Load prior document + state.
2. **Fetch** the stream's channels concurrently → merge into `Vec<SourceItem>`, rank by significance.
3. **Dedup**: `new = items whose source-qualified id ∉ seen_item_ids`.
4. **If `new` empty** → record `last_checked_at`; emit `done`("nothing new"); **no agent call** (restraint + cheap).
5. **Else** → extract the user's `## My notes`; `agent.synthesize({ topic, new, prior_doc })` →
   incremental reconcile (fold `new` into Current understanding; rewrite What-changed to this delta;
   update Open questions); **re-attach `## My notes` verbatim**; snapshot to history; atomic-write the
   document; update state (`seen += new ids`, `last_changed_at`, `doc_digest`); emit `done`(changed) + `stream_updated`.

The **reconcile prompt** (`reconcile.rs`) instructs: preserve & extend Current understanding (never
regenerate from scratch), cite each item, keep What-changed to genuinely-new significant items, stay
honest about Open questions, output only the three Freshet-owned sections. Wording is tuned against
real agent output (open question — unverifiable tonight).

### 4.5 Scheduler

`is_due(mode, interval_minutes, last_checked_at)` is a pure, tested function. A `tokio` task ticks
every minute while the app runs: find `active` streams that are due (`on_launch` shortly after start;
`interval` every N minutes), spawn **background** refreshes (never on the UI path), emit progress.
Manual "Refresh now" calls the same `engine::refresh`. (A desktop app only runs when open; the
always-on cloud runner is vision §11.)

### 4.6 TauriBridge

The real `bridge.rs` implements §7's commands over `store`/`engine`/`scheduler`/`agent`/`sources`,
mapping `anyhow::Error → String` at the boundary; `refresh_stream` and creation run on background
tasks and emit events. The frontend's `TauriBridge` (TS) wraps `invoke`/`listen`; the app auto-selects
`TauriBridge` in the Tauri runtime, `MockBridge` in the browser (so browser dev keeps working forever).

## 5. First-run onboarding

Backed by research on VS Code's Get-Started, Obsidian's vault picker, Tolaria, the BYO-LLM field
(Jan/AnythingLLM/Continue/LM Studio), Apple HIG, and 2025–26 onboarding studies.

**Principle:** Freshet is a *watch model, calm by default* — so its first-run should be the **quietest
in the field**. The signature moment is a document reconciling, so onboarding should **end by landing
in the empty, waiting document view, not a "you're all set!" celebration** (Apple HIG: onboarding ends
in the working app). Treat it as **everboarding** — a minimal gate now, contextual teaching later.
Evidence: ≤3-step flows complete ~72%, 7-step ~16%; ~70% skip linear tours; **cards beat carousels**
(VS Code tested and *rejected* a carousel). So: **2 required steps**, an invisible-when-it-works
auto-detect, everything else deferred to the moment of need.

**Step sequence:**

- **Pre-step — instant silent detect.** On first launch, probe PATH for `claude`/`codex` (§4.2),
  **deferred until after first paint** (Jan's fix; never block boot on network/subprocess). Hold the result.
- **Step 1 — Welcome + value (one line, one action).** A single quiet screen: one sentence framing the
  watch model ("Freshet turns topics into living documents that update themselves — quietly, in the
  background") and one primary button: **Choose where Freshet writes**. No carousel, no multi-slide tour.
- **Step 2 — Pick the root folder (the one structurally-required step).** Native directory picker
  (`tauri-plugin-dialog` → `NSOpenPanel`), pre-seeded with `~/Documents/Freshet` (creatable in-dialog).
  This is Obsidian's vault-pick model. **macOS hard constraint:** a sandboxed app can't access an
  arbitrary path programmatically — the panel *is* the authorization grant; persist it as a
  **security-scoped bookmark** and re-activate it on every launch (access otherwise expires on quit).
  Write `onboarded`/`has_root` to the store **immediately** so a crash mid-setup doesn't replay, and
  **don't lose the user's place if the picker is cancelled** (the VS Code #137544 momentum-killer).
- **Step 3 — Agent shown as a *resolved state*, not a question** (uses the pre-step result):
  - **Found** (the happy path, usually invisible) → a calm confirmed line: "Found Claude Code ✓ 2.1.x —
    Freshet will use it." One **Continue**. Nothing configured by hand. *(No competitor auto-detects a
    local agent — this is Freshet's best-in-class moment.)*
  - **None found** (decision 3) → a calm **blank-canvas** state (never a red error): "Freshet runs on
    your own local agent. Install **Claude Code** or **Codex**, then re-check," with install links, a
    **Re-check** button (Jan/AnythingLLM retry pattern), and a "Locate manually…" picker. Non-blocking —
    the app stays browsable; you just can't refresh until an agent exists.
  - **Distinguish states explicitly** (the Zed anti-pattern): *no agent configured* ≠ *agent configured
    but unreachable* ≠ *agent working* — never one message for all three.
- **Step 4 — First stream = the payoff.** Land directly in the main view with a focused empty state:
  one input — **"What do you want to watch?"** — and 2–3 ignorable example topic chips. Submitting opens
  the **creation chat** (§6); producing the first living document *is* the onboarding (value-first).

**Deferred (never gate first-run):** any API key (prompt inline at first use, not setup), notification
permission (ask only after a background refresh *finds something*, with priming — denied macOS perms
are hard to recover), refresh cadence, advanced settings, launch-at-login, theme/density.

**Calm empty-states (copy patterns):** *no agent* → blank-canvas + single CTA + re-check; *no streams*
→ "What do you want to watch?" + chips; *refreshed, nothing changed* → "Nothing new since yesterday."
in the periphery (a subtle timestamp), a valid calm state, never a dialog (honors quiet-by-default).

**Implementation (Tauri 2):** gate on a `tauri-plugin-store` key; show a **dedicated onboarding
window**, hide main until complete, then switch (don't model it as a modal). Use
`tauri-plugin-window-state` to restore geometry (first run is the only non-restored launch). Defer all
network/subprocess init until after first paint.

**State / bridge:** `config.onboarded`, `has_root`, the security-scoped bookmark, the selected agent.
Commands: `get_onboarding_state()`, `set_root_folder(path)` (opens the panel, stores the bookmark),
`list_agents()`, `recheck_agents()`, `set_default_agent(kind)`, `complete_onboarding()`. No accounts,
nothing leaves the machine.

## 6. Stream creation as a chat

The vision's "**planting, not querying**" (§4, §7.2): describe a curiosity, it narrows with you, then
shows a real first draft before you commit. This **upgrades the built form** — the form's fields
(topic · sources · cadence) become the *structured result* the chat fills in (and which stays editable),
not the entry point.

**Flow (agent-driven):**
1. User opens **New stream** → a chat seeded with a system prompt: *"Help the user scope a knowledge
   stream. Ask at most 1–2 narrowing questions (angle, depth, which free channels matter). Then
   propose a stream: a title, a one-line topic, a suggested subset of {reddit, hackernews, github,
   polymarket}, and a cadence. When the user is happy, emit the proposed description."*
2. The user types their interest; the agent replies (streamed via events), possibly asking a
   narrowing question, then **proposes a `StreamDescription`** (shown as an editable summary card —
   the old form, pre-filled).
3. On accept → **generate the first draft**: `agent.synthesize` over an initial fetch of the chosen
   channels → a real opening living document the user previews ("yes, that's the shape").
4. On commit → write the description + document + state; the stream appears on the desk.

**Backend (`creation.rs`) + bridge:**
- `start_creation()` → `{ session_id, opening_message }`.
- `creation_message(session_id, text)` → streams agent reply via a `creation_progress` event; may
  carry a `proposed_description`.
- `creation_preview(session_id, description)` → fetch + `synthesize` → `{ draft_markdown }`.
- `creation_commit(session_id, description)` → write everything → `StreamSummary`.

This reuses `Agent.chat` + the same `SourceProvider`/`synthesize` path as refresh, so creation and
refresh share the engine. ("generate a bump eventually" — interpreted as *generate the first draft /
initial document*; flagged in §11 if the owner meant something else, e.g. the "something moved" mark.)

## 7. Updated bridge contract (full)

Extends v1 §5.5. (TS mirror in `src/bridge/types.ts`; the `Bridge` interface gains these — `MockBridge`
implements them with canned behavior so the UI keeps working in the browser.)

```
config / onboarding:
  get_onboarding_state() -> { onboarded, has_root, agent: AgentStatus? }
  set_root_folder(path) ; get_config()
  list_agents() -> [AgentStatus] ; recheck_agents() -> [AgentStatus]
  set_default_agent(kind) ; complete_onboarding()
streams:
  list_streams() -> [StreamSummary]
  get_stream(id) -> { description, document_markdown, last_checked_at }
  refresh_stream(id) -> Summary{ changed, n_new }
  save_notes(id, markdown) ; set_stream_status(id, status)
creation (chat):
  start_creation() -> { session_id, opening_message }
  creation_message(session_id, text) -> { reply, proposed_description? }
  creation_preview(session_id, description) -> { draft_markdown }
  creation_commit(session_id, description) -> StreamSummary
events:
  refresh_progress { stream_id, phase, detail? }
  stream_updated { stream_id, changed }
  creation_progress { session_id, delta | message | proposed_description }
  agents_changed { agents }
```

## 8. Error handling

- **No agent** → refresh/creation surface a calm "connect a local agent" state (→ onboarding §5.3);
  nothing corrupted.
- **One channel fails** → skip it, note it in progress detail; refresh still proceeds with the rest.
- **Agent call fails / times out** → `error` phase; document + state untouched; quiet inline message.
- **Atomic writes + history snapshot** → no torn files; a bad synthesis is revertable.
- **Never block the UI** — all fetch/agent/refresh work on background tasks.

## 9. Testing

- **Pure/unit (CI, no network/agent):** `is_due`; dedup (source-qualified); "nothing new" path;
  My-notes splice byte-identity; atomic writes; history snapshot/index; serde round-trips; each
  `SourceProvider` against **recorded JSON fixtures**; discovery tiers via an injectable command runner.
- **Engine integration:** `FakeAgent` + `FakeSourceProvider` → refresh twice → run 1 populates, identical
  run 2 = "nothing new" (no agent call), run 2 + one extra item → it appears under What-changed; `## My
  notes` survives all of it.
- **Creation:** `FakeAgent` drives a scripted chat → proposed description → preview → commit writes files.
- **Frontend:** existing 50 tests keep passing against `MockBridge`; new onboarding + chat-creation
  surfaces tested against the mock.
- **Unverified (needs the owner's machine):** live `claude`/`codex` invocation; live channel HTTP; the
  reconcile-prompt quality. Clearly marked in code (`// UNVERIFIED: live path`) and in the build summary.

## 10. Build approach (tonight)

Subagent-driven, TDD, same discipline as the frontend: contracts/types → Store → SourceProviders
(fixtures) → Agent discovery + adapters (fake-runner tests; real invocation behind it) → Engine
(FakeAgent/FakeSource) → Scheduler → TauriBridge → frontend wiring (TauriBridge + auto-select) →
onboarding surface → creation chat. Commit in logical steps. Leave a `NIGHT-SUMMARY` note (where things
stand, what's verified vs not, decisions made) at the end.

## 11. Open questions / decisions for the owner

- **"generate a bump eventually"** — I read this as *generate the first-draft document* during creation
  (§6). If you meant the "something moved" notification/mark, or an actual OS notification, say so.
- **Chat-creation depth** — v1 keeps it to ≤2 narrowing questions + a proposed description + draft. More
  conversational range is a later enhancement.
- **`last30days` / richer & credentialed channels** — deferred as future `SourceProvider`s (todo #19).
- **Reconcile + creation prompt wording** — unverifiable tonight; tuned with you against real output.
- **Agent streaming granularity** — how much of the agent's token stream to surface in the UI during
  refresh/creation (full stream vs. phase ticks). Defaulting to phase ticks + a peek; refine later.
- **History/revert UI** — backend snapshots ship; the viewing/revert surface is a later sub-project.
```
