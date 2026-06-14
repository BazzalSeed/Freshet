import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import type { OutlineNode } from "../../lib/parseDoc";
import { Outline } from "./Outline";

const nodes: OutlineNode[] = [
  { id: "sec-what-changed", label: "What changed", level: 1, moved: true },
  { id: "durable-execution", label: "Durable execution", level: 2 },
  { id: "sec-open-questions", label: "Open questions", level: 1 },
];

test("renders the Outline header", () => {
  render(<Outline outline={nodes} onJump={() => {}} />);
  expect(screen.getByText("Outline")).toBeInTheDocument();
});

test("renders a level-2 node with data-level=2", () => {
  render(<Outline outline={nodes} onJump={() => {}} />);
  const node = screen.getByRole("button", { name: /durable execution/i });
  expect(node).toHaveAttribute("data-level", "2");
});

test("a moved node shows a data-moved element", () => {
  const { container } = render(<Outline outline={nodes} onJump={() => {}} />);
  expect(container.querySelector("[data-moved]")).not.toBeNull();
});

test("clicking a node calls onJump with its id", async () => {
  const onJump = vi.fn();
  render(<Outline outline={nodes} onJump={onJump} />);
  await userEvent.click(screen.getByRole("button", { name: /durable execution/i }));
  expect(onJump).toHaveBeenCalledWith("durable-execution");
});
