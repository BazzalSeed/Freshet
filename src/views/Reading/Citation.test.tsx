import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Citation } from "./Citation";

const c = {
  id: "hn1",
  source: "hackernews",
  title: "Claude Agent SDK v2",
  score: 412,
  url: "https://x",
};

test("chip opens popover with source", async () => {
  render(<Citation citation={c as any} />);
  expect(screen.getByText(/412/)).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button"));
  expect(await screen.findByText("Claude Agent SDK v2")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /open/i })).toHaveAttribute("href", "https://x");
});

test("chip label abbreviates known sources", () => {
  render(<Citation citation={c as any} />);
  expect(screen.getByRole("button")).toHaveTextContent("HN 412");
});
