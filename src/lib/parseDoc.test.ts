import { parseDoc } from "./parseDoc";
import { SAMPLE_DOC } from "../bridge/sampleData";

test("parses title, updated label, sources, outline, body, and notes", () => {
  const d = parseDoc(SAMPLE_DOC);
  expect(d.title).toBe("AI Agents");
  expect(d.updatedLabel).toMatch(/updated/i);

  // Body markdown holds the movements + footnote defs, but NOT the title,
  // updated label, or the My notes section.
  expect(d.bodyMarkdown).toContain("## What changed");
  expect(d.bodyMarkdown).toContain("## Current understanding");
  expect(d.bodyMarkdown).toContain("## Open questions");
  expect(d.bodyMarkdown).toContain("[^hn1]"); // inline ref preserved for the renderer
  expect(d.bodyMarkdown).toContain("[^hn1]:"); // footnote def re-appended so it resolves
  expect(d.bodyMarkdown).not.toContain("# AI Agents");
  expect(d.bodyMarkdown).not.toContain("## My notes");

  // My notes split off as raw markdown.
  expect(d.myNotes).toContain("Q3 planning");
  expect(d.myNotes).not.toContain("## My notes");

  // Citation metadata parsed from the ` · `-separated defs.
  const hn = d.sources.find((s) => s.id === "hn1")!;
  expect(hn.source).toBe("hackernews");
  expect(hn.score).toBe(412);
  expect(hn.url).toContain("http");
  expect(hn.title).toBe("Claude Agent SDK v2");

  // Outline = body headings + a terminal My notes node.
  expect(d.outline.map((n) => n.label)).toEqual([
    "What changed",
    "Current understanding",
    "Durable execution",
    "Tool calling",
    "Open questions",
    "My notes",
  ]);
  expect(d.outline.find((n) => n.label === "Durable execution")!.level).toBe(2);
  // Heading ids match the slugged ids the Outline jumps to.
  expect(d.outline.find((n) => n.label === "What changed")!.id).toBe("what-changed");
  expect(d.outline.find((n) => n.label === "My notes")!.id).toBe("my-notes");
});

test("title survives a My notes edit (no bleed)", () => {
  const d1 = parseDoc(SAMPLE_DOC);
  // Simulate the saveNotes splice: replace the My notes body, keep the prefix.
  const edited = SAMPLE_DOC.replace(
    /## My notes[\s\S]*$/,
    "## My notes\n- A brand new private thought.\n",
  );
  const d2 = parseDoc(edited);
  expect(d2.title).toBe(d1.title);
  expect(d2.title).toBe("AI Agents");
  expect(d2.myNotes).toContain("brand new private thought");
});
