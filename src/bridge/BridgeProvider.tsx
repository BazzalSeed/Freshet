import React, { createContext, useContext, useMemo } from "react";
import type { Bridge } from "./Bridge";
import { MockBridge } from "./MockBridge";

const BridgeContext = createContext<Bridge | null>(null);

interface BridgeProviderProps {
  bridge?: Bridge;
  children: React.ReactNode;
}

export function BridgeProvider({ bridge, children }: BridgeProviderProps) {
  const resolved = useMemo(() => bridge ?? new MockBridge(), [bridge]);
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
