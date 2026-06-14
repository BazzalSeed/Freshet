import { useState } from "react";
import { BridgeProvider } from "./bridge/BridgeProvider";
import { Desk } from "./views/Desk/Desk";
import { Reading } from "./views/Reading/Reading";
import { Create } from "./views/Create/Create";
import { useTheme } from "./theme/useTheme";
import type { StreamSummary } from "./bridge/types";
import "./App.css";

type View =
  | { kind: "desk" }
  | { kind: "reading"; id: string }
  | { kind: "create" };

function ThemeToggle() {
  const { theme, toggle } = useTheme();
  return (
    <button
      className="app-theme-toggle"
      aria-label="Toggle theme"
      onClick={toggle}
      type="button"
      title={theme === "dark" ? "Switch to light" : "Switch to dark"}
    >
      {theme === "dark" ? "☀" : "☾"}
    </button>
  );
}

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
      <ThemeToggle />

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

export default function App() {
  return (
    <BridgeProvider>
      <AppShell />
    </BridgeProvider>
  );
}
