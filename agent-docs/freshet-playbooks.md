# Freshet Agent-Team Playbooks

Copy-paste team prompts tuned to Freshet's actual surfaces and stack. Read the
[master reference](./agent-teams-reference.md) first. These are starting points — adjust
team size and lenses to the task.

**Stack context (from README):** local-first Tauri 2 desktop app · React + Vite SPA
frontend (own design language, no stock kit; Radix/Ark headless primitives) · Rust native
backend · thin typed command + event bridge between frontend/native · research fan-out +
reconciliation engine · BYO-LLM (detected local agent → local model → API key) · plain
markdown output into a vault folder.

**Invariants any teammate must honor (README §9):** installable local app (never hosted) ·
push not pull · stateful/dedup · quiet & non-blocking (the window never blocks on refresh) ·
significance over recency · local-first & BYO-LLM · don't rebuild the vault · topics not
accounts · calm over engagement.

> ⚠️ This is greenfield — only `README.md` exists today. Until the code lands, lean on
> **research/design teams** (high value, no merge conflicts) over parallel-implementation teams.

---

## Playbook 1 — Architecture design jam (do this first)

Freshet leaves the data model, scheduling, source set, and reconciliation to the builder
(README §10). A design team explores tradeoffs in parallel before any code exists.

```text
Create an agent team to design Freshet's core architecture from README.md. Spawn four
teammates, each owning one decision and producing a short design doc under docs/design/:
- "data-model": the vault/state schema — how streams, the living document, and
  dedup/change-detection state are stored as local files. Must honor: stateful, plain
  markdown output, don't rebuild the vault.
- "scheduler": the per-stream, event-driven cadence engine (on-launch / manual /
  interval). Must be non-blocking — the window is live immediately, results fill in.
- "research-engine": research fan-out + reconciliation — how sources are queried in
  parallel and synthesized into "what changed / settled / open". Significance over recency.
- "bridge": the Tauri frontend↔Rust typed command + event boundary and BYO-LLM
  detection (local agent → local model → API key).
Have them message each other where decisions interact (e.g. scheduler ↔ research-engine,
data-model ↔ bridge). Use Sonnet. Then synthesize a single architecture overview.
```

Why a team: the four decisions are genuinely independent but interact at the seams —
exactly where teammate-to-teammate messaging earns its cost.

---

## Playbook 2 — Source-integration research (competing options)

Picking the source set (Reddit, HN, GitHub, Polymarket, Bluesky, …) is research-heavy and
parallelizes cleanly.

```text
Create an agent team to evaluate candidate data sources for Freshet streams. One teammate
per source (Reddit, Hacker News, GitHub, Polymarket, Bluesky). Each reports: API
legitimacy & auth model, rate limits, what signal it offers, and whether a future
cloud runner could use it (API-legitimate) vs. local-only (session-based). Flag anything
needing a logged-in session as deferred (README §10). Use Sonnet. Synthesize a ranked
"sensible API-legitimate start" set.
```

---

## Playbook 3 — Three-lens review of a signature surface

Freshet has three bespoke surfaces carrying the feel: the **reading view**, the
**quiet-desk home**, the **stream-creation chat**. Review one with independent lenses.

```text
Create an agent team to review the reading-view implementation. Spawn three reviewers,
one owning each file set so they don't conflict:
- "craft": typography, motion (the document-reconcile signature moment), spring physics,
  restrained native depth — does it hit the "forward through craft, not ornament" bar (§5)?
- "calm": does it honor calm-by-default — no badges/counts/spinners, "what changed" at
  top, non-blocking? (§4, §9)
- "a11y-perf": accessibility via headless primitives, render performance, optimistic UI,
  zero-latency feel.
Have each report findings by severity. Synthesize across all three.
```

Swap the surface and lenses for the quiet-desk home or stream-creation chat.

---

## Playbook 4 — Competing-hypotheses bug hunt

For a confusing bug (e.g. "the background refresh blocks the window" — a direct §9
violation), competing hypotheses converge faster than one investigator.

```text
The window freezes briefly when a stream refreshes, violating the non-blocking invariant.
Spawn 4 agent teammates to investigate different hypotheses (frontend main-thread work,
the Tauri command bridge blocking, Rust async runtime contention, reconciliation running
on the UI path). Have them talk to each other and try to disprove each other's theories
like a scientific debate. Update docs/debug/refresh-block.md with the surviving consensus.
```

---

## Playbook 5 — Cross-layer feature (frontend / native / tests)

Once code exists, a feature spanning layers maps naturally to one teammate per layer.

```text
Create an agent team to implement the "refresh now" command end-to-end. Three teammates,
strict file ownership so no overwrites:
- "frontend": command-palette entry + optimistic "something moved" UI in the React SPA.
- "native": the Rust command + event that triggers a single-stream refresh, non-blocking.
- "tests": integration coverage for the bridge contract and the non-blocking guarantee.
Require plan approval before any teammate writes code; only approve plans that keep the
window non-blocking and include test coverage. Use Sonnet.
```

---

## Freshet-specific guardrails for any team

- **One owner per file.** The React SPA + Rust crate boundary makes this easy — assign
  teammates by layer (frontend / native / tests / docs).
- **Put the invariants in the spawn prompt.** Teammates don't inherit chat history; they
  *do* read `CLAUDE.md`. Keep the §9 invariants in root `CLAUDE.md` so every teammate sees
  them, and restate the task-relevant ones in the spawn prompt.
- **Plan approval for anything touching the vault, scheduler, or non-blocking path** —
  these are invariants, not preferences. Steer the lead: *"reject plans that block the
  window"*, *"reject plans that write anything but plain markdown to the vault."*
- **Design/research teams > implementation teams while greenfield.** Most current value is
  in §10's open decisions — no code to conflict over.
- **Sonnet for teammates; clean up when done.** (See [costs](./agent-teams-reference.md#10-token-cost).)

---

## Quick reference card

| I want to… | Reach for |
| :--- | :--- |
| Explore an open design decision (§10) | **Team** — Playbook 1, one teammate per decision |
| Compare data sources / libraries | **Team** — Playbook 2, one teammate per option |
| Review a signature surface thoroughly | **Team** — Playbook 3, independent lenses |
| Chase a confusing bug | **Team** — Playbook 4, competing hypotheses |
| Build a cross-layer feature | **Team** — Playbook 5, one owner per layer + plan approval |
| Isolate verbose output (tests, logs, docs fetch) | **Subagent** (cheaper, reports back) |
| Quick targeted edit, tight iteration | **Solo** (main conversation) |
