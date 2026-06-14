# Conversational Stream Creation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the blocking "fill a form → Preview (≈1 min synth)" stream-creation flow with a calm multi-turn chat that converges on a *per-source* search template, then navigates straight to the new stream and animates the document reconciling on real backend progress events.

**Architecture:** Three seams change. (1) **Data model** — a stream's `sources` becomes a list of `{channel, query}` so each source is searched with its own precise query (fixes "single vague term → junk"). (2) **Agent chat** — a new `chat_designer` bridge command drives `Agent::chat` with a stream-designer system prompt; the agent proposes a full `StreamDescription` as fenced JSON. (3) **Create = persist + reconcile** — `create_stream` only persists the description (fast); the first synthesis runs as a normal `refresh_stream`, so the existing `refresh_progress` events drive a live "reconciling" animation in the reading view.

**Tech Stack:** Rust (Tauri 2 backend, `serde`, `anyhow`), React 19 + TypeScript + Vite, vitest, `@testing-library/react`. No new dependencies.

---

## Background the engineer needs

- **Three architectural seams, all faked in tests:** `Bridge` (TS: `MockBridge` in browser/tests vs `TauriBridge` native), the `Agent` trait (Rust: `FakeAgent`), and `SourceProvider` (Rust: `FakeSourceProvider`).
- **Wire casing:** Rust structs use `#[serde(rename_all = "camelCase")]`; Rust enums use `snake_case` (e.g. `CadenceMode::OnLaunch` → `"on_launch"`). The TS types must match exactly.
- **Run tests:**
  - Rust: `cargo test --manifest-path src-tauri/Cargo.toml --lib` (run `source "$HOME/.cargo/env"` first if `cargo` isn't on PATH).
  - TS: `npx vitest run` (or a single file: `npx vitest run src/path/to/file.test.tsx`).
  - Build check: `npm run build` (tsc + vite) and `cargo build --manifest-path src-tauri/Cargo.toml`.
- **Current shape being replaced:** `StreamDescription.sources: Vec<String>`; `commands::generate_first_draft` + `bridge::generate_first_draft` (the Preview path); `commands::create_stream` does fetch + synth; the React `Create` view is a form (`src/views/Create/Create.tsx`) with topic/sources/cadence inputs and a Preview button.
- **CodeMirror / react-markdown already present** (used by the reading view). No new deps.
- **Reddit is excluded from defaults** (needs OAuth); available channels for proposals: `hackernews`, `github`, `polymarket`.

## File structure (what changes and why)

**Backend (`src-tauri/src/`)**
- `model.rs` — add `SourceQuery { channel, query }`; change `StreamDescription.sources` to `Vec<SourceQuery>`.
- `sources/mod.rs` — add `fetch_each(providers, sources, limit)` that searches each provider with its own query; keep `fetch_all` for any topic-wide use but stop using it in the engine.
- `agent/mod.rs` — add `STREAM_DESIGNER_SYSTEM` prompt + `parse_proposed_stream(reply, now) -> Option<StreamDescription>`.
- `agent/fake.rs` — make `FakeAgent::reflecting().chat()` return a deterministic brainstorm (so `FRESHET_FAKE_AGENT=1` exercises the flow).
- `commands.rs` — `create_stream` becomes persist-only; delete `generate_first_draft`, `DraftInput`, `DraftResult`; channels-fetch uses `fetch_each`.
- `engine/mod.rs` — `refresh` fetches via `fetch_each(providers, &desc.sources, …)`.
- `bridge.rs` — add `chat_designer`; change `create_stream` to persist-only (no async/events needed — the first synth is a separate `refresh_stream`); delete `generate_first_draft`. `resolve_providers` takes the channels from `desc.sources`.
- `lib.rs` — register `chat_designer`; drop `generate_first_draft` from the handler list.

**Frontend (`src/`)**
- `bridge/types.ts` — add `SourceQuery`, `ChatMessage`, `ChatReply`; change `StreamDescription.sources` to `SourceQuery[]`; remove `DraftInput`/`DraftResult`.
- `bridge/Bridge.ts` — add `chatDesigner`; remove `generateFirstDraft`; `createStream` unchanged in signature.
- `bridge/TauriBridge.ts` / `bridge/MockBridge.ts` — implement `chatDesigner`; make `createStream` persist-only in the mock; remove `generateFirstDraft`.
- `bridge/sampleData.ts` — update sample `StreamDescription.sources` to the new shape.
- `views/Create/Create.tsx` + `Create.css` — rewrite as a chat (messages, input, proposal card, "creating" hand-off).
- `views/Reading/Reading.tsx` + a new `views/Reading/Reconciling.tsx` + `Reconciling.css` — listen to `refresh_progress`, auto-trigger the first synth when the doc is empty, and animate the reconcile.

---

## Phase 1 — Per-source template (data model + sourcing)

### Task 1: Add `SourceQuery` and make `StreamDescription.sources` per-source

**Files:**
- Modify: `src-tauri/src/model.rs`
- Test: `src-tauri/src/model.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `model.rs`:

```rust
    #[test]
    fn source_query_round_trips_camel_case() {
        let q = SourceQuery { channel: "github".into(), query: "tokio-rs/tokio releases".into() };
        let json = serde_json::to_string(&q).expect("serialize");
        assert!(json.contains("\"channel\""), "got: {json}");
        assert!(json.contains("\"query\""), "got: {json}");
        let back: SourceQuery = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(q, back);
    }

    #[test]
    fn stream_description_has_source_queries() {
        let desc = StreamDescription {
            id: "s1".into(),
            title: "Tokio".into(),
            topic: "tokio async runtime".into(),
            sources: vec![
                SourceQuery { channel: "github".into(), query: "tokio-rs/tokio".into() },
                SourceQuery { channel: "hackernews".into(), query: "tokio runtime".into() },
            ],
            cadence: Cadence { mode: CadenceMode::OnLaunch, interval_minutes: None },
            status: StreamStatus::Active,
            created_at: "2026-06-14T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&desc).expect("serialize");
        let back: StreamDescription = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(desc, back);
        assert_eq!(back.sources[0].channel, "github");
        assert_eq!(back.sources[0].query, "tokio-rs/tokio");
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `source "$HOME/.cargo/env"; cargo test --manifest-path src-tauri/Cargo.toml --lib source_query`
Expected: FAIL to compile — `SourceQuery` not found, and the existing `sources: vec!["hackernews".into()]` literals no longer match the new field type.

- [ ] **Step 3: Add the type and change the field**

In `model.rs`, add after the `Cadence` struct:

```rust
/// One source channel plus the precise query to search it with. A stream's
/// "template" is the set of these — each source is searched on its own terms,
/// so a vague topic doesn't poison every channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceQuery {
    pub channel: String,
    pub query: String,
}
```

Change `StreamDescription`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamDescription {
    pub id: String,
    pub title: String,
    /// Human-facing description of the stream (used for the title + as reconcile context).
    pub topic: String,
    /// Per-source search template.
    pub sources: Vec<SourceQuery>,
    pub cadence: Cadence,
    pub status: StreamStatus,
    pub created_at: String,
}
```

- [ ] **Step 4: Fix the existing model test literals**

In `model.rs` tests, update both `stream_description_interval_round_trips_and_camel_case` and any other `StreamDescription { … sources: vec!["hackernews".into()] … }` to:

```rust
            sources: vec![SourceQuery { channel: "hackernews".into(), query: "rust async".into() }],
```

- [ ] **Step 5: Run the model tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib model::`
Expected: the model module compiles; new tests PASS. (Other modules won't compile yet — that's expected; later tasks fix them. Use `--lib model::` to scope.)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/model.rs
git commit -m "feat(model): per-source SourceQuery template on StreamDescription"
```

### Task 2: Search each source with its own query (`fetch_each`)

**Files:**
- Modify: `src-tauri/src/sources/mod.rs`
- Modify: `src-tauri/src/engine/mod.rs`
- Test: `src-tauri/src/sources/mod.rs` (inline)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `sources/mod.rs`:

```rust
    #[test]
    fn fetch_each_searches_per_source_and_merges() {
        use crate::model::SourceQuery;
        let providers: Vec<Box<dyn SourceProvider>> = vec![
            Box::new(FakeSourceProvider::new("github", vec![make_item("gh:1", "github")])),
            Box::new(FakeSourceProvider::new("hackernews", vec![make_item("hn:1", "hackernews")])),
        ];
        let sources = vec![
            SourceQuery { channel: "github".into(), query: "tokio-rs/tokio".into() },
            SourceQuery { channel: "hackernews".into(), query: "tokio runtime".into() },
        ];
        let items = fetch_each(&providers, &sources, 10);
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.source == "github"));
        assert!(items.iter().any(|i| i.source == "hackernews"));
    }

    #[test]
    fn fetch_each_skips_provider_with_no_matching_query() {
        use crate::model::SourceQuery;
        let providers: Vec<Box<dyn SourceProvider>> = vec![
            Box::new(FakeSourceProvider::new("github", vec![make_item("gh:1", "github")])),
        ];
        // No SourceQuery for "github" → that provider is skipped.
        let sources = vec![SourceQuery { channel: "reddit".into(), query: "x".into() }];
        let items = fetch_each(&providers, &sources, 10);
        assert!(items.is_empty(), "provider with no query must be skipped");
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib fetch_each`
Expected: FAIL — `fetch_each` not found.

- [ ] **Step 3: Implement `fetch_each`**

Add to `sources/mod.rs` (after `fetch_all`):

```rust
use crate::model::SourceQuery;

/// Run each provider with the query for its channel from `sources`. A provider
/// with no matching `SourceQuery` is skipped. Errors are logged and skipped so
/// one bad channel doesn't sink the pass.
pub fn fetch_each(
    providers: &[Box<dyn SourceProvider>],
    sources: &[SourceQuery],
    limit: usize,
) -> Vec<SourceItem> {
    let mut items = Vec::new();
    for provider in providers {
        let Some(sq) = sources.iter().find(|s| s.channel == provider.channel()) else {
            continue;
        };
        match provider.fetch(&sq.query, limit) {
            Ok(mut fetched) => {
                log::info!(
                    "sources: channel='{}' query={:?} fetched={} items",
                    provider.channel(), sq.query, fetched.len(),
                );
                items.append(&mut fetched);
            }
            Err(e) => log::warn!(
                "sources: provider '{}' failed (skipping): {e:#}", provider.channel()
            ),
        }
    }
    items
}
```

(The existing `use crate::model::SourceItem;` stays; add the `SourceQuery` import alongside it instead of a second `use` if the engineer prefers — either compiles.)

- [ ] **Step 4: Point the engine at `fetch_each`**

In `engine/mod.rs`, change the import and the fetch call. Replace:

```rust
use crate::sources::{fetch_all, SourceProvider};
```
with
```rust
use crate::sources::{fetch_each, SourceProvider};
```

Replace the fetch line in `refresh` (currently `let mut items = fetch_all(providers, &desc.topic, FETCH_LIMIT);`) with:

```rust
    let mut items = fetch_each(providers, &desc.sources, FETCH_LIMIT);
```

- [ ] **Step 5: Fix engine test descriptions**

In `engine/mod.rs` tests, the `desc()` helper builds `sources: vec!["hackernews".into()]`. Change to:

```rust
            sources: vec![crate::model::SourceQuery {
                channel: "hackernews".into(),
                query: "autonomous AI agents".into(),
            }],
```

(The `FakeSourceProvider` is registered with channel `"hackernews"`, so `fetch_each` will match it.)

- [ ] **Step 6: Run sources + engine tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib sources:: ; cargo test --manifest-path src-tauri/Cargo.toml --lib engine::`
Expected: `fetch_each` tests PASS; engine lifecycle tests PASS (items still flow because the fake provider's channel matches a `SourceQuery`).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/sources/mod.rs src-tauri/src/engine/mod.rs
git commit -m "feat(sources): fetch_each searches every source with its own query"
```

---

## Phase 2 — Conversational creation (backend)

### Task 3: Stream-designer prompt + proposal parsing

**Files:**
- Modify: `src-tauri/src/agent/mod.rs`
- Test: `src-tauri/src/agent/mod.rs` (inline)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `agent/mod.rs`:

```rust
    #[test]
    fn parse_proposed_stream_from_fenced_json() {
        let reply = "Sounds good — here's a focused stream.\n\n```json\n{\n\
          \"title\": \"Tokio Releases\",\n\
          \"topic\": \"tokio async runtime releases\",\n\
          \"sources\": [{\"channel\": \"github\", \"query\": \"tokio-rs/tokio\"},\n\
                        {\"channel\": \"hackernews\", \"query\": \"tokio runtime\"}],\n\
          \"cadence\": {\"mode\": \"on_launch\"}\n}\n```\n\nStart it or refine.";
        let desc = parse_proposed_stream(reply, "2026-06-14T10:00:00Z").expect("should parse");
        assert_eq!(desc.title, "Tokio Releases");
        assert_eq!(desc.id, "tokio-releases"); // slugified from title
        assert_eq!(desc.sources.len(), 2);
        assert_eq!(desc.sources[0].channel, "github");
        assert_eq!(desc.status, crate::model::StreamStatus::Active);
        assert_eq!(desc.created_at, "2026-06-14T10:00:00Z");
        assert_eq!(desc.cadence.mode, crate::model::CadenceMode::OnLaunch);
    }

    #[test]
    fn parse_proposed_stream_none_without_block() {
        assert!(parse_proposed_stream("just a question, no json yet", "2026-06-14T10:00:00Z").is_none());
    }

    #[test]
    fn parse_proposed_stream_none_on_malformed() {
        assert!(parse_proposed_stream("```json\n{ not valid }\n```", "2026-06-14T10:00:00Z").is_none());
    }

    #[test]
    fn stream_designer_system_mentions_per_source_json() {
        assert!(STREAM_DESIGNER_SYSTEM.contains("channel"));
        assert!(STREAM_DESIGNER_SYSTEM.contains("query"));
        assert!(STREAM_DESIGNER_SYSTEM.contains("json"));
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib parse_proposed_stream`
Expected: FAIL — `parse_proposed_stream` and `STREAM_DESIGNER_SYSTEM` not found.

- [ ] **Step 3: Add the prompt + parser**

In `agent/mod.rs`, add near `render_chat_prompt` (the imports already include `StreamDescription`; also use `crate::commands::slugify` and the model types):

```rust
use crate::model::{Cadence, CadenceMode, SourceQuery, StreamDescription, StreamStatus};

/// System prompt for the stream-creation brainstorm. The agent narrows a vague
/// topic into ONE focused, well-sourceable stream with a per-source query
/// template, then proposes it as a fenced ```json block.
pub const STREAM_DESIGNER_SYSTEM: &str = "\
You are Freshet's stream designer. Freshet turns a topic into a self-updating \
knowledge document it keeps watching over time. Help the user shape ONE focused, \
trackable stream through a short, calm conversation.\n\n\
Guidelines:\n\
- Keep replies brief (2-4 sentences). Ask at most one clarifying question per turn.\n\
- A single vague word sources badly. For EACH source, write a precise query (specific \
projects, repos, or terms) so the watch finds real signal.\n\
- Available sources: hackernews, github, polymarket. Choose only the ones that fit.\n\
- When the scope is clear, propose the stream by ENDING your message with a fenced \
```json block with exactly these keys:\n\
  {\"title\": \"<short title>\", \"topic\": \"<one-line description>\", \"sources\": \
[{\"channel\": \"github\", \"query\": \"<query>\"}], \"cadence\": {\"mode\": \"on_launch\"}}\n\
- Don't propose until each chosen source has a precise query. After proposing, say they \
can start it or keep refining.";

/// Shape the agent emits inside the fenced json block (no id/status/createdAt —
/// Freshet fills those in).
#[derive(serde::Deserialize)]
struct ProposedStream {
    title: String,
    topic: String,
    sources: Vec<SourceQuery>,
    cadence: Cadence,
}

/// Parse a proposed `StreamDescription` from the first fenced ```json block in
/// `reply`. `now` becomes `created_at`; `id` is slugified from the title;
/// `status` defaults to Active. Returns None if absent or malformed.
pub fn parse_proposed_stream(reply: &str, now: &str) -> Option<StreamDescription> {
    let start = reply.find(\"```json\")?;
    let after = &reply[start + \"```json\".len()..];
    let end = after.find(\"```\")?;
    let json = after[..end].trim();
    let p: ProposedStream = serde_json::from_str(json).ok()?;
    if p.title.trim().is_empty() || p.sources.is_empty() {
        return None;
    }
    Some(StreamDescription {
        id: crate::commands::slugify(&p.title),
        title: p.title,
        topic: p.topic,
        sources: p.sources,
        cadence: p.cadence,
        status: StreamStatus::Active,
        created_at: now.to_string(),
    })
}
```

Note: `CadenceMode` is imported for the test's reference; if Rust warns it's unused in non-test code, scope the import to what's used (`Cadence`, `SourceQuery`, `StreamDescription`, `StreamStatus`). Keep `CadenceMode` only if referenced.

- [ ] **Step 4: Run the parser tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib parse_proposed_stream ; cargo test --manifest-path src-tauri/Cargo.toml --lib stream_designer_system`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/agent/mod.rs
git commit -m "feat(agent): stream-designer prompt + JSON proposal parsing"
```

### Task 4: `chat_designer` bridge command + usable fake chat

**Files:**
- Modify: `src-tauri/src/agent/fake.rs`
- Modify: `src-tauri/src/bridge.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/agent/fake.rs` (inline)

- [ ] **Step 1: Write the failing test (fake chat is useful)**

Add to the `tests` module in `agent/fake.rs`:

```rust
    #[test]
    fn reflecting_chat_proposes_after_a_turn() {
        let agent = FakeAgent::reflecting(AgentKind::ClaudeCode);
        let history = vec![
            ChatTurn { role: "user".into(), content: "tokio releases".into() },
            ChatTurn { role: "assistant".into(), content: "which angle?".into() },
            ChatTurn { role: "user".into(), content: "just releases".into() },
        ];
        let reply = agent.chat("system", &history).expect("chat");
        assert!(reply.text.contains("```json"), "should propose json: {}", reply.text);
        assert!(reply.text.contains("\"channel\""), "proposal needs per-source queries");
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib reflecting_chat_proposes`
Expected: FAIL — `reflecting` currently returns the empty `canned_reply`.

- [ ] **Step 3: Make `FakeAgent::chat` deterministic in reflect mode**

In `agent/fake.rs`, replace the `chat` impl:

```rust
    fn chat(&self, _system: &str, history: &[ChatTurn]) -> anyhow::Result<ChatReply> {
        self.chat_calls.fetch_add(1, Ordering::SeqCst);
        if !self.reflect_items {
            return Ok(self.canned_reply.clone());
        }
        // Deterministic brainstorm: first turn asks, later turns propose.
        let user_turns = history.iter().filter(|t| t.role == "user").count();
        let topic = history
            .iter()
            .filter(|t| t.role == "user")
            .map(|t| t.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let text = if user_turns <= 1 {
            format!(
                "Got it — \"{}\". Which angle matters most: releases, discussion, or market signal?",
                topic.trim()
            )
        } else {
            let title = topic.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
            format!(
                "Here's a focused stream. Start it or refine.\n\n```json\n{{\"title\": \"{}\", \
                 \"topic\": \"{}\", \"sources\": [{{\"channel\": \"github\", \"query\": \"{}\"}}], \
                 \"cadence\": {{\"mode\": \"on_launch\"}}}}\n```",
                title, topic.trim(), topic.trim()
            )
        };
        Ok(ChatReply { text, proposed_description: None })
    }
```

- [ ] **Step 4: Run the fake test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib reflecting_chat_proposes`
Expected: PASS. Also run the existing `returns_canned_chat_reply_with_proposed` test — still PASS (non-reflect fakes still return their canned reply).

- [ ] **Step 5: Add the `chat_designer` command**

In `bridge.rs`, add imports and the command. Add to the `use crate::agent::…` line: `ChatTurn`, `STREAM_DESIGNER_SYSTEM`, `parse_proposed_stream`:

```rust
use crate::agent::{detect_agents, parse_proposed_stream, select_agent, Agent, ChatTurn, STREAM_DESIGNER_SYSTEM};
```

Add near the other stream commands:

```rust
/// One chat message over the wire.
#[derive(serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// The reply the frontend renders: the assistant text plus, when the agent has
/// proposed a stream, the parsed `StreamDescription`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatDesignerReply {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed: Option<StreamDescription>,
}

/// Drive one turn of the stream-design brainstorm.
#[tauri::command]
pub fn chat_designer(
    state: State<'_, BackendState>,
    history: Vec<ChatMessage>,
) -> Result<ChatDesignerReply, FreshetError> {
    let agent = resolve_agent(&state)?;
    let turns: Vec<ChatTurn> = history
        .into_iter()
        .map(|m| ChatTurn { role: m.role, content: m.content })
        .collect();
    let reply = agent
        .chat(STREAM_DESIGNER_SYSTEM, &turns)
        .map_err(|e| commands::classify_agent_error(&e))?;
    let proposed = parse_proposed_stream(&reply.text, &now_iso());
    Ok(ChatDesignerReply { text: reply.text, proposed })
}
```

- [ ] **Step 6: Register the command**

In `lib.rs`, add `bridge::chat_designer,` to the `tauri::generate_handler![…]` list.

- [ ] **Step 7: Build the backend**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles (other commands referencing the old `create_stream`/`generate_first_draft` still compile for now; they change in Task 5).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/agent/fake.rs src-tauri/src/bridge.rs src-tauri/src/lib.rs
git commit -m "feat(bridge): chat_designer command + deterministic fake brainstorm"
```

### Task 5: `create_stream` becomes persist-only; remove the Preview path

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/bridge.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands.rs` (inline)

- [ ] **Step 1: Rewrite the failing create test**

In `commands.rs` tests, replace `create_then_list_and_get` with a persist-only contract:

```rust
    #[test]
    fn create_persists_description_without_synth() {
        let dir = tmp();
        let root = dir.path();
        let desc = description_from_proposed("Tokio", "tokio runtime", "2026-06-14T10:00:00Z");
        let summary = create_stream(root, &desc).expect("create");
        assert_eq!(summary.id, desc.id);
        // No document yet — the first synth runs as a normal refresh.
        assert!(get_stream(root, &desc.id).expect("get").document_markdown.is_empty());
        // It is listed.
        assert_eq!(list_streams(root).len(), 1);
    }
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib create_persists`
Expected: FAIL — `create_stream` still takes an agent/providers; `description_from_proposed` doesn't exist.

- [ ] **Step 3: Simplify `create_stream` + add a description helper**

In `commands.rs`, replace the whole `create_stream` fn with a persist-only version, and add a small constructor used by tests + (later) the bridge isn't needed since the agent proposes the full desc — but keep a helper for tests:

```rust
/// Persist a new stream's description. The first synthesis runs separately as a
/// normal refresh (so it streams progress); creation itself is instant and never
/// blocks on the agent.
pub fn create_stream(root: &Path, desc: &StreamDescription) -> anyhow::Result<StreamSummary> {
    log::info!("commands::create_stream: id={:?} (persist-only)", desc.id);
    store::save_description(root, desc)?;
    Ok(StreamSummary {
        id: desc.id.clone(),
        title: desc.title.clone(),
        last_checked_at: None,
        changed_since_seen: true, // fresh stream reads as changed until first read
    })
}

/// Test/helper constructor for a minimal one-source description.
#[cfg(test)]
pub fn description_from_proposed(title: &str, topic: &str, now: &str) -> StreamDescription {
    StreamDescription {
        id: slugify(title),
        title: title.to_string(),
        topic: topic.to_string(),
        sources: vec![crate::model::SourceQuery {
            channel: "hackernews".into(),
            query: topic.to_string(),
        }],
        cadence: Cadence { mode: CadenceMode::OnLaunch, interval_minutes: None },
        status: StreamStatus::Active,
        created_at: now.to_string(),
    }
}
```

Delete `generate_first_draft`, `DraftInput`, `DraftResult`, `description_from_input`, and their tests (`first_draft_*`, `typed_error_no_sources_on_zero_items`, `typed_error_create_stream_no_sources`, `create_then_list_and_get`, `save_notes_updates_only_my_notes` keeps but its `create_stream` call must change — see Step 4). Remove the now-unused `ResearchInput` import if nothing else uses it in this file.

- [ ] **Step 4: Fix other tests that called the old `create_stream`**

`save_notes_updates_only_my_notes` and any test that did `create_stream(root, &desc, &agent, &providers, now)` must now (a) `create_stream(root, &desc)` then (b) run one `refresh_stream(root, &desc.id, &agent, &providers, now)` to produce the document. Rewrite its setup:

```rust
        let desc = description_from_proposed("Autonomous AI Agents", "ai agents", "2026-06-14T10:00:00Z");
        create_stream(root, &desc).expect("create");
        refresh_stream(root, &desc.id, &agent, &providers, "2026-06-14T10:00:00Z").expect("first synth");
```

Ensure the `providers` use channel `"hackernews"` so `fetch_each` matches `desc.sources`.

- [ ] **Step 5: Update the bridge command**

In `bridge.rs`, replace the `create_stream` command with the persist-only form, and delete `generate_first_draft`:

```rust
#[tauri::command]
pub fn create_stream(
    state: State<'_, BackendState>,
    description: StreamDescription,
) -> Result<StreamSummary, FreshetError> {
    let root = state.root().map_err(|e| FreshetError::agent_failed(&format!("{e:#}")))?;
    commands::create_stream(&root, &description)
        .map_err(|e| FreshetError::agent_failed(&format!("{e:#}")))
}
```

Update `resolve_providers` callers: `refresh` already uses `desc.sources`' channels — change `resolve_providers` to take channel names extracted from `Vec<SourceQuery>`. In `run_refresh_emitting`, replace `resolve_providers(state, &desc.sources)` with:

```rust
        let channels: Vec<String> = desc.sources.iter().map(|s| s.channel.clone()).collect();
        let providers = resolve_providers(state, &channels);
```

Remove `bridge::generate_first_draft` and its imports (`DraftInput`, `DraftResult`). Remove `generate_first_draft` from `lib.rs`'s handler list. (`create_stream` no longer needs `resolve_agent`/providers.)

- [ ] **Step 6: Run the backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`
Expected: all PASS. Fix any remaining references to deleted symbols the compiler flags.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src
git commit -m "feat(create): persist-only create_stream; first synth runs as refresh; drop Preview path"
```

---

## Phase 3 — Conversational creation (frontend)

### Task 6: Frontend types + bridge methods

**Files:**
- Modify: `src/bridge/types.ts`
- Modify: `src/bridge/Bridge.ts`
- Modify: `src/bridge/TauriBridge.ts`
- Modify: `src/bridge/MockBridge.ts`
- Modify: `src/bridge/sampleData.ts`
- Test: `src/bridge/MockBridge.test.ts` (create if absent)

- [ ] **Step 1: Write the failing test**

Create/extend `src/bridge/MockBridge.test.ts`:

```ts
import { MockBridge } from "./MockBridge";

test("chatDesigner proposes a per-source stream after a couple turns", async () => {
  const b = new MockBridge();
  const r1 = await b.chatDesigner([{ role: "user", content: "tokio" }]);
  expect(r1.text).toBeTruthy();
  const r2 = await b.chatDesigner([
    { role: "user", content: "tokio" },
    { role: "assistant", content: r1.text },
    { role: "user", content: "just releases" },
  ]);
  expect(r2.proposed).toBeTruthy();
  expect(r2.proposed!.sources[0].channel).toBeTruthy();
  expect(r2.proposed!.sources[0].query).toBeTruthy();
});

test("createStream persists without a document", async () => {
  const b = new MockBridge();
  const r2 = await b.chatDesigner([
    { role: "user", content: "tokio" },
    { role: "user", content: "just releases" },
  ]);
  const desc = r2.proposed!;
  const summary = await b.createStream(desc);
  expect(summary.id).toBe(desc.id);
  const got = await b.getStream(desc.id);
  expect(got.documentMarkdown).toBe(""); // synth happens on refresh
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `npx vitest run src/bridge/MockBridge.test.ts`
Expected: FAIL — `chatDesigner` not defined.

- [ ] **Step 3: Update `types.ts`**

In `src/bridge/types.ts`:

```ts
export interface SourceQuery { channel: string; query: string }
export interface StreamDescription {
  id: string;
  title: string;
  topic: string;
  sources: SourceQuery[];
  cadence: Cadence;
  status: StreamStatus;
  createdAt: string;
}
export interface ChatMessage { role: "user" | "assistant"; content: string }
export interface ChatReply { text: string; proposed?: StreamDescription }
```

Remove `DraftInput` and `DraftResult`. Keep `FREE_SOURCES`/`DEFAULT_SOURCES` (still referenced for labels).

- [ ] **Step 4: Update `Bridge.ts`**

Remove `generateFirstDraft`; add:

```ts
  chatDesigner(history: ChatMessage[]): Promise<ChatReply>;
```
(and import `ChatMessage`, `ChatReply`; remove `DraftInput`/`DraftResult` imports.)

- [ ] **Step 5: Update `TauriBridge.ts`**

Remove `generateFirstDraft`. Add:

```ts
  chatDesigner(history: ChatMessage[]): Promise<ChatReply> {
    return invoke<ChatReply>("chat_designer", { history });
  }
```

`createStream` unchanged (`invoke("create_stream", { description: desc })`).

- [ ] **Step 6: Update `MockBridge.ts`**

Remove `generateFirstDraft`. Make `createStream` persist-only (don't seed a document) and add `chatDesigner`:

```ts
  async createStream(desc: StreamDescription): Promise<StreamSummary> {
    this._throwIfAgentError();
    const summary: StreamSummary = { id: desc.id, title: desc.title, changedSinceSeen: true };
    this.state.summaries.push(summary);
    this.state.descriptions[desc.id] = { ...desc };
    this.state.documents[desc.id] = ""; // synth runs on first refresh
    this.persist();
    return { ...summary };
  }

  async chatDesigner(history: ChatMessage[]): Promise<ChatReply> {
    this._throwIfAgentError();
    const userTurns = history.filter((m) => m.role === "user");
    const topic = userTurns.map((m) => m.content).join(" ").trim();
    if (userTurns.length <= 1) {
      return { text: `Got it — "${topic}". Which angle matters most: releases, discussion, or market signal?` };
    }
    const title = topic.split(/\s+/).slice(0, 3).map((w) => w[0]?.toUpperCase() + w.slice(1)).join(" ") || "New Stream";
    const proposed: StreamDescription = {
      id: title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, ""),
      title,
      topic,
      sources: [
        { channel: "github", query: topic },
        { channel: "hackernews", query: topic },
      ],
      cadence: { mode: "on_launch" },
      status: "active",
      createdAt: new Date().toISOString(),
    };
    return { text: `Here's a focused stream. Start it or keep refining.`, proposed };
  }
```

(`refreshStream` in MockBridge should also populate the document on first call so the reconcile demo works in the browser — set `this.state.documents[id] = sampleDocFor(id)` inside `refreshStream` when the current doc is empty, before emitting phases.)

Add `ChatMessage`, `ChatReply` to the type imports; remove `DraftInput`/`DraftResult`.

- [ ] **Step 7: Update `sampleData.ts`**

Change every `sampleDescriptions[*].sources` from `["hackernews", …]` to the new shape, e.g.:

```ts
    sources: [
      { channel: "hackernews", query: "AI agents" },
      { channel: "github", query: "agent framework" },
    ],
```

- [ ] **Step 8: Run the bridge tests + typecheck**

Run: `npx vitest run src/bridge/MockBridge.test.ts && npm run build`
Expected: PASS; build clean (any component still importing `generateFirstDraft`/`DraftResult` will fail the build — those are fixed in Task 7).

- [ ] **Step 9: Commit**

```bash
git add src/bridge
git commit -m "feat(bridge): chatDesigner + per-source SourceQuery types; persist-only createStream"
```

### Task 7: Rewrite `Create` as a chat

**Files:**
- Rewrite: `src/views/Create/Create.tsx`
- Rewrite: `src/views/Create/Create.css`
- Test: `src/views/Create/Create.test.tsx`

- [ ] **Step 1: Write the failing test**

Replace `Create.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { BridgeProvider } from "../../bridge/BridgeProvider";
import { MockBridge } from "../../bridge/MockBridge";
import { Create } from "./Create";

function renderCreate(onCreated = vi.fn(), onCancel = vi.fn()) {
  return render(
    <BridgeProvider bridge={new MockBridge()}>
      <Create onCreated={onCreated} onCancel={onCancel} />
    </BridgeProvider>,
  );
}

test("sends a message and shows the assistant reply", async () => {
  renderCreate();
  await userEvent.type(screen.getByRole("textbox", { name: /message/i }), "tokio");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  expect(await screen.findByText(/which angle/i)).toBeInTheDocument();
});

test("a proposal shows a Start control that creates and hands off", async () => {
  const onCreated = vi.fn();
  renderCreate(onCreated);
  const input = screen.getByRole("textbox", { name: /message/i });
  await userEvent.type(input, "tokio");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  await userEvent.type(input, "just releases");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  const start = await screen.findByRole("button", { name: /start stream/i });
  await userEvent.click(start);
  await vi.waitFor(() => expect(onCreated).toHaveBeenCalled());
  expect(onCreated.mock.calls[0][0].id).toBeTruthy();
});

test("Cancel returns", async () => {
  const onCancel = vi.fn();
  renderCreate(vi.fn(), onCancel);
  await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
  expect(onCancel).toHaveBeenCalled();
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `npx vitest run src/views/Create/Create.test.tsx`
Expected: FAIL — old form is still rendered (no message textbox / Send).

- [ ] **Step 3: Implement the chat `Create`**

Replace `Create.tsx`:

```tsx
import { useState } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import { asFreshetError } from "../../bridge/types";
import type { ChatMessage, StreamDescription, StreamSummary, FreshetError } from "../../bridge/types";
import { AgentNotice } from "../../components/AgentNotice";
import "./Create.css";

const INTRO: ChatMessage = {
  role: "assistant",
  content: "What would you like to keep an eye on? Tell me the topic and I'll help shape a focused stream.",
};

export function Create({
  onCreated,
  onCancel,
}: {
  onCreated: (s: StreamSummary) => void;
  onCancel: () => void;
}) {
  const bridge = useBridge();
  const [messages, setMessages] = useState<ChatMessage[]>([INTRO]);
  const [proposal, setProposal] = useState<StreamDescription | null>(null);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<FreshetError | null>(null);

  async function send() {
    const text = input.trim();
    if (!text || sending) return;
    const next = [...messages, { role: "user", content: text } as ChatMessage];
    setMessages(next);
    setInput("");
    setSending(true);
    setError(null);
    try {
      // Only the dialogue (drop the client-side intro) goes to the agent.
      const history = next.filter((m, i) => !(i === 0 && m === INTRO));
      const reply = await bridge.chatDesigner(history);
      setMessages((m) => [...m, { role: "assistant", content: reply.text }]);
      if (reply.proposed) setProposal(reply.proposed);
    } catch (e) {
      setError(asFreshetError(e));
    } finally {
      setSending(false);
    }
  }

  async function start() {
    if (!proposal || creating) return;
    setCreating(true);
    setError(null);
    try {
      const summary = await bridge.createStream(proposal);
      onCreated(summary); // hand off; the reading view runs + animates the first synth
    } catch (e) {
      setError(asFreshetError(e));
      setCreating(false);
    }
  }

  return (
    <div className="create">
      <header className="create-header">
        <h1 className="create-title">New stream</h1>
        <button className="create-cancel" aria-label="Cancel" onClick={onCancel} type="button">
          Cancel
        </button>
      </header>

      <div className="create-thread" role="log">
        {messages.map((m, i) => (
          <div key={i} className="create-msg" data-role={m.role}>
            {stripJsonBlock(m.content)}
          </div>
        ))}
        {sending ? <div className="create-msg" data-role="assistant" data-pending>…</div> : null}

        {proposal ? (
          <div className="create-proposal" role="group" aria-label="Proposed stream">
            <p className="create-proposal-title">{proposal.title}</p>
            <ul className="create-proposal-sources">
              {proposal.sources.map((s) => (
                <li key={s.channel}>
                  <span className="create-proposal-channel">{s.channel}</span>
                  <span className="create-proposal-query">{s.query}</span>
                </li>
              ))}
            </ul>
            <button className="create-start" onClick={start} disabled={creating} type="button">
              {creating ? "Starting…" : "Start stream"}
            </button>
          </div>
        ) : null}
      </div>

      {error ? (
        <div className="create-agent-error">
          <AgentNotice error={error} onRecheck={async () => { await bridge.recheckAgents(); }} onRetry={send} />
        </div>
      ) : null}

      <form
        className="create-composer"
        onSubmit={(e) => { e.preventDefault(); void send(); }}
      >
        <input
          className="create-input"
          aria-label="Message"
          placeholder="What do you want to track?"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <button className="create-send" aria-label="Send" type="submit" disabled={sending}>
          Send
        </button>
      </form>
    </div>
  );
}

/** Hide the machine-readable proposal block from the chat bubble. */
function stripJsonBlock(text: string): string {
  return text.replace(/```json[\s\S]*?```/g, "").trim();
}
```

- [ ] **Step 4: Style it (`Create.css`)**

Replace `Create.css` with a calm chat layout that inherits the tokens (warm-paper/terminal). Minimum rules:

```css
.create { display: flex; flex-direction: column; height: 100vh; background: var(--bg); color: var(--ink); }
.create-header { display: flex; align-items: center; justify-content: space-between; padding: 1rem 1.5rem; }
.create-title { margin: 0; font-family: var(--serif); font-size: 1.3rem; color: var(--ink); }
.create-cancel { border: none; background: none; font-family: var(--mono); font-size: 0.8rem; color: var(--muted); cursor: pointer; }
.create-thread { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.85rem; padding: 1rem 1.5rem; max-width: 44rem; width: 100%; margin: 0 auto; }
.create-msg { font-family: var(--serif); font-size: 1.02rem; line-height: 1.55; max-width: 90%; }
.create-msg[data-role="assistant"] { color: var(--fg); align-self: flex-start; }
.create-msg[data-role="user"] { align-self: flex-end; background: var(--surface); border-radius: 12px; padding: 0.5rem 0.85rem; }
.create-msg[data-pending] { color: var(--muted-2); }
.create-proposal { align-self: flex-start; border: 1px solid var(--rule); border-radius: 12px; background: var(--surface-2); padding: 1rem 1.1rem; }
.create-proposal-title { margin: 0 0 0.6rem; font-family: var(--serif); font-size: 1.1rem; color: var(--ink); }
.create-proposal-sources { list-style: none; margin: 0 0 0.85rem; padding: 0; display: flex; flex-direction: column; gap: 0.35rem; }
.create-proposal-sources li { display: flex; gap: 0.6rem; font-family: var(--mono); font-size: 0.78rem; }
.create-proposal-channel { color: var(--accent); text-transform: uppercase; letter-spacing: 0.05em; min-width: 6rem; }
.create-proposal-query { color: var(--fg); }
.create-start { border: none; border-radius: 8px; background: var(--accent); color: var(--bg); font-family: var(--mono); font-size: 0.82rem; padding: 0.5rem 1rem; cursor: pointer; }
.create-start:disabled { opacity: 0.6; cursor: default; }
.create-composer { display: flex; gap: 0.6rem; padding: 1rem 1.5rem; max-width: 44rem; width: 100%; margin: 0 auto; }
.create-input { flex: 1; padding: 0.6rem 0.85rem; border: 1px solid var(--rule); border-radius: 10px; background: var(--surface-2); color: var(--ink); font-family: var(--serif); font-size: 1rem; }
.create-input:focus { outline: none; border-color: var(--accent); }
.create-send { border: none; border-radius: 10px; background: var(--surface); color: var(--ink); font-family: var(--mono); font-size: 0.8rem; padding: 0 1rem; cursor: pointer; }
.create-agent-error { padding: 0 1.5rem; max-width: 44rem; width: 100%; margin: 0 auto; }
```

- [ ] **Step 5: Run the Create tests**

Run: `npx vitest run src/views/Create/Create.test.tsx`
Expected: PASS.

- [ ] **Step 6: Fix `App.test.tsx`**

The "New stream navigates to Create" test asserts `getByLabelText("Topic")`. Change it to assert the chat composer:

```tsx
  expect(screen.getByRole("textbox", { name: /message/i })).toBeInTheDocument();
```

Run: `npx vitest run src/App.test.tsx` → PASS.

- [ ] **Step 7: Commit**

```bash
git add src/views/Create src/App.test.tsx
git commit -m "feat(create): conversational stream creation UI replacing the form"
```

---

## Phase 4 — Create → live reconciling

### Task 8: Reconcile animation + auto first-synth in the reading view

**Files:**
- Create: `src/views/Reading/Reconciling.tsx`
- Create: `src/views/Reading/Reconciling.css`
- Modify: `src/views/Reading/Reading.tsx`
- Test: `src/views/Reading/Reconciling.test.tsx`
- Test: `src/views/Reading/Reading.test.tsx`

- [ ] **Step 1: Write the failing Reconciling test**

Create `Reconciling.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { Reconciling } from "./Reconciling";

test("shows the current phase label", () => {
  render(<Reconciling phase="researching" />);
  expect(screen.getByText(/researching/i)).toBeInTheDocument();
});

test("renders a progressbar element", () => {
  render(<Reconciling phase="synthesizing" />);
  expect(screen.getByRole("progressbar")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run: `npx vitest run src/views/Reading/Reconciling.test.tsx`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement `Reconciling`**

`Reconciling.tsx`:

```tsx
import type { RefreshPhase } from "../../bridge/types";
import "./Reconciling.css";

const LABELS: Record<RefreshPhase, string> = {
  detecting: "Finding your agent…",
  researching: "Reading the sources…",
  synthesizing: "Writing the document…",
  done: "Done",
  error: "Something went wrong",
};

const ORDER: RefreshPhase[] = ["detecting", "researching", "synthesizing", "done"];

export function Reconciling({ phase }: { phase: RefreshPhase }) {
  const pct = Math.round(((ORDER.indexOf(phase) + 1) / ORDER.length) * 100);
  return (
    <div className="reconciling" aria-live="polite">
      <p className="reconciling-label">{LABELS[phase] ?? "Working…"}</p>
      <div
        className="reconciling-bar"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
      >
        <span className="reconciling-fill" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
```

`Reconciling.css`:

```css
.reconciling { max-width: 42rem; margin: 4rem auto 0; padding: 0 2rem; }
.reconciling-label { font-family: var(--mono); font-size: 0.85rem; color: var(--muted); letter-spacing: 0.02em; }
.reconciling-bar { margin-top: 0.75rem; height: 3px; border-radius: 2px; background: var(--rule); overflow: hidden; }
.reconciling-fill { display: block; height: 100%; background: var(--accent); transition: width 0.5s cubic-bezier(0.16, 1, 0.3, 1); }
@media (prefers-reduced-motion: reduce) { .reconciling-fill { transition: none; } }
```

- [ ] **Step 4: Run the Reconciling test**

Run: `npx vitest run src/views/Reading/Reconciling.test.tsx`
Expected: PASS.

- [ ] **Step 5: Wire it into `Reading` (auto first-synth + progress)**

In `Reading.tsx`, add phase state, subscribe to `onRefreshProgress`, and when the loaded document is empty, kick off the first synth. Add near the other state:

```tsx
  const [phase, setPhase] = useState<RefreshPhase | null>(null);
```

Add a subscription effect (import `RefreshPhase` from types):

```tsx
  useEffect(() => {
    const off = bridge.onRefreshProgress((e) => {
      if (e.streamId !== streamId) return;
      setPhase(e.phase);
      if (e.phase === "done") {
        bridge.getStream(streamId).then((r) => {
          setMarkdown(r.documentMarkdown);
          setDescription(r.description);
          setPhase(null);
        });
      }
    });
    return off;
  }, [bridge, streamId]);
```

In the initial load effect, when the document is empty, start the first synth and show progress:

```tsx
    bridge.getStream(streamId).then((r) => {
      if (!active) return;
      setMarkdown(r.documentMarkdown);
      setDescription(r.description);
      if (!r.documentMarkdown) {
        setPhase("researching");
        void bridge.refreshStream(streamId).catch((e) => {
          setRefreshError(asFreshetError(e));
          setPhase(null);
        });
      }
    });
```

In the render, when `phase` is set and there's no document yet, show the reconcile instead of the empty placeholder:

```tsx
      {phase && !markdown ? (
        <Reconciling phase={phase} />
      ) : doc ? (
        /* …existing reading-body… */
      ) : (
        <div className="reading-empty" aria-hidden />
      )}
```

Import `Reconciling`.

- [ ] **Step 6: Add a Reading test for the auto-synth path**

Add to `Reading.test.tsx`:

```tsx
test("an empty stream auto-runs the first synth and then shows the document", async () => {
  const bridge = new MockBridge();
  // Create an empty stream (persist-only).
  await bridge.createStream({
    id: "fresh", title: "Fresh", topic: "x",
    sources: [{ channel: "hackernews", query: "x" }],
    cadence: { mode: "on_launch" }, status: "active", createdAt: "2026-06-14T00:00:00Z",
  });
  const refreshSpy = vi.spyOn(bridge, "refreshStream");
  render(<BridgeProvider bridge={bridge}><Reading streamId="fresh" onBack={() => {}} /></BridgeProvider>);
  await vi.waitFor(() => expect(refreshSpy).toHaveBeenCalledWith("fresh"));
  // After the mock refresh populates the doc + emits "done", the title shows.
  expect(await screen.findByText("Fresh")).toBeInTheDocument();
});
```

(For this to pass, `MockBridge.refreshStream` must populate the empty document before emitting the phase events — implemented in Task 6, Step 6.)

- [ ] **Step 7: Run the reading tests + build**

Run: `npx vitest run src/views/Reading/ && npm run build`
Expected: PASS; build clean.

- [ ] **Step 8: Commit**

```bash
git add src/views/Reading
git commit -m "feat(reading): live reconcile animation + auto first-synth on empty stream"
```

### Task 9: End-to-end wiring check (Create → reconcile → document)

**Files:**
- Test: `src/App.test.tsx`

- [ ] **Step 1: Write the end-to-end test**

Add to `App.test.tsx`:

```tsx
test("create a stream via chat, then land on the reconciling stream", async () => {
  render(<App />);
  await userEvent.click(await screen.findByRole("button", { name: /new stream/i }));
  const input = screen.getByRole("textbox", { name: /message/i });
  await userEvent.type(input, "tokio");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  await userEvent.type(input, "just releases");
  await userEvent.click(screen.getByRole("button", { name: /send/i }));
  await userEvent.click(await screen.findByRole("button", { name: /start stream/i }));
  // We land on the new stream; its document appears after the mock synth.
  expect(await screen.findByRole("textbox", { name: /my notes/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run it**

Run: `npx vitest run src/App.test.tsx`
Expected: PASS. If the reconcile never resolves, confirm `MockBridge.refreshStream` emits a terminal `done` phase and populates the document.

- [ ] **Step 3: Full suites + build**

Run: `npx vitest run && cargo test --manifest-path src-tauri/Cargo.toml --lib && npm run build`
Expected: all green, build clean.

- [ ] **Step 4: Commit**

```bash
git add src/App.test.tsx
git commit -m "test(create): end-to-end chat-create → reconcile → document"
```

---

## Self-review checklist (run before handing off)

1. **Spec coverage:** replace-form-with-chat (Task 7) ✓ · phase-events + motion (Task 8) ✓ · per-source template (Tasks 1–2, surfaced in proposal card Task 7) ✓ · fixes single-term sourcing (per-source queries flow through `fetch_each`) ✓ · navigate-on-create + reconcile (Tasks 5, 8, 9) ✓.
2. **Type consistency:** `SourceQuery {channel, query}` identical in Rust (`model.rs`) and TS (`types.ts`); `StreamDescription.sources: SourceQuery[]` both sides; `chat_designer` ⇄ `chatDesigner` returns `{text, proposed?}`; `create_stream`/`createStream` is persist-only both sides; `refresh_progress`/`onRefreshProgress` phases (`detecting|researching|synthesizing|done|error`) unchanged.
3. **Removed cleanly:** `generate_first_draft`/`DraftInput`/`DraftResult`/`description_from_input` (Rust) and `generateFirstDraft`/`DraftInput`/`DraftResult` (TS) — grep to confirm no references remain (`grep -rn "generate_first_draft\|generateFirstDraft\|DraftResult\|DraftInput" src src-tauri/src`).

## Known follow-ups (out of scope for this plan)

- **Editable proposal card** (tweak sources/queries/cadence before Start) — v1 refines via chat only.
- **Token-by-token streaming** of the synthesis (needs `claude --output-format stream-json`) — v1 uses phase events.
- **Per-source provider query semantics** — `github` already treats the query as a search string; confirm `polymarket`/`hackernews` providers use the query sensibly (they take `topic` today; `fetch_each` just passes the per-source query into the same `fetch(query, limit)`).
- **Reconcile section-by-section fade-in** of the finished document (CSS stagger) — cosmetic polish.
