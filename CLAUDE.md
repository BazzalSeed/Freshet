# CLAUDE.md — Freshet

Freshet is a **local-first desktop app** (Tauri 2 shell · React + Vite SPA frontend · Rust
native backend) that turns topics into self-updating **knowledge streams** synthesized into
living markdown documents. It's a **watch model**, not a chat model: it pushes, remembers,
and stays quiet when nothing changed. See `README.md` for the full vision/design brief.

> Status: greenfield — only `README.md` exists. The data model, scheduler, source set, and
> reconciliation are deliberately left to the builder (README §10).

## Invariants the build must honor (README §9 — non-negotiable)

- **Installable local app**, never a hosted webapp.
- **Push, not pull** — works between visits; the user never re-asks.
- **Stateful** — knows what it already said; dedups and detects change over time.
- **Quiet by default & non-blocking** — "nothing new" is valid; never manufacture novelty;
  refreshes run in the background and **never gate the UI**.
- **Significance over recency.**
- **Local-first & BYO-LLM** — runs on the user's machine + keys (local agent → local model
  → API key). No accounts, no server holding user data.
- **Don't rebuild the vault** — emit plain markdown into a folder; don't become a note manager.
- **Topics, not accounts.** **Calm over engagement** — never optimize for time-in-app.

## Design language (README §5)

Forward through **craft, not ornament**: speed (zero latency, optimistic UI, no spinners),
motion (spring physics; the signature moment is a **document reconciling**), typography as
hero (the document *is* the product), restrained native macOS depth, keyboard-first.
Own design language — no stock component kit; Radix/Ark headless primitives for plumbing.

## Agent teams & subagents — read before orchestrating

Before spinning up an agent team or subagents, **consult [`agent-docs/`](./agent-docs/)**:

- [`agent-docs/README.md`](./agent-docs/README.md) — decision matrix (team vs. subagent vs. solo) + the 5 rules.
- [`agent-docs/agent-teams-reference.md`](./agent-docs/agent-teams-reference.md) — the master guide.
- [`agent-docs/subagents-reference.md`](./agent-docs/subagents-reference.md) — subagent definitions, hooks, forks.
- [`agent-docs/freshet-playbooks.md`](./agent-docs/freshet-playbooks.md) — copy-paste team prompts tuned to Freshet.

Agent teams are **enabled** (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` in
`.claude/settings.local.json`). Defaults: **Sonnet** for teammates, **3–5** teammates,
**one owner per file**, **plan approval** for anything touching the vault / scheduler /
non-blocking path, and **the lead cleans up**. While greenfield, prefer design/research
teams over parallel-implementation teams.
