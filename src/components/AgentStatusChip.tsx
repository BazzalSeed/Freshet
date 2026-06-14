/**
 * AgentStatusChip — a persistent header chip showing agent auth state.
 *
 * On mount, calls bridge.listAgents() and shows:
 *   ✓ <agent> <version>  — agent available and (presumably) authenticated
 *   ⚠ Not logged in      — agent found but auth state uncertain
 *   ⚠ No agent           — no agent binary found
 *
 * Clicking opens a small panel with status + re-auth guidance + a Re-check button.
 * Calm, muted styling; uses design tokens.
 */
import { useEffect, useRef, useState } from "react";
import { useBridge } from "../bridge/BridgeProvider";
import type { AgentStatus } from "../bridge/types";
import { AgentNotice } from "./AgentNotice";
import "./AgentStatusChip.css";

type ChipState =
  | { kind: "loading" }
  | { kind: "ok"; agent: AgentStatus }
  | { kind: "not_logged_in"; agent?: AgentStatus }
  | { kind: "no_agent" };

export function AgentStatusChip() {
  const bridge = useBridge();
  const [chipState, setChipState] = useState<ChipState>({ kind: "loading" });
  const [panelOpen, setPanelOpen] = useState(false);
  const [rechecking, setRechecking] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const detect = async () => {
    try {
      const agents = await bridge.listAgents();
      if (agents.length === 0) {
        setChipState({ kind: "no_agent" });
      } else {
        const first = agents[0];
        // If the agent is available we optimistically assume it's logged in —
        // auth failures only surface when a command is actually run.
        if (first.available) {
          setChipState({ kind: "ok", agent: first });
        } else {
          setChipState({ kind: "not_logged_in", agent: first });
        }
      }
    } catch {
      setChipState({ kind: "no_agent" });
    }
  };

  useEffect(() => {
    detect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Close panel on outside click.
  useEffect(() => {
    if (!panelOpen) return;
    function handleClick(e: MouseEvent) {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        setPanelOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [panelOpen]);

  const handleRecheck = async () => {
    setRechecking(true);
    try {
      await bridge.recheckAgents();
      await detect();
    } finally {
      setRechecking(false);
      // Keep the panel open so the user can see the result.
    }
  };

  function chipLabel(): string {
    switch (chipState.kind) {
      case "loading": return "Detecting agent…";
      case "ok": {
        const { agent } = chipState;
        const name = agent.kind === "claude_code" ? "Claude Code" : "Codex";
        return agent.version ? `${name} ${agent.version}` : name;
      }
      case "not_logged_in": return "Not logged in";
      case "no_agent":      return "No agent";
    }
  }

  function chipMod(): string {
    switch (chipState.kind) {
      case "ok":      return "ok";
      case "loading": return "loading";
      default:        return "warn";
    }
  }

  function chipIcon(): string {
    switch (chipState.kind) {
      case "ok":      return "✓";
      case "loading": return "·";
      default:        return "⚠";
    }
  }

  function noticeError() {
    if (chipState.kind === "not_logged_in") {
      return {
        code: "not_logged_in" as const,
        message: "The agent is not logged in.",
        hint: "Open your terminal, run `claude` then `/login`, then re-check.",
      };
    }
    if (chipState.kind === "no_agent") {
      return {
        code: "no_agent" as const,
        message: "No agent detected or selected.",
        hint: "Install Claude Code or Codex, then re-check.",
      };
    }
    return null;
  }

  return (
    <div className="agent-chip-wrap">
      <button
        ref={buttonRef}
        className={`agent-chip agent-chip--${chipMod()}`}
        type="button"
        aria-label={`Agent status: ${chipLabel()}`}
        aria-expanded={panelOpen}
        onClick={() => {
          if (chipState.kind !== "loading") {
            setPanelOpen(v => !v);
          }
        }}
        disabled={chipState.kind === "loading"}
      >
        <span className="agent-chip-icon" aria-hidden="true">{chipIcon()}</span>
        <span className="agent-chip-label">{chipLabel()}</span>
      </button>

      {panelOpen && (
        <div ref={panelRef} className="agent-chip-panel" role="dialog" aria-label="Agent status">
          {chipState.kind === "ok" ? (
            <div className="agent-chip-panel-ok">
              <p className="agent-chip-panel-ok-text">
                <strong>{chipLabel()}</strong> is ready.
              </p>
              <button
                className="agent-chip-recheck-btn"
                type="button"
                onClick={handleRecheck}
                disabled={rechecking}
              >
                {rechecking ? "Checking…" : "Re-check"}
              </button>
            </div>
          ) : (
            <AgentNotice
              error={noticeError()!}
              onRecheck={handleRecheck}
            />
          )}
        </div>
      )}
    </div>
  );
}
