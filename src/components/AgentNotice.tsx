/**
 * AgentNotice — a calm, actionable panel for agent errors.
 *
 * Rendered when an agent-using command returns a FreshetError. Never red or
 * alarming; uses the same structural design language as the rest of Freshet
 * (design tokens, typography as information, calm over engagement).
 */
import type { FreshetError } from "../bridge/types";
import "./AgentNotice.css";

export interface AgentNoticeProps {
  error: FreshetError;
  /** Called when the user clicks "Re-check" — should call bridge.recheckAgents() then retry */
  onRecheck?: () => void;
  /** Called when the user clicks "Try again" without a recheck */
  onRetry?: () => void;
}

function GuidanceSteps({ code }: { code: string }) {
  if (code === "not_logged_in") {
    return (
      <ol className="agent-notice-steps">
        <li>Open your terminal.</li>
        <li>Run <code className="agent-notice-code">claude</code> to open the Claude Code session.</li>
        <li>Inside that session, run <code className="agent-notice-code">/login</code> and follow the prompts.</li>
        <li>Once logged in, click <strong>Re-check</strong> below.</li>
      </ol>
    );
  }
  if (code === "no_agent") {
    return (
      <div className="agent-notice-links">
        <p className="agent-notice-step-text">Install an agent, then click <strong>Re-check</strong>:</p>
        <ul className="agent-notice-steps">
          <li>
            <strong>Claude Code</strong> —{" "}
            <a
              href="https://docs.anthropic.com/en/docs/claude-code/getting-started"
              target="_blank"
              rel="noopener noreferrer"
              className="agent-notice-link"
            >
              docs.anthropic.com/claude-code
            </a>
          </li>
          <li>
            <strong>Codex</strong> —{" "}
            <a
              href="https://github.com/openai/codex"
              target="_blank"
              rel="noopener noreferrer"
              className="agent-notice-link"
            >
              github.com/openai/codex
            </a>
          </li>
        </ul>
      </div>
    );
  }
  if (code === "no_sources") {
    return (
      <ul className="agent-notice-steps">
        <li>Try selecting a different source — some (e.g. Reddit) need OAuth setup and may return no results.</li>
        <li>Hacker News, GitHub, and Polymarket work without setup.</li>
      </ul>
    );
  }
  return null;
}

function errorTitle(code: string): string {
  switch (code) {
    case "not_logged_in": return "Agent not logged in";
    case "no_agent":      return "No agent found";
    case "timeout":       return "Agent timed out";
    case "no_sources":    return "No source results";
    default:              return "Agent error";
  }
}

export function AgentNotice({ error, onRecheck, onRetry }: AgentNoticeProps) {
  const title = errorTitle(error.code);
  const showRecheck = error.code === "not_logged_in" || error.code === "no_agent";
  const showRetry = !showRecheck || !!onRetry;

  return (
    <div className="agent-notice" role="status" aria-live="polite" data-code={error.code}>
      <div className="agent-notice-header">
        <span className="agent-notice-icon" aria-hidden="true">⚠</span>
        <span className="agent-notice-title">{title}</span>
      </div>

      <p className="agent-notice-message">{error.message}</p>

      {error.hint && (
        <p className="agent-notice-hint">{error.hint}</p>
      )}

      <GuidanceSteps code={error.code} />

      <div className="agent-notice-actions">
        {showRecheck && onRecheck && (
          <button
            className="agent-notice-btn agent-notice-btn-primary"
            type="button"
            onClick={onRecheck}
            aria-label="Re-check agent status"
          >
            Re-check
          </button>
        )}
        {showRetry && onRetry && (
          <button
            className="agent-notice-btn agent-notice-btn-secondary"
            type="button"
            onClick={onRetry}
            aria-label="Try again"
          >
            Try again
          </button>
        )}
      </div>
    </div>
  );
}
