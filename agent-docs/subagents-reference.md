# Subagents — Companion Reference

> Specialized AI assistants that handle specific tasks in their **own context window**,
> with a custom system prompt, tool access, and independent permissions. They report a
> summary back to the caller and **never talk to each other**.
> Use them to keep verbose work (search, logs, test output) out of the main conversation.

Subagents work within a single session. For sessions that communicate with each other, use
[agent teams](./agent-teams-reference.md). A subagent **definition** can be reused as a team
teammate (see [agent teams §8](./agent-teams-reference.md#8-reusable-teammate-roles-subagent-definitions)).

---

## Built-in subagents

| Agent | Model | Tools | Purpose |
| :--- | :--- | :--- | :--- |
| **Explore** | Haiku | Read-only (no Write/Edit) | Fast file discovery & codebase search. Specify thoroughness: *quick* / *medium* / *very thorough*. Skips CLAUDE.md + git status. |
| **Plan** | Inherits | Read-only | Research during plan mode. Skips CLAUDE.md + git status. |
| **general-purpose** | Inherits | All tools | Complex multi-step tasks needing exploration + action. |
| statusline-setup | Sonnet | — | `/statusline` config. |
| claude-code-guide | Haiku | — | Questions about Claude Code features. |

Block a built-in with `permissions.deny: ["Agent(Explore)"]`. Block **all** delegation by
denying the `Agent` tool.

---

## Defining a subagent

Markdown file with YAML frontmatter; the body is the system prompt:

```markdown
---
name: code-reviewer
description: Reviews code for quality and best practices
tools: Read, Glob, Grep
model: sonnet
---

You are a code reviewer. When invoked, analyze the code and provide
specific, actionable feedback on quality, security, and best practices.
```

> Files on disk load at **session start** — restart to pick up edits. Subagents created via
> `/agents` take effect immediately.

### Scopes (priority high → low)

| Location | Scope | How |
| :--- | :--- | :--- |
| Managed settings `.claude/agents/` | Org-wide | Deployed via managed settings |
| `--agents` CLI flag (JSON) | Current session | Pass JSON at launch (not saved) |
| `.claude/agents/` | Current project | Check into version control ✅ |
| `~/.claude/agents/` | All your projects | Personal |
| Plugin `agents/` dir | Where plugin enabled | Via plugins |

Identity comes from the `name` field, not the filename or subfolder. Keep names unique
across the tree. `/agents` opens a tabbed manager (Running + Library tabs).

### Frontmatter fields (only `name` + `description` required)

| Field | Notes |
| :--- | :--- |
| `name` | lowercase + hyphens; hooks receive it as `agent_type` |
| `description` | When Claude should delegate; add "use proactively" to encourage delegation |
| `tools` | Allowlist; inherits all if omitted |
| `disallowedTools` | Denylist; applied **before** `tools` |
| `model` | `sonnet`/`opus`/`haiku`/`fable`, full ID (`claude-opus-4-8`), or `inherit` (default) |
| `permissionMode` | `default`/`acceptEdits`/`auto`/`dontAsk`/`bypassPermissions`/`plan` |
| `maxTurns` | Max agentic turns before stopping |
| `skills` | Preload full skill content at startup (not applied when run as a teammate) |
| `mcpServers` | Inline def or reference by name (not applied when run as a teammate) |
| `hooks` | Lifecycle hooks scoped to this subagent |
| `memory` | `user`/`project`/`local` — persistent cross-session memory dir |
| `background` | `true` → always run as a background task |
| `effort` | `low`/`medium`/`high`/`xhigh`/`max` — overrides session effort |
| `isolation` | `worktree` → isolated git worktree copy, auto-cleaned if no changes |
| `color` | Display color in task list/transcript |
| `initialPrompt` | Auto-submitted first turn when run as main agent (`--agent`) |

> Plugin subagents ignore `hooks`, `mcpServers`, `permissionMode` for security.

### Tool restriction

- `tools` = allowlist; `disallowedTools` = denylist (applied first). A tool in both is removed.
- Not available to subagents even if listed: `AskUserQuestion`, `EnterPlanMode`,
  `ExitPlanMode` (unless `permissionMode: plan`), `ScheduleWakeup`, `WaitForMcpServers`.
- `Agent(worker, researcher)` in `tools` = allowlist of spawnable subagent types (main-thread
  `--agent` only). Omit `Agent` entirely → can't spawn any.

### Model resolution order

1. `CLAUDE_CODE_SUBAGENT_MODEL` env var
2. per-invocation `model` parameter
3. definition's `model` frontmatter
4. main conversation's model

---

## Invoking subagents

- **Natural language:** `Use the test-runner subagent to fix failing tests` — Claude decides.
- **@-mention:** `@"code-reviewer (agent)" look at the auth changes` — guarantees that subagent.
- **Whole session:** `claude --agent code-reviewer` — main thread takes the subagent's prompt,
  tools, model. Or set `"agent": "code-reviewer"` in `.claude/settings.json` (CLI flag wins).

### Foreground vs. background

- **Foreground** blocks the main conversation; permission prompts pass through.
- **Background** runs concurrently with already-granted permissions; **auto-denies** anything
  that would prompt. `Ctrl+B` backgrounds a running task. Disable all background tasks with
  `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1`.

### Forks (`/fork`)

A fork inherits the **entire conversation** instead of starting fresh — same system prompt,
tools, model, message history. Its tool calls stay out of your context; only the result
returns. Cheaper than a fresh subagent (shares the prompt cache). Enabled by default from
v2.1.161; toggle with `CLAUDE_CODE_FORK_SUBAGENT=1`/`0`. A fork can't spawn another fork.

```text
/fork draft unit tests for the parser changes so far
```

### Nested subagents (v2.1.172+)

A subagent can spawn its own subagents. Foreground: any depth (self-limiting). Background:
capped at depth 5 (a depth-5 background subagent gets no Agent tool).

---

## Hooks

Two ways to attach hooks: in the subagent's **frontmatter** (run while it's active) or in
**`settings.json`** (run in the main session on subagent lifecycle events).

### Team / subagent lifecycle events

| Event | Matcher | Can block? | Input fields (beyond common) |
| :--- | :--- | :--- | :--- |
| `SubagentStart` | agent type | No (observability) | `agent_id`, `agent_type`, `parent_agent_id` |
| `SubagentStop` | agent type | Yes (exit 2 → keep working) | + `stop_reason` |
| `TeammateIdle` | none | Yes (exit 2 → keep working) | `agent_id`, `agent_type` |
| `TaskCreated` | none | Yes (exit 2 → roll back) | `task_id`, `title`, `description`, `agent_id`, `agent_type` |
| `TaskCompleted` | none | Yes (exit 2 → block completion) | `task_id`, `title`, `status`, `agent_id`, `agent_type` |

Frontmatter `Stop` hooks become `SubagentStop` at runtime.

### General JSON output (all events)

| Field | Default | Meaning |
| :--- | :--- | :--- |
| `continue` | `true` | `false` → Claude stops entirely (overrides event-specific decisions) |
| `stopReason` | — | Shown to user when `continue: false` |
| `suppressOutput` | `false` | Hide stdout from transcript |
| `systemMessage` | — | Warning shown to user |
| `decision` | — | `"block"` (+ `reason`) for blockable events |
| `hookSpecificOutput.additionalContext` | — | Text injected into Claude's context |

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SubagentStop",
    "additionalContext": "Exploration incomplete. Some directories were not scanned."
  }
}
```

`settings.json` example — setup/cleanup around a specific subagent type:

```json
{
  "hooks": {
    "SubagentStart": [
      { "matcher": "db-agent",
        "hooks": [{ "type": "command", "command": "./scripts/setup-db-connection.sh" }] }
    ],
    "SubagentStop": [
      { "hooks": [{ "type": "command", "command": "./scripts/cleanup-db-connection.sh" }] }
    ]
  }
}
```

---

## Persistent memory

`memory: user|project|local` gives the subagent a directory that survives across sessions.
It's told to read/write there; the first 200 lines / 25KB of `MEMORY.md` is injected at
startup; Read/Write/Edit are auto-enabled.

| Scope | Location | Use when |
| :--- | :--- | :--- |
| `user` | `~/.claude/agent-memory/<name>/` | learnings apply across all projects |
| `project` | `.claude/agent-memory/<name>/` | project-specific, shareable (VCS) — **recommended default** |
| `local` | `.claude/agent-memory-local/<name>/` | project-specific, not checked in |

---

## Example definitions

A read-only reviewer (no Edit/Write):

```markdown
---
name: code-reviewer
description: Expert code review specialist. Proactively reviews code for quality, security, and maintainability. Use immediately after writing or modifying code.
tools: Read, Grep, Glob, Bash
model: inherit
---

You are a senior code reviewer ensuring high standards of code quality and security.

When invoked:
1. Run git diff to see recent changes
2. Focus on modified files
3. Begin review immediately

Provide feedback by priority: Critical (must fix) / Warnings (should fix) /
Suggestions (consider). Include specific fixes.
```

A debugger that can edit:

```markdown
---
name: debugger
description: Debugging specialist for errors, test failures, and unexpected behavior. Use proactively when encountering any issues.
tools: Read, Edit, Bash, Grep, Glob
---

You are an expert debugger specializing in root cause analysis. Capture the error,
isolate the failure, implement a minimal fix, and verify. Report root cause + evidence +
fix + testing approach. Fix the underlying issue, not the symptoms.
```

---

## Subagent vs. main conversation

**Main conversation** when: frequent back-and-forth, multiple phases share context, quick
targeted change, or latency matters. **Subagent** when: verbose output you don't need,
enforcing tool restrictions, or self-contained work that returns a summary. For a quick
question about existing context, use `/btw` (no tools, answer discarded).
