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

test("clicking the marker reveals its source (by id)", async () => {
  const onCite = vi.fn();
  render(<Citation citation={c as any} label={1} onCite={onCite} />);
  await userEvent.click(screen.getByRole("button"));
  expect(onCite).toHaveBeenCalledWith("hn1");
});

test("renders the footnote label as the marker", () => {
  render(<Citation citation={c as any} label={3} onCite={() => {}} />);
  expect(screen.getByRole("button")).toHaveTextContent("3");
});

test("names its source for hover/assistive tech", () => {
  render(<Citation citation={c as any} label={1} onCite={() => {}} />);
  expect(screen.getByRole("button")).toHaveAccessibleName(/Claude Agent SDK v2/i);
});
