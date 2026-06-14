/**
 * AgentNotice — renders actionable guidance for FreshetErrors.
 */
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { AgentNotice } from "./AgentNotice";
import type { FreshetError } from "../bridge/types";

function notice(error: FreshetError, onRecheck?: () => void, onRetry?: () => void) {
  return render(
    <AgentNotice error={error} onRecheck={onRecheck} onRetry={onRetry} />
  );
}

test("not_logged_in: renders login guidance", () => {
  notice({
    code: "not_logged_in",
    message: "The agent is not logged in.",
    hint: "Open your terminal, run `claude` then `/login`, then re-check.",
  });
  // Title span contains the title text
  expect(screen.getByText(/agent not logged in/i)).toBeInTheDocument();
  // Step guidance for re-auth — the list items
  expect(screen.getAllByText(/\/login/i).length).toBeGreaterThan(0);
});

test("not_logged_in: shows Re-check button and calls onRecheck", async () => {
  const onRecheck = vi.fn();
  notice(
    { code: "not_logged_in", message: "Not logged in.", hint: "Run /login" },
    onRecheck
  );
  const btn = screen.getByRole("button", { name: /re-check/i });
  expect(btn).toBeInTheDocument();
  await userEvent.click(btn);
  expect(onRecheck).toHaveBeenCalledOnce();
});

test("no_agent: renders install guidance with links", () => {
  notice({ code: "no_agent", message: "No agent found.", hint: "Install Claude Code or Codex." });
  // Title element — use exact match on the span
  const titles = screen.getAllByText(/no agent found/i);
  expect(titles.length).toBeGreaterThan(0);
  expect(screen.getByRole("link", { name: /docs\.anthropic\.com/i })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /github\.com\/openai\/codex/i })).toBeInTheDocument();
});

test("no_sources: renders source guidance", () => {
  notice({ code: "no_sources", message: "No results from sources." });
  expect(screen.getByText("No source results")).toBeInTheDocument();
  expect(screen.getByText(/hacker news/i)).toBeInTheDocument();
});

test("agent_failed: renders generic error", () => {
  notice({ code: "agent_failed", message: "Something broke badly." });
  expect(screen.getByText("Agent error")).toBeInTheDocument();
  expect(screen.getByText(/Something broke badly/)).toBeInTheDocument();
});

test("hint text is rendered when provided", () => {
  notice({ code: "timeout", message: "Timed out.", hint: "Try again later." });
  expect(screen.getByText("Try again later.")).toBeInTheDocument();
});

test("no Re-check button for no_sources (onRecheck not applicable)", () => {
  const onRecheck = vi.fn();
  notice({ code: "no_sources", message: "No results." }, onRecheck);
  // no_sources doesn't show re-check; it should show retry if onRetry provided
  expect(screen.queryByRole("button", { name: /re-check/i })).not.toBeInTheDocument();
});

test("Try again button calls onRetry", async () => {
  const onRetry = vi.fn();
  notice({ code: "agent_failed", message: "failed." }, undefined, onRetry);
  await userEvent.click(screen.getByRole("button", { name: /try again/i }));
  expect(onRetry).toHaveBeenCalledOnce();
});
