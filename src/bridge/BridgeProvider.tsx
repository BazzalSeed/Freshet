import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { Bridge } from "./Bridge";
import { MockBridge } from "./MockBridge";
import { TauriBridge } from "./TauriBridge";

const BridgeContext = createContext<Bridge | null>(null);

/**
 * True when running inside the Tauri native webview. Tauri injects
 * `__TAURI_INTERNALS__` (and historically `__TAURI__`) onto `window`; jsdom and
 * a plain browser have neither, so tests/dev fall back to `MockBridge`.
 */
function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

/** Pick the real backend in the native window, the mock everywhere else. */
function defaultBridge(): Bridge {
  return isTauri() ? new TauriBridge() : new MockBridge();
}

interface BridgeProviderProps {
  bridge?: Bridge;
  children: ReactNode;
}

export function BridgeProvider({ bridge, children }: BridgeProviderProps) {
  const resolved = useMemo(() => bridge ?? defaultBridge(), [bridge]);
  return (
    <BridgeContext.Provider value={resolved}>
      {children}
    </BridgeContext.Provider>
  );
}

export function useBridge(): Bridge {
  const ctx = useContext(BridgeContext);
  if (ctx === null) {
    throw new Error("useBridge must be used inside a <BridgeProvider>");
  }
  return ctx;
}
