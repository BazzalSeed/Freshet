import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { BridgeProvider } from "../../bridge/BridgeProvider";
import { MockBridge } from "../../bridge/MockBridge";
import { Reading } from "./Reading";

test("default clean column; toggles reveal Outline and Sources", async () => {
  render(<BridgeProvider bridge={new MockBridge()}><Reading streamId="ai-agents" onBack={()=>{}} /></BridgeProvider>);
  expect(await screen.findByText("AI Agents")).toBeInTheDocument();
  expect(screen.queryByText("Outline")).not.toBeInTheDocument();        // rail collapsed (toggle uses aria-label, not text)
  expect(screen.queryByText(/^Sources/)).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: /toggle outline/i }));
  expect(await screen.findByText("Outline")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: /toggle sources/i }));
  expect(await screen.findByText(/^Sources/)).toBeInTheDocument();
});

test("refresh and notes are wired to the bridge", async () => {
  const bridge = new MockBridge();
  const refreshSpy = vi.spyOn(bridge, "refreshStream");
  const notesSpy = vi.spyOn(bridge, "saveNotes");
  render(<BridgeProvider bridge={bridge}><Reading streamId="ai-agents" onBack={()=>{}} /></BridgeProvider>);
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: /refresh/i }));
  expect(refreshSpy).toHaveBeenCalledWith("ai-agents");
  const notes = screen.getByLabelText("My notes");
  await userEvent.clear(notes); await userEvent.type(notes, "new note"); await userEvent.tab();
  expect(notesSpy).toHaveBeenCalled();
  expect(notesSpy.mock.calls[0][1]).toMatch(/^## My notes/);
});
