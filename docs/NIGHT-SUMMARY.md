# Night Summary — 2026-06-14 (autonomous backend build)

Good morning. Here's exactly what happened overnight, what's solid, what's untested, and where to pick up.

## TL;DR

The **entire light backend is built, wired, and tested**, plus the **first-run onboarding UI**.
Freshet now runs end-to-end in the native window against the real backend (your local agent + live
channels) — but that **live path is unverified** (I can't safely drive your `claude`/`codex` or hit
real APIs unattended). Everything *around* it is tested against fakes/fixtures: **135 Rust + 63 TS
tests pass; both builds clean.** Branch: `walking-skeleton`. 10 commits tonight (`f4e7c25`…`039435c`).

## What landed

| Layer | Status | Tests |
| :--- | :--- | :--- |
| **Spec** — backend & core features (onboarding, chat creation, light backend) | written + committed | — |
| **Store** — atomic IO, config, model-B My-notes splice, history snapshots | done | ✓ |
| **SourceProviders** — HN / Reddit / GitHub / Polymarket over recorded fixtures | done | ✓ |
| **Agent layer** — 3-tier PATH-robust detection (Tolaria-style) + macOS GUI-PATH fix + claude/codex headless adapters + FakeAgent | done | ✓ |
| **Engine** — the watch loop (fetch → dedup → reconcile → nothing-new), My-notes preserved & never sent to the agent | done | ✓ (the two-run thesis test) |
| **Scheduler** — `is_due` / `runs_at_startup` / `due_for_tick` | done | ✓ |
| **TauriBridge** — all 14 commands, events, background refresh, startup detection + scheduler tick, CLI harness | done | ✓ |
| **Frontend wiring** — `TauriBridge` + auto-select (Tauri→real, browser→mock); `refresh_stream` returns `Summary` | done | ✓ |
| **Onboarding UI** — welcome → folder → agent-state → app (gated, calm, native folder picker) | done | ✓ |
| **Creation *chat* UI** (your idea #2) | **deferred** — the existing **form works end-to-end** (`generate_first_draft` + `create_stream` are wired) | — |

## Decisions I made (so you can veto)

1. **Sourcing = fetch the 4 free channels ourselves** (agent-agnostic), behind a pluggable
   `SourceProvider`. `last30days`/richer channels are a future provider (todo #19). *(You leaned
   "last30days first" but invited my view; this was the only path verifiable tonight + honors "any agent".)*
2. **No agent found → guide to install** (no API-key handling in v1).
3. **`refresh_stream` returns `Summary`** (awaits the engine; still emits progress events) — matches
   the frontend contract; scheduler refreshes stay fire-and-forget.
4. **Wire casing:** enums snake_case (`"on_launch"`, `"claude_code"`), structs camelCase
   (`changedSinceSeen`) — the existing frontend `types.ts` needed zero changes.
5. Minor: `FETCH_LIMIT = 30`/provider; `changedSinceSeen` is best-effort (`last_changed_at >=
   last_checked_at` — no persisted "last opened" marker yet); reconcile/chat prompt wording is
   provisional (needs your tuning).

## ⚠️ What's UNVERIFIED (33 `// UNVERIFIED: live path` markers)

These exist, compile, and are logically tested via fakes — but were **never actually run**:
- Driving the real `claude`/`codex` (the headless argv is built & unit-tested; the *spawn* is not).
- Real channel HTTP (the parsers are fixture-tested; the *live fetch* is not — and the Reddit/Polymarket
  JSON shapes are my best reconstruction, worth confirming against a real response).
- The system clock (`now_iso`) and all Tauri background tasks.

**This is the mile to debug with me.**

## Try it (the live path)

```
npm run tauri dev          # native window; first build is cached now, fast
```
→ onboarding: pick a folder → it detects your installed `claude` → land in the app → **New stream**
(the form): topic + sources + cadence → Preview → Create. That Create will *really* fetch HN/Reddit/
GitHub/Polymarket and drive your `claude` to synthesize the first document. **Expect bugs here** —
that's the unverified mile. (Browser `npm run dev` still works against the mock for UI work.)

## Recommended next steps (in order)

1. **Run the live path** above; we debug real agent invocation + channel HTTP together.
2. **Tune the reconcile prompt** (`src-tauri/src/agent/mod.rs::build_reconcile_prompt`) against real output.
3. **Confirm the Reddit & Polymarket API shapes** against a live response; adjust the parsers if needed.
4. **Build the creation *chat* UI** (Phase 9, spec §6) — and tell me what "generate a bump eventually" meant
   (I read it as *generate the first-draft document*; flagged in the spec §11).
5. **A final adversarial review** of the backend (I did per-phase reviews; a whole-backend pass is worth it).

## Where everything is

- Specs: `docs/superpowers/specs/2026-06-14-backend-and-core-features-design.md` (+ the v1 + frontend specs).
- Plan: `docs/superpowers/plans/2026-06-14-freshet-backend.md`.
- Rust backend: `src-tauri/src/{model,store,sources,agent,engine,scheduler,bridge,commands}.rs` + `bin/freshet_cli.rs`.
- Frontend wiring + onboarding: `src/bridge/TauriBridge.ts`, `src/views/Onboarding/`.
- It's all behind the **`Bridge` / `Agent` / `SourceProvider`** seams — changes are cheap and localized.

Everything compiles and the tests are green. The risky part is honest and labeled. Nothing destructive ran.
