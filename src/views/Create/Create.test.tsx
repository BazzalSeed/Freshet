import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, beforeEach } from "vitest";
import { BridgeProvider } from "../../bridge/BridgeProvider";
import { MockBridge } from "../../bridge/MockBridge";
import { Create } from "./Create";

beforeEach(() => localStorage.clear());

function renderCreate(overrides?: {
  onCreated?: (s: import("../../bridge/types").StreamSummary) => void;
  onCancel?: () => void;
}) {
  const onCreated = overrides?.onCreated ?? vi.fn();
  const onCancel = overrides?.onCancel ?? vi.fn();
  const bridge = new MockBridge();
  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={onCreated} onCancel={onCancel} />
    </BridgeProvider>
  );
  return { onCreated, onCancel, bridge };
}

test("type topic, check hackernews, click Preview → preview area shows draft text", async () => {
  renderCreate();

  await userEvent.type(screen.getByLabelText("Topic"), "quantum computing");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  await userEvent.click(screen.getByRole("button", { name: /preview/i }));

  // Preview area must appear with draft markdown
  const preview = await screen.findByRole("region", { name: /preview/i });
  expect(preview).toBeInTheDocument();
  expect(preview.textContent).toMatch(/quantum computing/i);
});

test("Create button becomes enabled after successful Preview", async () => {
  renderCreate();

  await userEvent.type(screen.getByLabelText("Topic"), "quantum computing");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));

  // Create should be disabled before preview
  expect(screen.getByRole("button", { name: /^create$/i })).toBeDisabled();

  await userEvent.click(screen.getByRole("button", { name: /preview/i }));
  await screen.findByRole("region", { name: /preview/i });

  expect(screen.getByRole("button", { name: /^create$/i })).toBeEnabled();
});

test("clicking Create calls bridge.createStream then onCreated", async () => {
  const onCreated = vi.fn();
  const bridge = new MockBridge();
  const createSpy = vi.spyOn(bridge, "createStream");

  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={onCreated} onCancel={vi.fn()} />
    </BridgeProvider>
  );

  await userEvent.type(screen.getByLabelText("Topic"), "quantum computing");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  await userEvent.click(screen.getByRole("button", { name: /preview/i }));
  await screen.findByRole("region", { name: /preview/i });

  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

  await waitFor(() => {
    expect(createSpy).toHaveBeenCalled();
    expect(onCreated).toHaveBeenCalled();
  });
});

test("interval mode with no intervalMinutes blocks Preview (generateFirstDraft NOT called)", async () => {
  const bridge = new MockBridge();
  const draftSpy = vi.spyOn(bridge, "generateFirstDraft");

  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={vi.fn()} onCancel={vi.fn()} />
    </BridgeProvider>
  );

  await userEvent.type(screen.getByLabelText("Topic"), "some topic");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));

  // Switch cadence to interval
  await userEvent.selectOptions(screen.getByRole("combobox", { name: /cadence mode/i }), "interval");

  // intervalMinutes input should appear but is empty
  expect(screen.getByLabelText("Interval minutes")).toBeInTheDocument();

  // Preview should be disabled
  const previewBtn = screen.getByRole("button", { name: /preview/i });
  expect(previewBtn).toBeDisabled();

  // generateFirstDraft must NOT have been called
  expect(draftSpy).not.toHaveBeenCalled();
});

test("Cancel calls onCancel", async () => {
  const onCancel = vi.fn();
  renderCreate({ onCancel });
  await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
  expect(onCancel).toHaveBeenCalled();
});

test("Preview is disabled when topic is empty", async () => {
  renderCreate();
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  expect(screen.getByRole("button", { name: /preview/i })).toBeDisabled();
});

test("Preview is disabled when no sources are selected", async () => {
  renderCreate();
  await userEvent.type(screen.getByLabelText("Topic"), "some topic");
  // No sources checked
  expect(screen.getByRole("button", { name: /preview/i })).toBeDisabled();
});

// ── Agent error surfaces ───────────────────────────────────────────────────

test("agentState:not_logged_in — Preview shows AgentNotice with login guidance", async () => {
  const bridge = new MockBridge({ agentState: "not_logged_in" });
  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={vi.fn()} onCancel={vi.fn()} />
    </BridgeProvider>
  );

  await userEvent.type(screen.getByLabelText("Topic"), "ai agents");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  await userEvent.click(screen.getByRole("button", { name: /preview/i }));

  // AgentNotice must appear with login guidance (title text).
  expect(await screen.findByText(/agent not logged in/i)).toBeInTheDocument();
  // Must show the /login step — may match in title, message, or hint.
  expect(screen.getAllByText(/\/login/i).length).toBeGreaterThan(0);
  // Must show the Re-check button.
  expect(screen.getByRole("button", { name: /re-check/i })).toBeInTheDocument();
  // The preview draft area must NOT appear.
  expect(screen.queryByRole("region", { name: /preview/i })).not.toBeInTheDocument();
});

test("agentState:not_logged_in — clicking Re-check calls bridge.recheckAgents", async () => {
  const bridge = new MockBridge({ agentState: "not_logged_in" });
  const recheckSpy = vi.spyOn(bridge, "recheckAgents");

  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={vi.fn()} onCancel={vi.fn()} />
    </BridgeProvider>
  );

  await userEvent.type(screen.getByLabelText("Topic"), "ai agents");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  await userEvent.click(screen.getByRole("button", { name: /preview/i }));

  await screen.findByText(/agent not logged in/i);
  await userEvent.click(screen.getByRole("button", { name: /re-check/i }));

  expect(recheckSpy).toHaveBeenCalled();
});

test("agentState:none — Preview shows AgentNotice with no_agent guidance", async () => {
  const bridge = new MockBridge({ agentState: "none" });
  render(
    <BridgeProvider bridge={bridge}>
      <Create onCreated={vi.fn()} onCancel={vi.fn()} />
    </BridgeProvider>
  );

  await userEvent.type(screen.getByLabelText("Topic"), "ai agents");
  await userEvent.click(screen.getByRole("checkbox", { name: /hackernews/i }));
  await userEvent.click(screen.getByRole("button", { name: /preview/i }));

  expect(await screen.findByText(/no agent found/i)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /re-check/i })).toBeInTheDocument();
});
