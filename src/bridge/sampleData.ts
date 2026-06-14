import type { StreamSummary, StreamDescription } from "./types";

export const SAMPLE_DOC = `# AI Agents
_updated 2 days ago · 4 sources_

## What changed
- Anthropic shipped the Claude Agent SDK v2 — durable workflows now survive restarts mid-task. [^hn1]
- Sentiment is turning against ReAct-style loops for production. [^r1]

## Current understanding
### Durable execution
Surviving restarts mid-task is the live frontier. [^gh1]
### Tool calling
Largely standardized; the open fight is the protocol (MCP vs. bespoke). [^hn1]

## Open questions
- Does MCP become the default tool protocol, or fragment? [^pm1]

[^hn1]: hackernews · Claude Agent SDK v2 · 412 · 2026-06-11 · https://news.ycombinator.com/item?id=1
[^r1]: reddit · Off ReAct loops in prod · 280 · 2026-06-10 · https://reddit.com/r/ml/1
[^gh1]: github · anthropics/agent-sdk v2.0 · 1200 · 2026-06-09 · https://github.com/x
[^pm1]: polymarket · MCP default by EOY · 61 · 2026-06-08 · https://polymarket.com/x

## My notes
- Watching the MCP-vs-bespoke fight — revisit before Q3 planning.
`;

export const sampleStreams: StreamSummary[] = [
  { id: "ai-agents", title: "AI Agents", lastCheckedAt: "2026-06-11T10:00:00.000Z", changedSinceSeen: true },
  { id: "rust-async", title: "Rust Async", lastCheckedAt: "2026-06-10T08:00:00.000Z", changedSinceSeen: false },
  { id: "local-llms", title: "Local LLMs", lastCheckedAt: "2026-06-09T14:00:00.000Z", changedSinceSeen: false },
];

export const sampleDescriptions: Record<string, StreamDescription> = {
  "ai-agents": {
    id: "ai-agents",
    title: "AI Agents",
    topic: "AI agents, agentic frameworks, and tool-calling protocols",
    sources: ["hackernews", "reddit", "github", "polymarket"],
    cadence: { mode: "interval", intervalMinutes: 360 },
    status: "active",
    createdAt: "2026-06-01T00:00:00.000Z",
  },
  "rust-async": {
    id: "rust-async",
    title: "Rust Async",
    topic: "Rust async/await ecosystem, tokio, async-std, and runtime news",
    sources: ["reddit", "github"],
    cadence: { mode: "interval", intervalMinutes: 720 },
    status: "active",
    createdAt: "2026-06-02T00:00:00.000Z",
  },
  "local-llms": {
    id: "local-llms",
    title: "Local LLMs",
    topic: "Running LLMs locally: llama.cpp, Ollama, hardware requirements, benchmarks",
    sources: ["hackernews", "reddit"],
    cadence: { mode: "on_launch" },
    status: "active",
    createdAt: "2026-06-03T00:00:00.000Z",
  },
};

function minimalDoc(title: string): string {
  return `# ${title}
_updated recently · 2 sources_

## What changed
- No significant changes since last check.

## Current understanding
### Overview
Tracking the latest developments in this space.

## Open questions
- What are the next major milestones?

## My notes
- Initial watch — nothing to flag yet.
`;
}

export function sampleDocFor(id: string): string {
  if (id === "ai-agents") return SAMPLE_DOC;
  const desc = sampleDescriptions[id];
  return minimalDoc(desc?.title ?? id);
}
