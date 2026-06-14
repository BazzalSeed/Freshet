export type CadenceMode = "manual" | "on_launch" | "interval";
export interface Cadence { mode: CadenceMode; intervalMinutes?: number }
export type StreamStatus = "active" | "paused" | "retired";
export interface StreamSummary { id: string; title: string; lastCheckedAt?: string; changedSinceSeen: boolean }
export interface StreamDescription { id: string; title: string; topic: string; sources: string[]; cadence: Cadence; status: StreamStatus; createdAt: string }
export interface GetStreamResult { description: StreamDescription; documentMarkdown: string; lastCheckedAt?: string }
export interface Summary { changed: boolean; nNew: number }
export interface DraftInput { topic: string; sources: string[]; cadence: Cadence }
export interface DraftResult { draftMarkdown: string; proposedDescription: StreamDescription }
export type RefreshPhase = "detecting" | "researching" | "synthesizing" | "done" | "error";
export interface RefreshProgress { streamId: string; phase: RefreshPhase }
export const FREE_SOURCES = ["reddit", "hackernews", "github", "polymarket"] as const;
// Reddit needs OAuth for reliable access (HTTP 403 on anonymous requests — tracked).
// Keep it in FREE_SOURCES so it remains selectable, but exclude it from defaults.
export const DEFAULT_SOURCES = ["hackernews", "github", "polymarket"] as const;

// ── Onboarding / config / agents ─────────────────────────────────────────────
export type AgentKind = "claude_code" | "codex";
export interface AgentStatus { kind: AgentKind; available: boolean; version?: string; path?: string }
export interface OnboardingState { onboarded: boolean; hasRoot: boolean; agent?: AgentStatus }
export interface AppConfig { root?: string; selectedAgent?: AgentKind; onboarded: boolean }
