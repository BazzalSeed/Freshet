/**
 * AgentStatusChip — persistent header chip showing agent auth state.
 */
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, vi } from "vitest";
import { BridgeProvider } from "../bridge/BridgeProvider";
import { MockBridge } from "../bridge/MockBridge";
import { AgentStatusChip } from "./AgentStatusChip";

beforeEach(() => localStorage.clear());

function renderChip(bridge: MockBridge) {
  return render(
    <BridgeProvider bridge={bridge}>
      <AgentStatusChip />
    </BridgeProvider>
  );
}

test("ok state: shows check mark and agent name/version", async () => {
  const bridge = new MockBridge({ agentState: "ok" });
  renderChip(bridge);
  // The chip renders "Claude Code mock" (name + version from MOCK_AGENT)
  const chip = await screen.findByRole("button", { name: /agent status/i });
  expect(chip).toBeInTheDocument();
  expect(chip.textContent).toMatch(/claude code/i);
  // Icon in ok state
  expect(chip.textContent).toContain("✓");
});

test("not_logged_in state: shows warning icon and 'Not logged in'", async () => {
  // When the agent exists but available=false, the chip shows "Not logged in".
  const bridge = new MockBridge({
    onboardingState: { onboarded: true, hasRoot: true, agent: { kind: "claude_code", available: false } },
  });
  renderChip(bridge);
  const chip = await screen.findByRole("button", { name: /agent status/i });
  await waitFor(() => {
    expect(chip.textContent).toMatch(/not logged in/i);
  });
  expect(chip.textContent).toContain("⚠");
});

test("no_agent state: shows warning icon and 'No agent'", async () => {
  const bridge = new MockBridge({ agentState: "none" });
  renderChip(bridge);
  const chip = await screen.findByRole("button", { name: /agent status/i });
  await waitFor(() => {
    expect(chip.textContent).toMatch(/no agent/i);
  });
  expect(chip.textContent).toContain("⚠");
});

test("clicking the chip opens a panel", async () => {
  const bridge = new MockBridge({ agentState: "ok" });
  renderChip(bridge);
  const chip = await screen.findByRole("button", { name: /agent status/i });
  await waitFor(() => expect(chip).not.toBeDisabled());
  await userEvent.click(chip);
  expect(screen.getByRole("dialog", { name: /agent status/i })).toBeInTheDocument();
});

test("panel for not_logged_in shows login guidance and Re-check button", async () => {
  const bridge = new MockBridge({
    onboardingState: { onboarded: true, hasRoot: true, agent: { kind: "claude_code", available: false } },
  });
  renderChip(bridge);
  const chip = await screen.findByRole("button", { name: /agent status/i });
  // Wait until chip shows not logged in state
  await waitFor(() => expect(chip.textContent).toMatch(/not logged in/i));
  await userEvent.click(chip);
  // Panel should contain login guidance — check by role=dialog
  const panel = screen.getByRole("dialog", { name: /agent status/i });
  expect(panel).toBeInTheDocument();
  expect(panel).toHaveTextContent(/not logged in/i);
  expect(screen.getByRole("button", { name: /re-check/i })).toBeInTheDocument();
});

test("Re-check button calls bridge.recheckAgents", async () => {
  const bridge = new MockBridge({
    onboardingState: { onboarded: true, hasRoot: true, agent: { kind: "claude_code", available: false } },
  });
  const spy = vi.spyOn(bridge, "recheckAgents");
  renderChip(bridge);
  const chip = await screen.findByRole("button", { name: /agent status/i });
  await userEvent.click(chip);
  await userEvent.click(screen.getByRole("button", { name: /re-check/i }));
  expect(spy).toHaveBeenCalled();
});
