# Agent Teams — Master Reference

> Coordinate multiple Claude Code instances working together as a team, with a shared
> task list, inter-agent messaging, and centralized management.
> Experimental; requires Claude Code **v2.1.32+** (this machine: v2.1.176).

One session is the **team lead** (coordinates, assigns, synthesizes). **Teammates**
work independently, each in its own context window, and communicate directly with each
other. Unlike subagents (which only report back to the main agent), you can also talk
to any teammate directly.

---

## 1. Enable

Disabled by default. Set the env var in `settings.json` (already done for Freshet in
`.claude/settings.local.json`):

```json
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  }
}
```

Or export `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` in the shell. Read at startup —
restart the session after changing it.

---

## 2. When to use them

Strongest use cases (parallel exploration adds **real** value):

- **Research & review** — teammates investigate different aspects, then share/challenge findings.
- **New modules/features** — each teammate owns a separate piece, no stepping on each other.
- **Debugging with competing hypotheses** — teammates test rival theories in parallel and converge.
- **Cross-layer coordination** — frontend / backend / tests, one owner each.

Avoid for: sequential tasks, same-file edits, work with many dependencies → use a single
session or subagents. Teams add coordination overhead and use **significantly** more tokens.

### Teams vs. subagents

| | Subagents | Agent teams |
| :--- | :--- | :--- |
| Context | Own window; results return to caller | Own window; fully independent |
| Communication | Report to main agent only | Teammates message each other directly |
| Coordination | Main agent manages all work | Shared task list, self-coordination |
| Best for | Focused tasks where only the result matters | Complex work needing discussion |
| Token cost | Lower | Higher (each teammate is a separate Claude) |

---

## 3. Architecture

| Component | Role |
| :--- | :--- |
| **Team lead** | Main session that creates the team, spawns teammates, coordinates work |
| **Teammates** | Separate Claude Code instances, each working assigned tasks |
| **Task list** | Shared list of work items teammates claim and complete |
| **Mailbox** | Messaging system for agent-to-agent communication |

Task dependencies are managed automatically: completing a task unblocks tasks that
depended on it, no manual intervention.

**Stored locally (auto-generated, ephemeral — gone on cleanup or session end):**

- Team config: `~/.claude/teams/{team-name}/config.json`
- Task list: `~/.claude/tasks/{team-name}/`

The team config holds runtime state (session IDs, tmux pane IDs) — **do not hand-edit or
pre-author it**; it's overwritten on the next state update. Its `members` array (name,
agent ID, agent type) lets teammates discover each other. There is **no** project-level
team config; a `.claude/teams/teams.json` is treated as an ordinary file, not config.

---

## 4. Starting a team

Describe the task and team structure in natural language; the lead creates the team,
spawns teammates, and coordinates. Example that works (three independent roles):

```text
I'm designing a CLI tool that helps developers track TODO comments across
their codebase. Create an agent team to explore this from different angles: one
teammate on UX, one on technical architecture, one playing devil's advocate.
```

Two ways teams start, both with your approval:

- **You request a team** — give a parallel-friendly task and explicitly ask for a team.
- **Claude proposes a team** — it may suggest one; you confirm before it proceeds.

### Specify teammates and models

The lead picks a count, or you state it:

```text
Create a team with 4 teammates to refactor these modules in parallel.
Use Sonnet for each teammate.
```

Teammates **don't** inherit the lead's `/model`. Set **Default teammate model** in
`/config` (pick **Default (leader's model)** to follow the lead). Prefer **Sonnet** for
teammates — balances capability and cost for coordination work.

---

## 5. Display modes

| Mode | Behavior | Requirements |
| :--- | :--- | :--- |
| **In-process** | All teammates run in your main terminal | Any terminal, no setup |
| **Split panes** | Each teammate gets its own pane | tmux **or** iTerm2 |

Default is `"auto"`: split panes if you're already in tmux or iTerm2, else in-process.

```json
// ~/.claude/settings.json
{ "teammateMode": "in-process" }   // or "tmux" or "auto"
```

```bash
claude --teammate-mode in-process   # force for one session
```

- **In-process navigation:** `Shift+Down` cycles teammates (wraps back to lead); type to
  message the selected teammate; `Enter` views a session, `Esc` interrupts its turn;
  `Ctrl+T` toggles the task list.
- **Split panes:** click into a pane to interact. Needs tmux or iTerm2 + the
  [`it2` CLI](https://github.com/mkusaka/it2) (enable iTerm2 → Settings → General → Magic
  → Python API). `tmux -CC` in iTerm2 is the suggested entrypoint; tmux works best on macOS.
- **Not supported for split panes:** VS Code integrated terminal, Windows Terminal, Ghostty.
  Use in-process there.

---

## 6. Controlling the team

### Talk to teammates directly

Each teammate is a full, independent session. Message any one to add instructions, ask
follow-ups, or redirect. (In-process: `Shift+Down` then type. Split: click the pane.)

### Plan approval for risky work

Make a teammate plan before implementing — it stays in read-only plan mode until the lead
approves:

```text
Spawn an architect teammate to refactor the authentication module.
Require plan approval before they make any changes.
```

The teammate sends a plan-approval request; the lead approves or rejects with feedback
(rejected → teammate revises and resubmits). The lead decides **autonomously** — steer it
with criteria in your prompt: *"only approve plans that include test coverage"*,
*"reject plans that modify the database schema."*

### Assign and claim tasks

Shared task list; states: **pending → in progress → completed**. Tasks can depend on other
tasks (a pending task with unresolved deps can't be claimed until they complete).

- **Lead assigns** — tell the lead which task goes to which teammate.
- **Self-claim** — after finishing, a teammate picks up the next unassigned, unblocked task.

Claiming uses **file locking** to prevent races when teammates grab the same task.

### Naming teammates

The lead names each teammate at spawn; any teammate can message any other by name. For
predictable names you can reference later, tell the lead what to call each one.

### Shut down a teammate

```text
Ask the researcher teammate to shut down
```

The lead sends a shutdown request; the teammate approves (exits gracefully) or rejects
with an explanation.

### Clean up the team

```text
Clean up the team
```

Removes shared team resources. **Cleanup fails if any teammate is still running** — shut
them down first. Claude often cleans up on its own, so a later request may report nothing
to clean up. **Always clean up via the lead** — teammates may not resolve team context
correctly, leaving resources inconsistent.

---

## 7. Context & communication

Each teammate has its own context window. On spawn a teammate loads the same project
context as a regular session — **CLAUDE.md, MCP servers, skills** — plus the lead's spawn
prompt. **The lead's conversation history does NOT carry over.**

How information is shared:

- **Automatic message delivery** — messages are delivered to recipients automatically; the
  lead doesn't poll.
- **Idle notifications** — when a teammate finishes and stops, it notifies the lead.
- **Shared task list** — all agents see task status and claim available work.
- **Teammate messaging** — message one teammate by name; to reach everyone, send one
  message per recipient.

### Permissions

Teammates start with the **lead's** permission settings (incl.
`--dangerously-skip-permissions`). You can change an individual teammate's mode **after**
spawning, but **cannot** set per-teammate modes at spawn time. Teammate permission requests
bubble up to the lead — pre-approve common ops in your permission settings to cut friction.

---

## 8. Reusable teammate roles (subagent definitions)

When spawning a teammate, reference a [subagent](./subagents-reference.md) type from any
scope (project, user, plugin, CLI):

```text
Spawn a teammate using the security-reviewer agent type to audit the auth module.
```

The teammate honors that definition's **`tools` allowlist** and **`model`**, and the
definition's body is **appended** to the teammate's system prompt (not replacing it). Team
coordination tools (`SendMessage`, task management) are **always** available even when
`tools` restricts other tools.

> ⚠️ The `skills` and `mcpServers` frontmatter fields are **not** applied when a definition
> runs as a teammate. Teammates load skills/MCP from project + user settings like a normal
> session.

Define a role once → reuse it as both a delegated subagent and a team teammate.

---

## 9. Quality gates with hooks

Hooks enforce rules at team lifecycle points. Exit code **2** blocks the action and sends
feedback. (Full input schemas in [`subagents-reference.md` §Hooks](./subagents-reference.md#hooks).)

| Hook | Fires when | Exit 2 effect |
| :--- | :--- | :--- |
| [`TeammateIdle`](https://code.claude.com/docs/en/hooks#teammateidle) | A teammate is about to go idle | Keeps the teammate working (stderr → user) |
| [`TaskCreated`](https://code.claude.com/docs/en/hooks#taskcreated) | A task is being created | Rolls back creation (stderr → Claude) |
| [`TaskCompleted`](https://code.claude.com/docs/en/hooks#taskcompleted) | A task is being marked complete | Prevents completion (stderr → Claude) |

Example — block "done" while tests fail (`TaskCompleted`), JSON form:

```json
{ "decision": "block", "reason": "Task completion criteria not met; tests are still failing" }
```

Example — keep a teammate working past idle (`TeammateIdle`), exit-code form:

```bash
#!/bin/bash
if [ some_condition ]; then
  echo "Keep this teammate working" >&2
  exit 2
fi
exit 0
```

Use `{"continue": false, "stopReason": "..."}` to halt the **whole team**.

---

## 10. Token cost

Teams use **significantly** more tokens than a single session — roughly **~7× a standard
session when teammates run in plan mode**, since each teammate is a separate Claude with
its own context window. Cost scales ~linearly with active teammates and how long they run.

Keep it manageable:

- **Use Sonnet** for teammates.
- **Keep teams small** — usage is ~proportional to team size.
- **Focused spawn prompts** — everything in the prompt is in their context from turn one.
- **Clean up when done** — idle teammates still consume tokens.

For routine work, a single session is more cost-effective. See
[`/costs`](https://code.claude.com/docs/en/costs#agent-team-token-costs).

---

## 11. Best practices

- **Give enough context in the spawn prompt** — teammates don't inherit chat history.
  Include task-specifics:

  ```text
  Spawn a security reviewer teammate with the prompt: "Review the authentication module
  at src/auth/ for security vulnerabilities. Focus on token handling, session
  management, and input validation. The app uses JWT tokens stored in httpOnly
  cookies. Report any issues with severity ratings."
  ```

- **Team size: start 3–5.** Token cost scales linearly; coordination overhead rises;
  diminishing returns past a point. ~5–6 tasks per teammate keeps everyone productive.
  15 independent tasks → 3 teammates is a good start. Three focused beat five scattered.
- **Size tasks right** — self-contained units with a clear deliverable (a function, a test
  file, a review). Too small → overhead exceeds benefit; too large → long runs without
  check-ins risk wasted effort. If the lead isn't making enough tasks, tell it to split.
- **Wait for teammates** — if the lead starts implementing instead of delegating:
  `Wait for your teammates to complete their tasks before proceeding`.
- **Start with research & review** — clear boundaries, no merge conflicts, shows the value
  before you tackle parallel implementation.
- **Avoid file conflicts** — one owner per file/file-set.
- **Monitor & steer** — check progress, redirect, synthesize as findings arrive. Don't let
  a team run unattended too long.

---

## 12. Worked examples

### Parallel code review

```text
Create an agent team to review PR #142. Spawn three reviewers:
- One focused on security implications
- One checking performance impact
- One validating test coverage
Have them each review and report findings.
```

Each applies a distinct lens; the lead synthesizes across all three.

### Competing-hypotheses debugging

```text
Users report the app exits after one message instead of staying connected.
Spawn 5 agent teammates to investigate different hypotheses. Have them talk to
each other to try to disprove each other's theories, like a scientific
debate. Update the findings doc with whatever consensus emerges.
```

The adversarial debate beats sequential investigation (which anchors on the first theory).

---

## 13. Troubleshooting

| Symptom | Fix |
| :--- | :--- |
| **Teammates not appearing** | In-process: press `Shift+Down` to cycle. Check the task was complex enough to warrant a team. For split panes: `which tmux`; for iTerm2 verify `it2` + Python API. |
| **Too many permission prompts** | Pre-approve common ops in permission settings before spawning. |
| **Teammate stopped on an error** | View its output (`Shift+Down` / click pane), give instructions, or spawn a replacement. |
| **Lead shuts down too early** | Tell it to keep going / wait for teammates before proceeding. |
| **Orphaned tmux session** | `tmux ls` then `tmux kill-session -t <name>`. |

---

## 14. Limitations (experimental)

- **No session resumption with in-process teammates** — `/resume` and `/rewind` don't
  restore them; the lead may message teammates that no longer exist → tell it to spawn new ones.
- **Task status can lag** — teammates sometimes don't mark tasks complete, blocking
  dependents. Update status manually or nudge the teammate.
- **Shutdown can be slow** — teammates finish the current request/tool call first.
- **One team at a time** — a lead manages one team; clean up before creating another.
- **No nested teams** — teammates can't spawn their own teams/teammates; only the lead manages.
- **Lead is fixed** — the creating session is lead for the team's lifetime; no promotion/transfer.
- **Permissions set at spawn** — all start with the lead's mode; change individually after, not at spawn.
- **Split panes require tmux or iTerm2** — in-process works everywhere.

> **Tip:** `CLAUDE.md` works normally — teammates read it from their working directory.
> Use it to give all teammates project-specific guidance (see Freshet's root `CLAUDE.md`).
