/**
 * Tests for the first-run onboarding flow.
 *
 * Uses MockBridge configured for not-onboarded states so the full step
 * sequence can be exercised without touching localStorage or Tauri.
 */
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";
import App from "../../App";
import { BridgeProvider } from "../../bridge/BridgeProvider";
import { MockBridge } from "../../bridge/MockBridge";
import { Onboarding } from "./Onboarding";

beforeEach(() => localStorage.clear());

/* ── helpers ─────────────────────────────────────────────────────────────── */

function renderWithBridge(bridge: MockBridge) {
  return render(
    <BridgeProvider bridge={bridge}>
      <Onboarding initialAgent={undefined} onDone={() => {}} />
    </BridgeProvider>,
  );
}

/* ── 1. Existing tests are unaffected (default onboarded:true) ───────────── */

describe("App — default (onboarded)", () => {
  test("renders Desk directly when onboarded=true", async () => {
    render(<App />);
    expect(await screen.findByText("AI Agents")).toBeInTheDocument();
  });
});

/* ── 2. Not-onboarded flow ───────────────────────────────────────────────── */

describe("Onboarding — not onboarded, agent found", () => {
  function makeNotOnboardedBridge() {
    const bridge = new MockBridge({
      onboardingState: {
        onboarded: false,
        hasRoot: false,
        agent: { kind: "claude_code", available: true, version: "2.1.0" },
      },
    });
    // Spy on key methods
    vi.spyOn(bridge, "setRootFolder");
    vi.spyOn(bridge, "completeOnboarding");
    vi.spyOn(bridge, "recheckAgents");
    return bridge;
  }

  test("App renders Onboarding when not onboarded", async () => {
    const bridge = makeNotOnboardedBridge();
    render(<App bridge={bridge} />);
    // Welcome step text should appear instead of the Desk
    expect(
      await screen.findByText(
        /Freshet turns topics into living documents/i,
      ),
    ).toBeInTheDocument();
    // Desk content should NOT appear
    expect(screen.queryByText("AI Agents")).not.toBeInTheDocument();
  });

  test("Welcome step renders one CTA button", async () => {
    const bridge = makeNotOnboardedBridge();
    renderWithBridge(bridge);
    expect(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    ).toBeInTheDocument();
  });

  test("Advancing to folder step shows path input", async () => {
    const bridge = makeNotOnboardedBridge();
    renderWithBridge(bridge);
    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    expect(
      await screen.findByLabelText("Output folder path"),
    ).toBeInTheDocument();
  });

  test("Entering a path and continuing calls setRootFolder", async () => {
    const bridge = makeNotOnboardedBridge();
    renderWithBridge(bridge);

    // Advance to folder step
    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );

    // Type a path in the text input
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.clear(input);
    await userEvent.type(input, "/home/user/Freshet");

    // Click Continue
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    // setRootFolder must have been called with the entered path
    await waitFor(() => {
      expect(bridge.setRootFolder).toHaveBeenCalledWith("/home/user/Freshet");
    });
  });

  test("Agent step shows found-agent confirmation", async () => {
    const bridge = makeNotOnboardedBridge();
    renderWithBridge(bridge);

    // Welcome → folder → agent
    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    // Agent step: should show "Found Claude Code ✓"
    expect(await screen.findByText(/Found Claude Code/i)).toBeInTheDocument();
    // Version string should appear
    expect(await screen.findByText("2.1.0")).toBeInTheDocument();
  });

  test("Clicking Continue on agent step calls completeOnboarding", async () => {
    const bridge = makeNotOnboardedBridge();
    renderWithBridge(bridge);

    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    await screen.findByText(/Found Claude Code/i);
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    await waitFor(() => {
      expect(bridge.completeOnboarding).toHaveBeenCalledOnce();
    });
  });

  test("Full onboarding flow ends by rendering the main app (desk element)", async () => {
    const bridge = makeNotOnboardedBridge();
    // After completeOnboarding, the onDone callback switches gate to "app".
    render(<App bridge={bridge} />);

    // Walk through the flow
    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));
    await screen.findByText(/Found Claude Code/i);
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    // After done, the main Desk should render
    await waitFor(() => {
      // The Desk has a "New stream" button and lists streams
      expect(screen.getByRole("button", { name: /new stream/i })).toBeInTheDocument();
    });
  });
});

/* ── 3. Not-onboarded flow — no agent found ──────────────────────────────── */

describe("Onboarding — not onboarded, no agent", () => {
  function makeNoneFoundBridge() {
    const bridge = new MockBridge({
      onboardingState: {
        onboarded: false,
        hasRoot: false,
        agent: null,
      },
    });
    vi.spyOn(bridge, "recheckAgents");
    vi.spyOn(bridge, "completeOnboarding");
    return bridge;
  }

  test("agent step shows install / re-check state (not an error)", async () => {
    const bridge = makeNoneFoundBridge();
    renderWithBridge(bridge);

    // Advance through Welcome → Folder → Agent
    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet-none");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    // Should show the not-found guidance — not a red error message
    expect(
      await screen.findByText(/Freshet runs on your own local agent/i),
    ).toBeInTheDocument();
    // Install links are present
    expect(screen.getAllByRole("link", { name: /claude code/i }).length).toBeGreaterThan(0);
    // Re-check button is present
    expect(screen.getByRole("button", { name: /re-check/i })).toBeInTheDocument();
  });

  test("Re-check button calls recheckAgents", async () => {
    const bridge = makeNoneFoundBridge();
    renderWithBridge(bridge);

    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet-none");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    await screen.findByRole("button", { name: /re-check/i });
    await userEvent.click(screen.getByRole("button", { name: /re-check/i }));

    await waitFor(() => {
      expect(bridge.recheckAgents).toHaveBeenCalledOnce();
    });
  });

  test("'Continue without an agent' still calls completeOnboarding", async () => {
    const bridge = makeNoneFoundBridge();
    renderWithBridge(bridge);

    await userEvent.click(
      await screen.findByRole("button", { name: /choose where freshet writes/i }),
    );
    const input = await screen.findByLabelText("Output folder path");
    await userEvent.type(input, "/tmp/freshet-none");
    await userEvent.click(screen.getByRole("button", { name: /^continue$/i }));

    await screen.findByText(/Freshet runs on your own local agent/i);
    await userEvent.click(
      screen.getByRole("button", { name: /continue without an agent/i }),
    );

    await waitFor(() => {
      expect(bridge.completeOnboarding).toHaveBeenCalledOnce();
    });
  });
});

/* ── 4. MockBridge test helper ───────────────────────────────────────────── */

describe("MockBridge.__setOnboarding", () => {
  test("allows flipping onboarding state after construction", async () => {
    const bridge = new MockBridge(); // default: onboarded=true
    const state1 = await bridge.getOnboardingState();
    expect(state1.onboarded).toBe(true);

    bridge.__setOnboarding({ onboarded: false, hasRoot: false, agent: null });
    const state2 = await bridge.getOnboardingState();
    expect(state2.onboarded).toBe(false);
  });
});
