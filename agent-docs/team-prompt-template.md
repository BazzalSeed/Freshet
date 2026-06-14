# Agent-Team Prompt Template

A reusable, fill-in-the-blank prompt for spinning up an **agent team** (the experimental
`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` feature: a lead spawns teammates that message each
other and share a task list). Keep this file in the vault and copy a section each time.

> Ground truth: [`agent-teams-reference.md`](./agent-teams-reference.md) ·
> [`freshet-playbooks.md`](./freshet-playbooks.md). This file just makes the spawn prompt
> repeatable.

---

## 1. Findings — does a dedicated skill already exist?

**No.** As of 2026-06-12 there is **no installed skill or plugin — and none in the official
Claude Code marketplace set surfaced here — dedicated to prompting/orchestrating an agent
TEAM** (the lead-spawns-teammates, shared-task-list, inter-agent-messaging feature).

What *does* exist, and why none of it counts:

| Skill | Scope | Agent **team**? |
| :--- | :--- | :--- |
| `superpowers:dispatching-parallel-agents` | "2+ independent tasks without shared state or sequential dependencies" — dispatches parallel **subagents** that report results back to the caller | **No.** This is the subagent case explicitly. Subagents have their own window but **never message each other** and there is **no shared task list**. Different feature from agent teams. |
| `superpowers:subagent-driven-development` | Executes plan tasks via **subagents** in the current session | **No** — subagents. |
| `superpowers:brainstorming`, `writing-plans`, `executing-plans` | Single-session planning/execution flows | **No** — solo. |
| `deep-research` | Fan-out web search harness (internal, not the teams feature) | **No.** |

> ⚠️ **`superpowers:dispatching-parallel-agents` is for SUBAGENTS, not teams.** Subagents
> report a summary back to the one caller and cannot talk to each other or self-coordinate a
> shared task list. Agent teams are the opposite: independent teammates that message each
> other and claim from a shared list. Don't reach for that skill expecting team behavior.

**Conclusion:** the agent-teams feature is driven by a **natural-language spawn prompt**, not a
skill. This template is that prompt, encoding the best practices from the reference.

---

## 2. The template

**Just use plain language.** The spine is one header line plus one numbered line per role:

> `Create a team of [N] using [model] to …` → `1. [Role] → owns [path/] + produces [artifact]`

That single role line carries the two rules that stop teams colliding: **one owner per area**
(`owns path/`) and **a concrete deliverable** (`produces artifact`). Aim for **3–5** roles.

### Quick version (trivial / low-risk teams)

```text
Create a team of <N> using Sonnet to <ONE-SENTENCE GOAL>.

1. <Role> → owns <path/ or area> + produces <artifact>
2. <Role> → owns <path/ or area> + produces <artifact>
3. <Role> → owns <path/ or area> + produces <artifact>
```

### Full version (anything non-trivial)

Add the four things the one-liner can't carry: standalone **context** per role (teammates
inherit `CLAUDE.md`/MCP/skills but **not** our chat history), the **seams** where roles
interact, optional **plan approval**, and the **wait → synthesize → clean up** close.

```text
Create a team of <N> using Sonnet to <ONE-SENTENCE GOAL>.

1. <Role> → owns <path/> + produces <artifact>
   Context (no chat history carries over): <facts, paths, constraints, invariants>
2. <Role> → owns <path/> + produces <artifact>
   Context: <...>
3. <Role> → owns <path/> + produces <artifact>
   Context: <...>

Seams: <who messages whom where work overlaps — e.g. 1 ↔ 2 on the shared interface>.
<For debugging: have them try to disprove each other's theories like a scientific debate.>

<IF RISKY — touches the vault, scheduler, non-blocking path, schema, or any code:>
IMPORTANT: require plan approval before any teammate makes changes. Approve only plans that
<CRITERIA — e.g. keep the window non-blocking AND include tests>; reject plans that
<RED LINES — e.g. block the window / write anything but plain markdown to the vault>.

Wait for all teammates to finish before you proceed — don't start implementing yourself.
Then synthesize into <ONE deliverable — an overview doc / a ranked list / a merged review>
and clean up the team (shut teammates down, then "Clean up the team").
```

Why each piece is there:
- **`Create a team of N using Sonnet …`** triggers the lead to build the team and pins the
  model — teammates don't inherit your `/model`.
- **`owns path/`** — one owner per area; two teammates on one file overwrite each other.
- **`produces artifact`** — a concrete deliverable, so "done" is unambiguous.
- **Context line** — teammates don't see this chat; restate the paths/constraints they need.
- **Plan approval** — only for risky paths; the lead approves autonomously, so the *criteria* steer it.
- **`Wait … before you proceed`** — stops the lead doing the work solo.
- **Cleanup** — always via the lead, never a teammate.

---

## 3. Filled-in examples

### Example A — Architecture design jam (greenfield, no code yet)

```text
Create a team of 4 using Sonnet to design Freshet's core architecture from docs/product-vision.md.

1. data-model      → owns docs/design/data-model.md      + produces the vault/state schema + file layout
2. scheduler       → owns docs/design/scheduler.md        + produces the per-stream cadence engine design
3. research-engine → owns docs/design/research-engine.md  + produces the fan-out + reconcile design
4. bridge          → owns docs/design/bridge.md           + produces the Tauri↔Rust bridge contract sketch

Context for all: honor vision §9 invariants — stateful, plain-markdown out, don't rebuild
the vault, non-blocking window, BYO-LLM (local agent → local model → API key).
Seams: scheduler ↔ research-engine; data-model ↔ bridge.
Wait for all teammates, then synthesize docs/design/overview.md and clean up the team.
```

### Example B — Three-lens review of a signature surface (reading view)

```text
Create a team of 3 using Sonnet to review the reading-view implementation (review only, no code changes).

1. craft     → owns <reading-view component files>   + produces findings by severity
2. calm      → owns <reading-view state/data files>  + produces findings by severity
3. a11y-perf → owns <reading-view styles/primitives> + produces findings by severity

Lenses — craft: typography, motion (the reconcile signature moment), restrained native depth
(vision §5). calm: no badges/counts/spinners, "what changed" on top, non-blocking (§4, §9).
a11y-perf: headless-primitive accessibility, render perf, optimistic/zero-latency feel.
Seams: flag any overlap so two lenses don't double-count it.
Wait for all three, then synthesize one prioritized review and clean up the team.
```

(Swap surface + lenses for the quiet-desk home or stream-creation chat.)

### Example C — Cross-layer feature with plan approval (once code exists)

```text
Create a team of 3 using Sonnet to implement the "refresh now" command end-to-end.

1. frontend → owns the React command-palette + optimistic "something moved" UI + produces the UI + wiring
2. native   → owns the Rust refresh command + event (non-blocking)              + produces the command + event
3. tests    → owns the integration tests                                        + produces bridge-contract + non-blocking coverage

IMPORTANT: require plan approval before any teammate writes code. Approve only plans that
keep the window non-blocking AND include tests; reject plans that block the window or write
anything but plain markdown to the vault.
Wait for all teammates, then synthesize a short integration summary and clean up the team.
```

---

## 4. Quick checklist (eyeball before hitting Enter)

- [ ] **3–5 roles**, each one line: `Role → owns path/ + produces artifact` (three focused beat five scattered).
- [ ] **`using Sonnet`** in the header (teammates don't inherit the lead's `/model`).
- [ ] **`owns path/` is unique** per role — no two roles touch the same file.
- [ ] **`produces <artifact>`** is concrete for each role, so "done" is unambiguous.
- [ ] **Context restated** for any non-trivial role — paths, constraints, invariants (no chat
      history carries over; only `CLAUDE.md`/MCP/skills do).
- [ ] **Seams named** — who messages whom where decisions interact.
- [ ] **Plan approval + criteria** if the work touches the vault, scheduler, non-blocking
      path, schema, or any code; red lines spelled out.
- [ ] **`Wait for teammates before proceeding`** included so the lead doesn't go solo.
- [ ] **Deliverable + synthesis step** named (one merged output, not N scattered ones).
- [ ] **Cleanup planned** — shut teammates down, then "Clean up the team" via the lead.
- [ ] **Is a team even right?** Independent + parallel + needs discussion → team. Just reports
      back → subagents (cheaper). Sequential/same-file → solo.
```
