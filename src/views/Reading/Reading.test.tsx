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

test("refresh is wired to the bridge and the notes editor is present", async () => {
  // CodeMirror can't be reliably driven via userEvent under jsdom, so the
  // notes save round-trip is covered by parseDoc + the Rust save_notes test;
  // here we assert the refresh wiring and that the editor renders.
  const bridge = new MockBridge();
  const refreshSpy = vi.spyOn(bridge, "refreshStream");
  render(<BridgeProvider bridge={bridge}><Reading streamId="ai-agents" onBack={()=>{}} /></BridgeProvider>);
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: /refresh/i }));
  expect(refreshSpy).toHaveBeenCalledWith("ai-agents");
  expect(screen.getByRole("textbox", { name: /my notes/i })).toBeInTheDocument();
});
