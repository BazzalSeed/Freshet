# Freshet Walking Skeleton — Design Spec

*Status: approved design (2026-06-12). Scope: the first sub-project of Freshet — a thin
end-to-end vertical that threads every architectural layer. See
[`docs/product-vision.md`](../../product-vision.md) for the product vision and `agent-docs/`
for how the implementation team will be orchestrated.*

---

## 1. Purpose

Prove Freshet's core thesis — **watch → remember → tell what changed** — end-to-end through
every architectural layer, with each layer paper-thin but real and connected. The skeleton
exists to **de-risk the architecture** (especially the engine loop and the frontend↔native
bridge) and to create the real codebase the vision §10 decisions can then be designed
against. It is not a product release; it is the spine everything else hangs on.

## 2. Success criteria (demoable)

1. The app opens to a **quiet desk** showing one seeded stream and its last-checked time.
2. Clicking **"Refresh now"** is **non-blocking** — the UI stays live, a progress indicator
   advances, then the living document appears in the **reading view**.
3. The living document is **plain markdown on disk** in the configured root folder (any editor can open it).
4. A **second** "Refresh now":
   - updates the document, with a **"What changed"** section reflecting *only* genuinely new
     items (dedup works), **or**
   - reports **"nothing new"** when there are no new items (restraint — and no LLM call).
5. All logic lives in the **Rust core**; the frontend is thin; one typed command + one event
   channel cross the bridge.

If a fresh observer can watch run #1 produce a document and run #2 surface only what's new (or
honestly say nothing changed), the skeleton has done its job.

## 3. Scope

**In scope**
- One **seeded** stream (a hand-written stream-description file — the artifact the future
  creation chat will emit).
- One **source**: Hacker News via the Algolia search API (keyword search, no auth).
- **Manual** trigger only ("Refresh now").
- One **LLM provider**: the detected local `claude` CLI (`claude -p`), behind a trait.
- **Reconciliation** of new findings into the living document.
- **Output-folder** write of the markdown document + a hidden state sidecar.
- **Minimal UI**: desk (one stream) + reading view + refresh button.
- **Two-run change detection** and the **"nothing new"** path.

**Out of scope (each a later sub-project)**
- Stream-creation chat (collaborative scoping → live first draft).
- Scheduler / cadence (on-launch / interval). Trigger is manual only.
- Multiple sources; source ranking by significance beyond minimal ordering.
- The full BYO-LLM ladder (local model, API key). Skeleton requires `claude` on `PATH`.
- Multi-stream; settings; design polish / signature animations; packaging / signing.

## 4. Invariants honored (vision §9)

The skeleton must not violate these even while thin:

- **Non-blocking** — refresh runs on a background task; the window never freezes.
- **Stateful** — it remembers what it already incorporated, enabling dedup + change detection.
- **Plain markdown out** — the living document is plain `.md`; machine state is kept in a
  **separate hidden sidecar**, never polluting the output folder the user reads.
- **Quiet by default** — "nothing new" is a first-class, cheap outcome; never manufacture novelty.
- **Local-first & BYO-LLM** — runs on the user's machine via a detected local agent.
- **Don't rebuild the vault** (the product vision's term for the user's notes folder) — write a document
  into the user's folder; do not become a note manager.

## 5. Architecture

### 5.1 Layers

```
Frontend (React/Vite in the webview)
  • Desk view     — the one stream, last-checked time, soft "something moved" mark
  • Reading view  — renders the living-document markdown
  • "Refresh now" — invokes the bridge command
Bridge (Tauri)
  • command  refresh_stream(stream_id) -> Summary
  • event    refresh_progress { stream_id, phase: fetching | synthesizing | done | error }
Core (Rust)
  • StreamStore   — load the seeded stream description; resolve root-folder paths
  • SourceAdapter — HN: query(topic) -> [Item{ id, title, url, points, created, snippet }]
  • Memory/State  — load/save seen item ids + prior doc digest
  • Reconciler    — (prior_doc, prior_seen, new_items) -> (new_doc, changed_summary, new_seen)
  • Provider      — local `claude -p`, behind a trait (faked in tests)
  • Store         — atomic read/write of the .md document + the hidden .json state sidecar
```

### 5.2 Data flow — one refresh

```
click "Refresh now"
  → command refresh_stream(id)
  → core: load stream description + prior document + prior state
  → SourceAdapter: fetch HN items for the topic
  → dedup: new = fetched ids not in seen_item_ids
  → if new is empty:
        record last_checked_at; emit progress(done, "nothing new"); return Summary{changed:false}
  → else:
        Provider synthesizes (prior doc + new items) → updated doc + "What changed"
        Store: atomic-write document; update state (seen += new ids, last_changed_at)
        emit progress(done, "updated"); return Summary{changed:true, n_new}
  → frontend re-reads the document and renders it; shows the "something moved" mark if changed
```

The UI thread is never on this path — the core runs the refresh on a background task and the
frontend learns of progress via events.

## 6. Data structures

The user configures one **root folder**; Freshet maintains its structure inside it. The living
document is **visible plain markdown at the root**; Freshet's private files (the seed stream
description and run state) live in a **hidden `.freshet/` subfolder** so they never clutter the
user's note app. The skeleton defaults the root to a fixed dev path (see §12).

**Stream description** — the seed file at `<root>/.freshet/streams/<id>.json`. *This is the
exact contract the future creation chat will emit, so the engine never knows whether a human
or the chat produced it.*

```json
{ "id": "ai-agents", "title": "AI Agents", "topic": "AI agent frameworks",
  "source": "hackernews", "created_at": "2026-06-12T00:00:00Z" }
```

**Living document** — `<root>/<title>.md`, plain markdown, "what changed" at the top per §4:

```markdown
# AI Agents
_updated 2026-06-12 14:03_

## What changed
- …only genuinely new items…

## Current understanding
- …settled, organized, cited…

## Open questions
- …kept honestly open…
```

**State sidecar** — `<root>/.freshet/state/<id>.json`, hidden and never visible to the note app:

```json
{ "seen_item_ids": ["hn_123", "hn_456"], "last_checked_at": "…",
  "last_changed_at": "…", "doc_digest": "sha256:…" }
```

## 7. The "what changed" mechanic (the heart)

- HN items carry stable ids; `seen_item_ids` tracks the ones already incorporated.
- **New** = fetched ids not in `seen_item_ids`.
- **Empty new** → skip the LLM entirely, record `last_checked_at`, report "nothing new". Cheap
  and honest — this is the restraint invariant made concrete.
- **Non-empty new** → the provider receives `(prior document + new items)` and returns the
  updated document with "What changed" populated from those items; `seen_item_ids` grows and
  `last_changed_at` updates.
- This makes **run #2 the payoff**: the same fetch yields "nothing new"; a fetch with one extra
  item surfaces exactly that item under "What changed".

## 8. Error handling (thin but real)

- **HN fetch fails** → `error` event; quiet inline message in the UI; document + state left
  **untouched**.
- **`claude` CLI missing or fails** → detected at the start of refresh; surface "no LLM
  available"; never corrupt the document. The skeleton documents that `claude` must be on `PATH`.
- **Store writes are atomic** — write to a temp file, then rename — so a crash never leaves a
  half-written document or a torn state file.
- **Never block the UI** — the refresh runs on a background task in the Rust core; the frontend
  stays responsive throughout (this is the non-blocking invariant under test).

## 9. Testing strategy

- **Rust unit tests**: HN adapter against a **recorded JSON fixture**; dedup logic; reconciler
  new-item selection; atomic write; the "nothing new" path.
- **Provider behind a trait**: tests inject a **fake** provider returning canned text — no real
  LLM call in tests. The real `claude` CLI is used only at runtime.
- **One integration test** via a headless CLI harness: seed + fixture → run refresh twice →
  assert (a) run 1 populates the document; (b) an identical run 2 reports "nothing new";
  (c) run 2 with one extra item surfaces it under "What changed".
- **Frontend**: minimal; verified via the browser-mock loop, optionally one component test that
  renders a document.

## 10. Build / dev approach (vision §6)

- Develop the frontend as a **web app at localhost** against a **mock bridge** first.
- Build the Rust core as a **library + a tiny CLI harness** — headless, and doubling as the
  integration-test driver.
- Then **wrap in Tauri**, swapping the mock bridge for real `invoke` / events.
- Net effect: the core is exercisable both headlessly (CLI/tests) and in the window, and the
  bridge seam is real without fighting native packaging early.

## 11. Documentation & delivery plan

This spec is the **shared, authoritative overview**. Next:

1. **writing-plans (one pass)** → a single combined implementation plan with clearly-marked
   **frontend / backend / review** sections and explicit file ownership, plus an
   acceptance/definition-of-done section derived from §2 and §9.
2. **Spawn an implementation agent team** (see `agent-docs/team-prompt-template.md`) that shares
   this spec + the plan as repo files: a **backend** teammate owning the Rust core, a
   **frontend** teammate owning the React/Vite UI + mock bridge, and a **review** teammate
   owning acceptance against §2/§9. The bridge contract (§5.1) is the shared interface where the
   frontend and backend teammates must coordinate. Use Sonnet; require plan approval for changes
   that touch the output-folder write path or the non-blocking guarantee.

## 12. Open questions (deferred, not blocking)

- Exact default root-folder location and how the user configures it (skeleton can default to a
  fixed dev path).
- The precise reconciliation prompt wording — tuned during implementation against real HN output.
- Whether the headless CLI harness ships as a real `freshet` subcommand or stays test-only.
