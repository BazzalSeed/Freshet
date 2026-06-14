import { render } from "@testing-library/react";
import { BridgeProvider, useBridge } from "./BridgeProvider";
import { MockBridge } from "./MockBridge";

beforeEach(() => localStorage.clear());

function StreamCountProbe() {
  const bridge = useBridge();
  const [count, setCount] = React.useState<number | null>(null);
  React.useEffect(() => {
    bridge.listStreams().then(streams => setCount(streams.length));
  }, [bridge]);
  if (count === null) return <span>loading</span>;
  return <span>{count} streams</span>;
}

// React needs to be in scope for JSX
import React from "react";

test("BridgeProvider exposes bridge; probe sees stream count > 0", async () => {
  const { findByText } = render(
    <BridgeProvider bridge={new MockBridge()}>
      <StreamCountProbe />
    </BridgeProvider>
  );
  // findByText waits for the async state update
  const el = await findByText(/\d+ streams/);
  const count = parseInt(el.textContent ?? "0", 10);
  expect(count).toBeGreaterThan(0);
});

test("useBridge throws when used outside provider", () => {
  // Suppress React's error boundary console output
  const spy = vi.spyOn(console, "error").mockImplementation(() => {});
  expect(() => render(<StreamCountProbe />)).toThrow();
  spy.mockRestore();
});

test("BridgeProvider auto-selects MockBridge in jsdom (no Tauri global)", () => {
  // jsdom defines `window` but neither `__TAURI_INTERNALS__` nor `__TAURI__`,
  // so the provider should fall back to the mock.
  expect("__TAURI_INTERNALS__" in window).toBe(false);
  expect("__TAURI__" in window).toBe(false);

  let captured: unknown = null;
  function CaptureProbe() {
    captured = useBridge();
    return null;
  }
  render(
    <BridgeProvider>
      <CaptureProbe />
    </BridgeProvider>
  );
  expect(captured).toBeInstanceOf(MockBridge);
});
