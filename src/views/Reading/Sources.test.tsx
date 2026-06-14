import { render, screen } from "@testing-library/react";
import type { Citation } from "../../lib/parseDoc";
import { Sources } from "./Sources";

const sources: Citation[] = [
  { id: "hn1", source: "hackernews", title: "Claude Agent SDK v2", score: 412, url: "https://x/1" },
  { id: "r1", source: "reddit", title: "Off ReAct loops in prod", score: 280, url: "https://x/2" },
  { id: "gh1", source: "github", title: "anthropics/agent-sdk v2.0", score: 1200, url: "https://x/3" },
  { id: "pm1", source: "polymarket", title: "MCP default by EOY", score: 61, url: "https://x/4" },
];

test("header shows the count", () => {
  render(<Sources sources={sources} />);
  const header = screen.getByText(/Sources/);
  expect(header).toHaveTextContent("Sources");
  expect(header).toHaveTextContent("4");
});

test("renders each source title", () => {
  render(<Sources sources={sources} />);
  for (const s of sources) {
    expect(screen.getByText(s.title)).toBeInTheDocument();
  }
});
