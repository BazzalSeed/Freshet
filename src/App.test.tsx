import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach } from "vitest";
import App from "./App";

beforeEach(() => localStorage.clear());

test("renders Desk by default (a seeded stream title shows)", async () => {
  render(<App />);
  // The Desk lists seeded streams — at least one title appears
  expect(await screen.findByText("AI Agents")).toBeInTheDocument();
});

test("clicking a stream row navigates to Reading (title / My notes appears)", async () => {
  render(<App />);
  await screen.findByText("AI Agents");
  // Click the row button (exact title match to avoid "Refresh AI Agents")
  await userEvent.click(screen.getByRole("button", { name: "AI Agents" }));
  // Reading view should load — Chrome shows the title, Document shows My notes section
  expect(await screen.findByText("AI Agents")).toBeInTheDocument();
  // My notes textarea is present in the Reading view
  expect(await screen.findByLabelText("My notes")).toBeInTheDocument();
});

test("a back control in Reading returns to Desk", async () => {
  render(<App />);
  await screen.findByText("AI Agents");
  await userEvent.click(screen.getByRole("button", { name: "AI Agents" }));
  await screen.findByLabelText("My notes");
  // The Chrome back button
  await userEvent.click(screen.getByRole("button", { name: /back/i }));
  // We're back on Desk
  expect(await screen.findByRole("button", { name: /new stream/i })).toBeInTheDocument();
});

test("theme toggle button is present", async () => {
  render(<App />);
  await screen.findByText("AI Agents");
  expect(screen.getByRole("button", { name: /toggle theme/i })).toBeInTheDocument();
});

test("New stream navigates to Create and Cancel returns to Desk", async () => {
  render(<App />);
  await screen.findByRole("button", { name: /new stream/i });
  await userEvent.click(screen.getByRole("button", { name: /new stream/i }));
  // Create view shows topic input
  expect(screen.getByLabelText("Topic")).toBeInTheDocument();
  // Cancel returns to Desk
  await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
  expect(await screen.findByRole("button", { name: /new stream/i })).toBeInTheDocument();
});
