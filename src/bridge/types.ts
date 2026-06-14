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

// ── Typed agent errors (mirrors Rust FreshetError) ───────────────────────────

/**
 * Structured error returned by agent-using bridge commands.
 * Tauri serializes the Rust `Err(FreshetError)` variant as this object,
 * which the JS `catch` handler receives directly.
 *
 * Codes:
 *   not_logged_in  — agent is not authenticated; show re-auth steps
 *   no_agent       — no agent binary detected; show install guidance
 *   timeout        — agent invocation timed out
 *   no_sources     — all source providers returned 0 items
 *   agent_failed   — any other agent error (message has detail)
 */
export interface FreshetError {
  code: "not_logged_in" | "no_agent" | "timeout" | "no_sources" | "agent_failed" | string;
  message: string;
  hint?: string;
}

/**
 * Parse whatever `invoke` throws into a `FreshetError`.
 *
 * Tauri delivers the `Err(FreshetError)` from Rust as a plain JS object with
 * `code`/`message`/`hint` fields. A plain `string` or `Error` thrown by the
 * bridge (e.g. pre-typed commands) falls back to `{ code: "agent_failed" }`.
 */
export function asFreshetError(e: unknown): FreshetError {
  if (e != null && typeof e === "object" && "code" in e && "message" in e) {
    const obj = e as Record<string, unknown>;
    if (typeof obj.code === "string" && typeof obj.message === "string") {
      return {
        code: obj.code,
        message: obj.message,
        hint: typeof obj.hint === "string" ? obj.hint : undefined,
      };
    }
  }
  if (typeof e === "string") {
    return { code: "agent_failed", message: e };
  }
  if (e instanceof Error) {
    return { code: "agent_failed", message: e.message };
  }
  return { code: "agent_failed", message: "An unknown error occurred." };
}
