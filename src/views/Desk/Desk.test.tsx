import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, beforeEach } from "vitest";
import { BridgeProvider } from "../../bridge/BridgeProvider";
import { MockBridge } from "../../bridge/MockBridge";
import { Desk } from "./Desk";

beforeEach(() => localStorage.clear());

function renderDesk(overrides?: { onOpen?: (id: string) => void; onNew?: () => void }) {
  const onOpen = overrides?.onOpen ?? vi.fn();
  const onNew = overrides?.onNew ?? vi.fn();
  const bridge = new MockBridge();
  render(
    <BridgeProvider bridge={bridge}>
      <Desk onOpen={onOpen} onNew={onNew} />
    </BridgeProvider>
  );
  return { onOpen, onNew, bridge };
}

test("renders seeded stream titles", async () => {
  renderDesk();
  expect(await screen.findByText("AI Agents")).toBeInTheDocument();
  expect(await screen.findByText("Rust Async")).toBeInTheDocument();
  expect(await screen.findByText("Local LLMs")).toBeInTheDocument();
});

test("at least one row has a data-moved element (ai-agents is seeded changed)", async () => {
  renderDesk();
  await screen.findByText("AI Agents");
  const moved = document.querySelector("[data-moved]");
  expect(moved).not.toBeNull();
});

test("clicking a stream row calls onOpen with the stream id", async () => {
  const onOpen = vi.fn();
  renderDesk({ onOpen });
  // Use exact string match so the "Refresh AI Agents" button doesn't also match
  const row = await screen.findByRole("button", { name: "AI Agents" });
  await userEvent.click(row);
  expect(onOpen).toHaveBeenCalledWith("ai-agents");
});

test("clicking refresh calls bridge.refreshStream with the id", async () => {
  const bridge = new MockBridge();
  const refreshSpy = vi.spyOn(bridge, "refreshStream");
  const onOpen = vi.fn();
  const onNew = vi.fn();
  render(
    <BridgeProvider bridge={bridge}>
      <Desk onOpen={onOpen} onNew={onNew} />
    </BridgeProvider>
  );
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: /Refresh AI Agents/i }));
  expect(refreshSpy).toHaveBeenCalledWith("ai-agents");
});

test("list is still present immediately after clicking refresh (no gating spinner)", async () => {
  const bridge = new MockBridge();
  // Make refreshStream hang so we can check while it's in-flight
  vi.spyOn(bridge, "refreshStream").mockReturnValue(new Promise(() => {}));
  const onOpen = vi.fn();
  const onNew = vi.fn();
  render(
    <BridgeProvider bridge={bridge}>
      <Desk onOpen={onOpen} onNew={onNew} />
    </BridgeProvider>
  );
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: /Refresh AI Agents/i }));
  // List must still be visible — not gated by a spinner
  expect(screen.getByText("AI Agents")).toBeInTheDocument();
  expect(screen.getByText("Rust Async")).toBeInTheDocument();
});

test("clicking New stream calls onNew", async () => {
  const onNew = vi.fn();
  renderDesk({ onNew });
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: /new stream/i }));
  expect(onNew).toHaveBeenCalled();
});
