import { useState, useEffect } from "react";
import { BridgeProvider, useBridge } from "./bridge/BridgeProvider";
import { Desk } from "./views/Desk/Desk";
import { Reading } from "./views/Reading/Reading";
import { Create } from "./views/Create/Create";
import { Onboarding } from "./views/Onboarding/Onboarding";
import type { StreamSummary, AgentStatus } from "./bridge/types";
import type { Bridge } from "./bridge/Bridge";
import "./App.css";

type View =
  | { kind: "desk" }
  | { kind: "reading"; id: string }
  | { kind: "create" };

function AppShell() {
  const [view, setView] = useState<View>({ kind: "desk" });

  function handleOpen(id: string) {
    setView({ kind: "reading", id });
  }

  function handleNew() {
    setView({ kind: "create" });
  }

  function handleCreated(_summary: StreamSummary) {
    setView({ kind: "desk" });
  }

  function handleBack() {
    setView({ kind: "desk" });
  }

  return (
    <div className="app">
      {view.kind === "desk" && (
        <Desk onOpen={handleOpen} onNew={handleNew} />
      )}
      {view.kind === "reading" && (
        <Reading streamId={view.id} onBack={handleBack} />
      )}
      {view.kind === "create" && (
        <Create onCreated={handleCreated} onCancel={handleBack} />
      )}
    </div>
  );
}

/**
 * Gates the main app behind onboarding. On mount queries getOnboardingState();
 * if not yet onboarded renders <Onboarding>; otherwise goes straight to the
 * main shell. While loading shows a calm blank placeholder (no spinner gate).
 */
type GateState =
  | { phase: "loading" }
  | { phase: "onboarding"; agent?: AgentStatus | null }
  | { phase: "app" };

function GatedApp() {
  const bridge = useBridge();
  const [gate, setGate] = useState<GateState>({ phase: "loading" });

  useEffect(() => {
    bridge.getOnboardingState().then((state) => {
      if (state.onboarded) {
        setGate({ phase: "app" });
      } else {
        setGate({ phase: "onboarding", agent: state.agent ?? null });
      }
    });
  }, [bridge]);

  if (gate.phase === "loading") {
    return <div className="ob-loading" aria-hidden />;
  }

  if (gate.phase === "onboarding") {
    return (
      <Onboarding
        initialAgent={gate.agent}
        onDone={() => setGate({ phase: "app" })}
      />
    );
  }

  return <AppShell />;
}

interface AppProps {
  /** Optional bridge override — used in tests to inject a MockBridge. */
  bridge?: Bridge;
}

export default function App({ bridge }: AppProps = {}) {
  return (
    <BridgeProvider bridge={bridge}>
      <GatedApp />
    </BridgeProvider>
  );
}
