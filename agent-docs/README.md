# Agent Docs — Master Reference for Building Agent Teams

This folder is the reference Claude (and humans) consult before spinning up **agent
teams** or **subagents** on Freshet. It distills the official Claude Code docs into a
working playbook, plus Freshet-specific recipes.

> Source of truth: <https://code.claude.com/docs/en/agent-teams>,
> [`/sub-agents`](https://code.claude.com/docs/en/sub-agents),
> [`/hooks`](https://code.claude.com/docs/en/hooks),
> [`/costs`](https://code.claude.com/docs/en/costs).
> Captured 2026-06-12 against Claude Code **v2.1.176**. Agent teams require **v2.1.32+**.

## Files

| File | What's in it |
| :--- | :--- |
| [`agent-teams-reference.md`](./agent-teams-reference.md) | **The master guide.** Architecture, enabling, display modes, spawning, task list, messaging, plan approval, hooks, limitations, token costs. |
| [`subagents-reference.md`](./subagents-reference.md) | Subagents companion: frontmatter fields, scopes, tool restrictions, models, hooks, forks. Reusable as teammate roles. |
| [`freshet-playbooks.md`](./freshet-playbooks.md) | Copy-paste team prompts tuned to Freshet's actual surfaces (reading view, quiet-desk, stream-creation chat, research fan-out, Tauri bridge). |

## 30-second decision: team, subagent, or solo?

```
Is the side-work independent and parallelizable?
│
├─ No  ────────────────────────────────────────────► Solo (main conversation)
│        sequential, same-file edits, tight back-and-forth
│
└─ Yes
   │
   ├─ Do the workers need to talk to EACH OTHER,
   │  challenge findings, or self-coordinate a shared task list?
   │     │
   │     ├─ Yes ──────────────────────────────────► AGENT TEAM
   │     │         research debates, cross-layer features,
   │     │         competing-hypotheses debugging
   │     │
   │     └─ No, they just report a result back ───► SUBAGENTS
   │               isolate verbose output, parallel research,
   │               focused one-shot tasks (cheaper)
```

| | Subagents | Agent teams |
| :--- | :--- | :--- |
| Context | Own window; result returns to caller | Own window; fully independent |
| Communication | Report to main agent only | Teammates message each other directly |
| Coordination | Main agent manages all work | Shared task list, self-coordination |
| Best for | Focused tasks where only the result matters | Work needing discussion & collaboration |
| Token cost | Lower (result summarized back) | **Higher** (~7× when teammates run plan mode) |

## Status on this machine

- ✅ Enabled — `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` is set in
  `.claude/settings.local.json` (takes effect on session start).
- Claude Code v2.1.176 (≥ 2.1.32 required). Restart the session if teams aren't available.

## The five rules that matter most

1. **Start with research/review**, not parallel code-writing. Clear boundaries, no merge conflicts.
2. **3–5 teammates**, 5–6 tasks each. Three focused beat five scattered.
3. **One owner per file** — two teammates editing the same file overwrite each other.
4. **Spawn prompts carry everything** — teammates inherit CLAUDE.md/MCP/skills but **not** the lead's chat history.
5. **The lead cleans up** — never a teammate. Shut teammates down, then `Clean up the team`.
