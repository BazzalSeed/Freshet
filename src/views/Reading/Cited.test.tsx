import { render, screen } from "@testing-library/react";
import { Cited } from "./Cited";

const sources = [
  { id: "hn1", source: "hackernews", title: "SDK v2", score: 412, url: "https://x" },
];

test("renders prose and replaces [^id] with a Citation chip", () => {
  render(<Cited text="Anthropic shipped v2. [^hn1]" sources={sources as any} />);
  expect(screen.getByText(/Anthropic shipped v2\./)).toBeInTheDocument();
  expect(screen.getByRole("button")).toBeInTheDocument();
  expect(screen.queryByText(/\[\^hn1\]/)).not.toBeInTheDocument();
});

test("renders literal marker when id has no matching source", () => {
  render(<Cited text="Mystery claim. [^unknown]" sources={sources as any} />);
  expect(screen.getByText(/\[\^unknown\]/)).toBeInTheDocument();
  expect(screen.queryByRole("button")).not.toBeInTheDocument();
});
