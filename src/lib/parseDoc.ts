export interface Citation {
  id: string;
  source: string;
  title: string;
  score?: number;
  date?: string;
  url: string;
}

export interface OutlineNode {
  id: string;
  label: string;
  level: 1 | 2;
  moved?: boolean;
}

export interface ParsedDoc {
  title: string;
  updatedLabel: string;
  sources: Citation[];
  outline: OutlineNode[];
  /**
   * The Freshet-owned movements (What changed / Current understanding / Open
   * questions) as raw GFM markdown, with the `[^id]:` footnote definitions
   * appended so react-markdown + remark-gfm resolve the inline `[^id]` refs.
   * Excludes the title, the updated label, and the `## My notes` section.
   */
  bodyMarkdown: string;
  /** Raw markdown body under `## My notes` (user-owned; rendered by the editor). */
  myNotes: string;
}

const FOOTNOTE_DEF_RE = /^\[\^([^\]]+)\]:\s*(.+)$/;

export function slugify(label: string): string {
  return label
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/**
 * Parse a footnote-definition line of the form:
 *   [^id]: source · title · score · date · url
 * Fields are separated by " · "; score is numeric; url is the last field.
 * Returns null if the line is not a footnote definition.
 */
function parseCitation(line: string): Citation | null {
  const m = line.match(FOOTNOTE_DEF_RE);
  if (!m) return null;
  const id = m[1];
  const fields = m[2].split(" · ").map((f) => f.trim());
  const source = fields[0] ?? "";
  const url = fields[fields.length - 1] ?? "";
  const middle = fields.slice(1, -1);
  let title = "";
  let score: number | undefined;
  let date: string | undefined;
  for (const field of middle) {
    if (score === undefined && /^\d+$/.test(field)) {
      score = Number(field);
    } else if (date === undefined && /^\d{4}-\d{2}-\d{2}/.test(field)) {
      date = field;
    } else if (!title) {
      title = field;
    }
  }
  return { id, source, title, ...(score !== undefined ? { score } : {}), ...(date ? { date } : {}), url };
}

const MY_NOTES_RE = /^##\s+My notes\s*$/;
const H1_RE = /^#\s+(.+)$/;
const H2_RE = /^##\s+(.+)$/;
const H3_RE = /^###\s+(.+)$/;
const UPDATED_RE = /^_(.+)_\s*$/;

export function parseDoc(md: string): ParsedDoc {
  const allLines = md.split("\n");

  // 1. Pull out the footnote-definition lines globally: they become the source
  //    metadata AND get re-appended to bodyMarkdown so refs resolve.
  const sources: Citation[] = [];
  const footnoteDefs: string[] = [];
  const lines: string[] = [];
  for (const line of allLines) {
    const cite = parseCitation(line);
    if (cite) {
      sources.push(cite);
      footnoteDefs.push(line);
    } else {
      lines.push(line);
    }
  }

  // 2. Title + updated label (rendered by the view header, not react-markdown).
  //    Only look ABOVE the first movement (## …) so a `#` heading the user types
  //    in their notes can never be mistaken for the stream title.
  const firstH2 = lines.findIndex((l) => H2_RE.test(l));
  const headerEnd = firstH2 === -1 ? lines.length : firstH2;
  let title = "";
  let updatedLabel = "";
  let titleIdx = -1;
  let updatedIdx = -1;
  for (let i = 0; i < headerEnd; i++) {
    const t = lines[i].match(H1_RE);
    if (t && !title) {
      title = t[1].trim();
      titleIdx = i;
      continue;
    }
    const u = lines[i].match(UPDATED_RE);
    if (u && !updatedLabel) {
      updatedLabel = u[1].trim();
      updatedIdx = i;
    }
  }

  // 3. Split off the My notes section.
  const myNotesIdx = lines.findIndex((l) => MY_NOTES_RE.test(l));
  const bodyEnd = myNotesIdx === -1 ? lines.length : myNotesIdx;

  const bodyLines = lines
    .slice(0, bodyEnd)
    .filter((_, i) => i !== titleIdx && i !== updatedIdx);

  const myNotes =
    myNotesIdx === -1 ? "" : lines.slice(myNotesIdx + 1).join("\n").trim();

  const body = bodyLines.join("\n").trim();
  const bodyMarkdown =
    footnoteDefs.length > 0 ? `${body}\n\n${footnoteDefs.join("\n")}` : body;

  // Order sources by first inline reference so the Sources panel numbers match
  // the rendered superscripts (GFM numbers footnotes by order of first ref).
  const refRe = /\[\^([^\]]+)\]/g;
  const refOrder: string[] = [];
  const seenRef = new Set<string>();
  for (let m = refRe.exec(body); m !== null; m = refRe.exec(body)) {
    if (!seenRef.has(m[1])) {
      seenRef.add(m[1]);
      refOrder.push(m[1]);
    }
  }
  const refIndex = (id: string) => {
    const i = refOrder.indexOf(id);
    return i === -1 ? Number.MAX_SAFE_INTEGER : i;
  };
  sources.sort((a, b) => refIndex(a.id) - refIndex(b.id));

  // 4. Outline from the body headings (## level 1, ### level 2), plus a
  //    terminal "My notes" node so the rail can jump to the editor.
  const outline: OutlineNode[] = [];
  for (const line of bodyLines) {
    const h3 = line.match(H3_RE);
    if (h3) {
      const label = h3[1].trim();
      outline.push({ id: slugify(label), label, level: 2 });
      continue;
    }
    const h2 = line.match(H2_RE);
    if (h2) {
      const label = h2[1].trim();
      outline.push({ id: slugify(label), label, level: 1 });
    }
  }
  if (myNotesIdx !== -1) {
    outline.push({ id: "my-notes", label: "My notes", level: 1 });
  }

  return { title, updatedLabel, sources, outline, bodyMarkdown, myNotes };
}
