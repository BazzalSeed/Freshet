/**
 * First-run onboarding flow — 3 steps + done.
 *
 * Step 1 — Welcome: one sentence, one CTA.
 * Step 2 — Root folder: native picker (Tauri) or text input fallback (browser/tests).
 * Step 3 — Agent: resolved state (found | not found). Non-blocking.
 * Done  — calls completeOnboarding() then hands control back to the app.
 *
 * Bridge methods used:
 *   getOnboardingState()  listAgents()  recheckAgents()
 *   setRootFolder(path)   completeOnboarding()
 */

import { useState, useCallback } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import type { AgentStatus } from "../../bridge/types";
import "./Onboarding.css";

/* ── helpers ──────────────────────────────────────────────────────────────── */

/** True when running inside the Tauri native window. */
function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/** Open a native directory picker via tauri-plugin-dialog. */
async function pickFolderNative(): Promise<string | null> {
  try {
    // Dynamic import so the browser bundle never hard-fails on the missing module.
    const { open } = await import("@tauri-apps/plugin-dialog");
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") return result;
    return null;
  } catch {
    return null;
  }
}

/* ── Step 1: Welcome ──────────────────────────────────────────────────────── */

interface WelcomeProps {
  onNext: () => void;
}

function WelcomeStep({ onNext }: WelcomeProps) {
  return (
    <div className="ob-body">
      <p className="ob-description">
        Freshet turns topics into living documents that update themselves —
        quietly, in the background.
      </p>
      <div className="ob-actions">
        <button className="ob-btn-primary" onClick={onNext} type="button">
          Choose where Freshet writes
        </button>
      </div>
    </div>
  );
}

/* ── Step 2: Root folder ──────────────────────────────────────────────────── */

interface FolderStepProps {
  onNext: (path: string) => void;
}

function FolderStep({ onNext }: FolderStepProps) {
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const bridge = useBridge();

  const handleBrowse = useCallback(async () => {
    if (!isTauri()) return; // browser: use text input only
    setBusy(true);
    const picked = await pickFolderNative();
    setBusy(false);
    if (picked) setPath(picked);
  }, []);

  const handleContinue = useCallback(async () => {
    const trimmed = path.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      await bridge.setRootFolder(trimmed);
      onNext(trimmed);
    } finally {
      setBusy(false);
    }
  }, [path, bridge, onNext]);

  return (
    <div className="ob-body">
      <p className="ob-description">
        Freshet writes plain markdown into a folder you choose. It never
        touches anything else.
      </p>

      <div className="ob-folder-row">
        <input
          className="ob-folder-input"
          type="text"
          aria-label="Output folder path"
          placeholder="~/Documents/Freshet"
          value={path}
          onChange={(e) => setPath(e.target.value)}
        />
        {isTauri() && (
          <button
            className="ob-folder-browse"
            type="button"
            onClick={handleBrowse}
            disabled={busy}
            aria-label="Browse for folder"
          >
            Browse…
          </button>
        )}
      </div>

      <div className="ob-actions">
        <button
          className="ob-btn-primary"
          type="button"
          onClick={handleContinue}
          disabled={!path.trim() || busy}
        >
          Continue
        </button>
      </div>
    </div>
  );
}

/* ── Step 3: Agent ────────────────────────────────────────────────────────── */

interface AgentStepProps {
  agent: AgentStatus | null | undefined;
  onNext: () => void;
}

function AgentStep({ agent, onNext }: AgentStepProps) {
  const bridge = useBridge();
  const [currentAgent, setCurrentAgent] = useState<AgentStatus | null | undefined>(agent);
  const [rechecking, setRechecking] = useState(false);

  const handleRecheck = useCallback(async () => {
    setRechecking(true);
    try {
      const agents = await bridge.recheckAgents();
      const found = agents.find((a) => a.available) ?? null;
      setCurrentAgent(found);
    } finally {
      setRechecking(false);
    }
  }, [bridge]);

  const agentFound = currentAgent && currentAgent.available;

  return (
    <div className="ob-body">
      {agentFound ? (
        <>
          <p className="ob-agent-found">
            <strong>Found {currentAgent.kind === "claude_code" ? "Claude Code" : "Codex"}</strong>{" "}
            &#10003;{" "}
            {currentAgent.version ? (
              <span className="ob-agent-version">{currentAgent.version}</span>
            ) : null}{" "}
            — Freshet will use it.
          </p>
          <div className="ob-actions">
            <button className="ob-btn-primary" type="button" onClick={onNext}>
              Continue
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="ob-agent-notfound">
            Freshet runs on your own local agent. Install{" "}
            <a
              className="ob-install-link"
              href="https://claude.ai/download"
              target="_blank"
              rel="noreferrer"
            >
              Claude Code
            </a>{" "}
            or{" "}
            <a
              className="ob-install-link"
              href="https://github.com/openai/codex"
              target="_blank"
              rel="noreferrer"
            >
              Codex
            </a>
            , then re-check.
          </p>
          <div className="ob-install-links">
            <a
              className="ob-install-link"
              href="https://claude.ai/download"
              target="_blank"
              rel="noreferrer"
            >
              Install Claude Code →
            </a>
            <a
              className="ob-install-link"
              href="https://github.com/openai/codex"
              target="_blank"
              rel="noreferrer"
            >
              Install Codex →
            </a>
          </div>
          <div className="ob-actions">
            <button
              className="ob-btn-secondary"
              type="button"
              onClick={handleRecheck}
              disabled={rechecking}
            >
              {rechecking ? "Checking…" : "Re-check"}
            </button>
            <button className="ob-btn-primary" type="button" onClick={onNext}>
              Continue without an agent
            </button>
          </div>
        </>
      )}
    </div>
  );
}

/* ── Main Onboarding component ────────────────────────────────────────────── */

type OnboardingStep = "welcome" | "folder" | "agent" | "done";

interface OnboardingProps {
  /** Initial agent status from getOnboardingState() — may be undefined. */
  initialAgent?: AgentStatus | null;
  /** Called when onboarding completes (after completeOnboarding()). */
  onDone: () => void;
}

export function Onboarding({ initialAgent, onDone }: OnboardingProps) {
  const bridge = useBridge();
  const [step, setStep] = useState<OnboardingStep>("welcome");
  const [agent, setAgent] = useState<AgentStatus | null | undefined>(initialAgent);

  const handleFolderNext = useCallback(
    (_path: string) => {
      // After folder is set, fetch agents (they may have been discovered already
      // at launch; this just surfaces whatever the bridge knows).
      bridge.listAgents().then((agents) => {
        const found = agents.find((a) => a.available) ?? null;
        setAgent(found);
        setStep("agent");
      });
    },
    [bridge],
  );

  const handleAgentNext = useCallback(async () => {
    await bridge.completeOnboarding();
    setStep("done");
    onDone();
  }, [bridge, onDone]);

  const stepLabel: Record<OnboardingStep, string> = {
    welcome: "Welcome",
    folder: "Output folder",
    agent: "Local agent",
    done: "Done",
  };

  return (
    <div className="ob-shell">
      <div className="ob-card">
        <h1 className="ob-wordmark">freshet</h1>

        {step !== "done" && (
          <p className="ob-description" style={{ color: "var(--muted)", fontSize: "0.8125rem" }}>
            {stepLabel[step]}
          </p>
        )}

        {step === "welcome" && (
          <WelcomeStep onNext={() => setStep("folder")} />
        )}

        {step === "folder" && (
          <FolderStep onNext={handleFolderNext} />
        )}

        {step === "agent" && (
          <AgentStep agent={agent} onNext={handleAgentNext} />
        )}
      </div>
    </div>
  );
}
