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
  await b.saveNotes("ai-agents", "## My notes\n- edited note\n");
  const doc = await b.getStream("ai-agents");
  expect(doc.documentMarkdown).toContain("- edited note");
  // saveNotes must ONLY touch the My notes block — earlier movements intact:
  expect(doc.documentMarkdown).toContain("## What changed");
});

test("persists across instances via localStorage", async () => {
  const a = new MockBridge();
  await a.saveNotes("ai-agents", "## My notes\n- persisted\n");
  const b = new MockBridge();                 // fresh instance, same localStorage
  const doc = await b.getStream("ai-agents");
  expect(doc.documentMarkdown).toContain("- persisted");
});
