# Freshet v1 — Design Spec

*Status: design (2026-06-12). Scope: a **functional v1 of the whole product**, end-to-end,
built by a 4-teammate agent team (one per vertical). See
[`docs/product-vision.md`](../../product-vision.md) for the product vision, `agent-docs/` for
how the team is orchestrated, and [`docs/rust-primer.md`](../../rust-primer.md) for the Rust.*

> This supersedes the earlier "walking skeleton" framing. v1 is functional but **not
> polished** — UI craft/animation and credentialed sources are deliberately deferred.

> **Update (2026-06-14) — the backend + two refinements live in
> [Backend & Core Features](./2026-06-14-backend-and-core-features-design.md):**
> (1) **stream creation is now a chat** (collaborative scoping → description + first draft); the
> form becomes its editable *result*, not the entry point. (2) **First-run onboarding** is designed
> there (welcome → root folder → agent detection → first stream). (3) **Sourcing moved** from "the
> agent runs `last30days`" to a pluggable **`SourceProvider`** — Rust fetches the free channels
> directly (agent-agnostic; the `Agent` now only *synthesizes*, not *researches*). Where this spec
> and that one differ on the backend, **that one wins**.

---

## 1. Purpose

Ship a **usable Freshet**: create a stream by describing a curiosity, have it research across
many channels, synthesize a living document, and on later refreshes **tell you only what
changed** — staying quiet when nothing did. Every architectural layer is real and functional;
visual polish comes later. The build is parallelized across an agent team, so the spec's job
is to **lock the shared contracts** (§5) tightly enough that four teammates can work at once
without colliding.

## 2. Success criteria (demoable)

1. **Create a stream** from a form (topic · sources · cadence); the app generates a **first
   draft** you preview before committing; committing writes a stream description + the document.
2. **Quiet-desk home** lists multiple streams, each with last-checked time and a soft
   "something moved" mark when a refresh changed it. Refresh is **non-blocking**.
3. **Refresh** (manual, on-launch, or interval) drives the **detected local agent** to research
   the stream's channels, then updates the **living document** — plain markdown on disk.
4. A **second** refresh updates the document with a **"What changed"** section reflecting only
   genuinely new items, **or** reports **"nothing new"** (no agent synthesis call needed).
5. **Reading view** renders the living document.
6. The agent layer is **agent-agnostic by construction** — the `Agent` trait + a second-agent
   stub prove it isn't Claude-shaped — with **Claude Code the fully-wired adapter in v1**; adding
   another agent means implementing one trait (its own sourcing strategy may differ from `last30days`).

## 3. Scope

**In scope (functional, not polished)**
- **Three surfaces, working:** stream-creation (form + generated first-draft preview) · quiet-desk
  home (multi-stream, refresh, "something moved") · reading view (the living document).
- **Multi-channel sourcing** via the detected agent (Claude → the `last30days` skill), limited to
  **free / no-key channels**: Reddit, Hacker News, GitHub, Polymarket (Bluesky optional via app password).
- **The watch engine:** remember + reconcile + "what changed" + the "nothing new" restraint path.
- **Agent-agnostic integrations:** discover the local agent, abstract it behind a trait, drive it
  for research + synthesis; design the §9 fallback ladder.
- **Cadence:** manual + on-launch + per-stream **interval** (a small scheduler).
- **Multiple streams**; pause / retire a stream.
- **Version history** — lightweight per-document history (timestamped snapshots under
  `.freshet/history/` in v1; embeddable git later as a sync substrate), giving history,
  refresh-to-refresh diffs, and safe revert (backend phase).
- **Stack:** Tauri 2 + Rust core + React/Vite frontend.

**Out of scope (deferred)**
- UI polish / signature animations / native macOS depth ("make it pop").
- **Credentialed channels** (X login; YouTube `yt-dlp`; TikTok/IG/Threads/Pinterest via
  ScrapeCreators; Perplexity via OpenRouter; Brave web search) — the "more sources" phase.
- Local-model and raw-API-key providers as *implemented* fallbacks (designed in §6, not built).
- Packaging / signing / auto-update; deep settings; the always-on cloud runner (vision §11).

## 4. Invariants honored (vision §9)

- **Non-blocking** — refresh runs on a background task; the window never freezes.
- **Stateful** — remembers incorporated items; enables dedup + change detection.
- **Plain markdown out** — the living document is plain `.md`; machine state lives in a hidden
  `.freshet/` subfolder, never polluting the user's notes folder.
- **Quiet by default** — "nothing new" is a first-class, cheap outcome; never manufacture novelty.
- **Significance over recency** — `last30days` already ranks by engagement/money; the engine
  prefers significant change over the merely new.
- **Local-first & BYO-agent** — runs on the user's machine via a **detected local agent (any
  supported), not just Claude**; fall back to local model, then API key (designed).
- **Don't rebuild the vault** (the vision's term for the user's notes folder) — write a document
  into the user's folder; do not become a note manager.

## 5. Shared contracts (the crux — lock these first)

These interfaces are the seams between verticals. The team agrees them **before** parallel work.
They are owned jointly and defined here; changes require coordination across teammates.

### 5.1 Stream description — created by the creation surface, consumed by the engine

`<root>/.freshet/streams/<id>.json`. *The exact artifact the creation form emits; the engine
never knows whether a human or a future chat produced it.*

```json
{
  "id": "ai-agents",
  "title": "AI Agents",
  "topic": "AI agent frameworks and autonomous coding agents",
  "sources": ["reddit", "hackernews", "github", "polymarket"],
  "cadence": { "mode": "interval", "interval_minutes": 1440 },
  "status": "active",
  "created_at": "2026-06-12T00:00:00Z"
}
```

`cadence.mode` ∈ `manual | on_launch | interval` (interval requires `interval_minutes`).
`status` ∈ `active | paused | retired`.

### 5.2 Living document — `<root>/<title>.md`, plain markdown, "what changed" on top

**Ownership (model B).** Freshet **owns** three sections — What changed · Current understanding ·
Open questions — and rewrites them on refresh. The **`## My notes` block at the bottom is the
user's**: Freshet reads it, preserves it **verbatim**, and never edits it. (If the user edits a
Freshet-owned section in an external app, the next refresh overwrites it; `## My notes` is the
safe place to annotate.)

```markdown
# AI Agents
_updated 2026-06-12 14:03_

## What changed
- …only genuinely new, significant items, cited…

## Current understanding
- …settled, organized, cited…

## Open questions
- …kept honestly open…

## My notes
<!-- yours — Freshet never edits anything below this heading -->
- …your annotations…
```

### 5.3 State sidecar — `<root>/.freshet/state/<id>.json`, hidden, never shown to the note app

```json
{ "seen_item_ids": ["reddit:abc", "hackernews:123", "github:owner/repo#42"],
  "last_checked_at": "…", "last_changed_at": "…", "doc_digest": "sha256:…" }
```

Item ids are **source-qualified** (`<source>:<native-id>`) so dedup is unambiguous across channels.

### 5.4 Agent trait — owned by `integrations`, consumed by the engine

```rust
enum AgentKind { ClaudeCode, Codex, /* … */ }

struct ResearchBrief {
    items: Vec<SourceItem>,   // for dedup: id, source, url, title, score, snippet
    brief_markdown: String,   // the agent's synthesized prose, with citations
}

trait Agent {
    fn kind(&self) -> AgentKind;
    fn research(&self, topic: &str, sources: &[String]) -> anyhow::Result<ResearchBrief>;
    fn synthesize(&self, prompt: &str) -> anyhow::Result<String>;
}

fn detect_agents() -> Vec<Box<dyn Agent>>;   // probe PATH; ordered by preference
```

The **`ResearchBrief.items` list is the contract the engine dedups against** — every agent
adapter must return source-qualified items, not just prose. How a given agent produces them
(Claude via `last30days`; others via their own capability) is the adapter's concern.

### 5.5 Bridge — Tauri commands (frontend → core) and events (core → frontend)

```
commands:
  list_streams() -> [StreamSummary{ id, title, last_checked_at, changed_since_seen }]
  get_stream(id) -> { description, document_markdown, state_summary }
  generate_first_draft(input: DraftInput) -> { draft_markdown, proposed_description }
  create_stream(description) -> StreamSummary
  refresh_stream(id) -> Summary{ changed: bool, n_new: u32 }
  save_notes(id, markdown)                 // persist the user's `## My notes` block only
  set_stream_status(id, status)            // pause / retire / reactivate
  get_config() / set_root_folder(path)
  list_agents() -> [AgentInfo{ kind, available }]
events:
  refresh_progress { stream_id, phase: detecting | researching | synthesizing | done | error, detail? }
  stream_updated   { stream_id, changed }
```

The bridge boundary returns `Result<_, String>`; rich `anyhow` errors are stringified there.

## 6. The integrations layer (agent-agnostic)

Owned by the **integrations** teammate. Responsibilities:

1. **Discovery** — probe for supported agents on `PATH` (`claude`, `codex`, …), check they run,
   record availability + a preference order. Surface via `list_agents()`.
2. **Abstraction** — implement `Agent` (§5.4) per supported agent. v1 ships the **Claude Code**
   adapter (drives the `last30days` skill for `research`, and `claude -p` for `synthesize`) and a
   **second adapter stub** (e.g. Codex) proving the abstraction isn't Claude-shaped.
3. **Sourcing flow** — given `(topic, sources)`, instruct the agent to research the **free/no-key
   channels** and return a `ResearchBrief` with **source-qualified items** + prose. For Claude this
   wraps `last30days` (constraining it to the free channels and capturing its items).
4. **Fallback ladder (designed, partial build)** — agent → local model → API key, per §9. v1
   implements the agent path; the trait + selection logic leave clean seams for the others.

> Open design point (for integrations): the exact mechanism to get **structured items** out of
> `last30days` (parse its markdown/citations vs. request a JSON emit). The contract (§5.4) is fixed;
> the extraction is the adapter's to solve. Tracked in §15.

## 7. The watch engine (the heart) — owned by `backend`

On refresh for a stream:

1. Load the stream description + prior document + prior state.
2. Ask the selected `Agent` to `research(topic, sources)` → `ResearchBrief`.
3. **Dedup**: `new = brief.items` whose source-qualified id ∉ `seen_item_ids`.
4. **If `new` is empty** → record `last_checked_at`; emit `done` ("nothing new"); **no
   `synthesize` call**. (Restraint + cheap.)
5. **Else** → reconcile **incrementally**: extract the user's `## My notes` block; `synthesize(prior
   Freshet-owned doc + new items)` → fold `new` into "Current understanding" (never regenerate from
   scratch), rewrite "What changed" to this delta, update "Open questions"; **re-attach `## My notes`
   verbatim**; atomically write the document; update state (`seen_item_ids += new ids`,
   `last_changed_at`, `doc_digest`); emit `done` (changed) + `stream_updated`.

The user's `## My notes` block (model B, §5.2) is **never** sent to the agent for rewriting and is
always preserved byte-for-byte across refreshes.

Significance: because `last30days` items carry engagement scores, the engine can prefer
significant new items and keep low-signal noise out of "What changed".

## 8. Surfaces (functional, not polished) — owned by `frontend`

- **Stream creation** — a form (topic · multiselect sources · cadence) → calls
  `generate_first_draft` → shows the draft + proposed description → `create_stream` on commit.
  (The full collaborative *chat* is a later polish-phase upgrade.)
- **Quiet-desk home** — lists streams from `list_streams`; each row shows last-checked + a soft
  "something moved" mark when `changed_since_seen`; a per-stream and global "Refresh now"; pause/retire.
  No badges, no counts, no spinners gating the view.
- **Reading view** — renders `document_markdown` (What changed → Current understanding → Open
  questions) **read-only**, with the `## My notes` block **editable** and saved via `save_notes`.
  Plain, legible typography; the *signature* reconcile animation is deferred.

All three are built in the browser-mock loop first (mock bridge), then wired to real Tauri commands.

## 9. Cadence / scheduler — owned by `backend`

- **manual** — "Refresh now" (per stream + global).
- **on_launch** — streams with `on_launch`/`interval` refresh shortly after app start, in the
  background, never blocking the window.
- **interval** — a lightweight in-process scheduler ticks per stream every `interval_minutes`
  while the app is open. (A desktop app only runs when open; the always-on cloud runner is vision §11.)

## 10. Data & file layout

```
<root>/                         ← user-configured root folder
  <Stream Title>.md             ← living document (visible, plain markdown)
  .freshet/                     ← hidden; Freshet's private state
    config.json                 ← root folder, selected agent, preferences
    streams/<id>.json           ← stream descriptions
    state/<id>.json             ← per-stream run state (seen ids, timestamps, digest)
```

### Version history

Freshet keeps a **lightweight per-document history** so you can see how a topic's understanding
evolved, diff one refresh against the last (a file-level view of what changed), and safely revert a
bad synthesis.

**v1 — timestamped snapshots.** On each change, write a copy to `.freshet/history/<id>/<iso>.md`
(plus a small index). Zero external dependency, inherently linear (Freshet is the only writer), and
no `.git` to collide with a vault the user already version-controls. Diff/revert over markdown are trivial.

**Later — embeddable git** (pure-Rust `gitoxide`, no `git` binary required) if/when the cloud sync
(vision §11) lands: `git push/pull` is the natural sync substrate and standard tooling can inspect
history. **Not Mercurial** — a heavier real dependency (extra install, no embedded Rust library).

Either way this is **complementary** to the state sidecar (§5.3): history is human-facing; the
sidecar remains the change-detection mechanism.

## 11. Error handling

- **Agent missing / none detected** → creation + refresh surface "no local agent available" with
  guidance; nothing is corrupted. `list_agents()` drives a clear empty-state.
- **Research / synthesize fails** → `error` event; quiet inline message; document + state untouched.
- **Atomic writes** — temp file → rename — so a crash never leaves a half-written document or torn state.
- **Never block the UI** — all refresh work runs on background tasks; the window stays responsive.

## 12. Testing

- **Rust unit tests** (`backend`): dedup (source-qualified ids), the "nothing new" path, reconciler
  new-item selection, atomic writes, the scheduler's tick logic, serde round-trips of all §5 schemas.
- **Agent trait faked** (`integrations` + `backend`): a fake `Agent` returns canned `ResearchBrief`s;
  no real agent in tests. Integration test: seed + fixture brief → refresh twice → assert run 1
  populates the document, identical run 2 = "nothing new", run 2 + one extra item = it appears in
  "What changed".
- **Agent adapter tests** (`integrations`): the Claude adapter against recorded `last30days` output
  fixtures (no live network); item-extraction correctness.
- **Frontend** (`frontend`): the three surfaces against a mock bridge in the browser-mock loop;
  a component test rendering a document.

## 13. Build / dev approach

- Develop the frontend as a web app at localhost against a **mock bridge** first; build the Rust core
  as a **library + a small CLI harness** (headless, also the integration-test driver); then wrap in
  Tauri, swapping the mock bridge for real `invoke` / events.
- The CLI harness lets `integrations` and `backend` exercise the full refresh loop without the UI.

## 14. Team plan (4 verticals)

Per `agent-docs/` (use `agent-docs/team-prompt-template.md`). Sonnet teammates; the bridge (§5.5)
and Agent trait (§5.4) are the seams where teammates coordinate; require plan approval for changes
to the output-folder write path, the non-blocking guarantee, or any §5 contract.

| Teammate | Owns | Produces |
| :--- | :--- | :--- |
| **frontend** | React/Vite SPA — the three surfaces + bridge client | functional UI wired to the bridge |
| **backend** | Rust core — engine, store, scheduler, bridge server | the watch loop + files + commands/events |
| **integrations** | the `Agent` trait, agent discovery, the Claude/`last30days` adapter + a second-agent stub | agent-agnostic sourcing returning `ResearchBrief` |
| **review** | acceptance vs. §2 success criteria and §9 invariants | a prioritized review + sign-off |

**Contracts-first:** §5 is agreed and stubbed before parallel work, so the four can run at once.
Per the decision on doc granularity, this spec is the shared overview; **writing-plans** produces
one combined implementation plan with frontend / backend / integrations / review sections +
acceptance criteria.

## 15. Open questions (deferred, not blocking)

- **Structured-item extraction from `last30days`** — parse its markdown/citations vs. request a JSON
  emit; the §5.4 contract is fixed regardless (integrations to solve).
- **Default root-folder location** and the first-run flow to set it (can default to a dev path).
- **Reconciliation prompt** wording — tuned during implementation against real agent output.
- **Second-agent adapter** — which agent to use as the non-Claude proof (Codex assumed); how it sources
  without `last30days`.
- **"Significance" threshold** — how aggressively to filter low-score new items out of "What changed".
- **Version-history mechanism** — snapshots for v1 vs. adopting embeddable git earlier (only worth it
  once cloud sync exists); if git, where the repo lives and how to coexist with a user-managed vault (§10).
