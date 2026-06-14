# Freshet

A **local-first desktop app** that turns the few topics you care about into **knowledge
streams** — standing subscriptions that quietly keep themselves current and synthesize what
they find into a living document you actually want to read.

The shift: from a **chat model** (you pull, it forgets, it always produces something) to a
**watch model** (it pushes, it remembers, it only tells you what changed). It has the
discipline to stay quiet when nothing real happened.

> **Status:** early / greenfield. No app yet — we're designing and building
> [Freshet v1](docs/superpowers/specs/2026-06-12-freshet-v1-design.md), a functional
> whole-product slice built by an agent team. Build/run instructions land with it.

## Docs

- **[Product vision, feel & design](docs/product-vision.md)** — what Freshet is, why it
  matters, how it should feel and look. Source of the §-numbered principles (`§9`), design
  language (`§5`), and scope (`§10`) referenced across the codebase. **Start here.**
- **[Specs](docs/superpowers/specs/)** — one design spec per sub-project. First up:
  [Freshet v1](docs/superpowers/specs/2026-06-12-freshet-v1-design.md).
- **[Frontend feel](docs/superpowers/specs/2026-06-13-frontend-feel-design.md)** — locked palette, type, and reading-view layout.
- **[Rust primer](docs/rust-primer.md)** — the Rust you'll actually hit in the Tauri backend, Freshet-tailored.
- **[agent-docs/](agent-docs/)** — how we orchestrate agent teams & subagents to build this.
- **[CLAUDE.md](CLAUDE.md)** — working guidance for Claude Code in this repo.

## Stack

- **Shell:** Tauri 2 — native window, OS webview, small binaries.
- **Frontend:** React + Vite SPA, own design language (headless Radix/Ark primitives, no stock kit).
- **Core:** Rust — research fan-out, reconciliation, file IO, local-LLM calls.
- **LLM:** bring-your-own — detected local agent (Claude Code / Codex) → local model → API key.

## License

TBD — open-source vs. closed is an open decision (see product vision `§11`, `§13`).
