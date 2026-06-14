import { parseDoc } from "./parseDoc";
import { SAMPLE_DOC } from "../bridge/sampleData";

test("parses movements, subsections, sources, outline", () => {
  const d = parseDoc(SAMPLE_DOC);
  expect(d.title).toBe("AI Agents");
  expect(d.updatedLabel).toMatch(/updated/i);
  expect(d.whatChanged.length).toBe(2);
  expect(d.whatChanged[0]).toContain("[^hn1]"); // citation marker preserved for the renderer
  expect(d.current.map((s) => s.heading)).toEqual(["Durable execution", "Tool calling"]);
  expect(d.openQuestions.length).toBe(1); // footnote-def lines must NOT count as open questions
  expect(d.myNotes).toContain("Q3 planning");
  const hn = d.sources.find((s) => s.id === "hn1")!;
  expect(hn.source).toBe("hackernews");
  expect(hn.score).toBe(412);
  expect(hn.url).toContain("http");
  expect(hn.title).toBe("Claude Agent SDK v2");
  expect(d.outline.map((n) => n.label)).toEqual([
    "What changed",
    "Current understanding",
    "Durable execution",
    "Tool calling",
    "Open questions",
    "My notes",
  ]);
  expect(d.outline.find((n) => n.label === "Durable execution")!.level).toBe(2);
});
