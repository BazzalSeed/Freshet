import { MockBridge } from "./MockBridge";

beforeEach(() => localStorage.clear());

test("lists streams; refresh marks changed; saveNotes persists in markdown", async () => {
  const b = new MockBridge();
  const list = await b.listStreams();
  expect(list.length).toBeGreaterThan(0);
  const r = await b.refreshStream(list[0].id);
  expect(typeof r.changed).toBe("boolean");
  const after = await b.listStreams();
  expect(after.find(s => s.id === list[0].id)!.changedSinceSeen).toBe(r.changed);

  // Capture the entire slice BEFORE "## My notes" so we can prove it's byte-identical after editing.
  const before = await b.getStream("ai-agents");
  const splitIdx = before.documentMarkdown.indexOf("\n## My notes");
  expect(splitIdx).toBeGreaterThan(-1);
  const prefixBefore = before.documentMarkdown.slice(0, splitIdx);
  // Sanity: citation definitions live in that prefix (above My notes).
  expect(prefixBefore).toContain("[^hn1]: hackernews");

  await b.saveNotes("ai-agents", "## My notes\n- edited note\n");
  const doc = await b.getStream("ai-agents");
  expect(doc.documentMarkdown).toContain("- edited note");
  // saveNotes must ONLY touch the My notes block — earlier movements intact:
  expect(doc.documentMarkdown).toContain("## What changed");
  // Citation/footnote definitions MUST survive a notes edit (regression: they used to be wiped):
  expect(doc.documentMarkdown).toContain("[^hn1]: hackernews");

  // The whole prefix before "## My notes" must be byte-identical to before the edit.
  const splitIdxAfter = doc.documentMarkdown.indexOf("\n## My notes");
  const prefixAfter = doc.documentMarkdown.slice(0, splitIdxAfter);
  expect(prefixAfter).toBe(prefixBefore);
});

test("persists across instances via localStorage", async () => {
  const a = new MockBridge();
  await a.saveNotes("ai-agents", "## My notes\n- persisted\n");
  const b = new MockBridge();                 // fresh instance, same localStorage
  const doc = await b.getStream("ai-agents");
  expect(doc.documentMarkdown).toContain("- persisted");
});
