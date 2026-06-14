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

export interface CurrentSection {
  heading?: string;
  body: string[];
}

export interface ParsedDoc {
  title: string;
  updatedLabel: string;
  sources: Citation[];
  outline: OutlineNode[];
  whatChanged: string[];
  current: CurrentSection[];
  openQuestions: string[];
  myNotes: string;
}

const FOOTNOTE_DEF_RE = /^\[\^([^\]]+)\]:\s*(.+)$/;

function slugify(label: string): string {
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
  // Minimum: source · ... · url (url is always last).
  const source = fields[0] ?? "";
  const url = fields[fields.length - 1] ?? "";
  // Middle fields: title, optional score (numeric), optional date.
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

export function parseDoc(md: string): ParsedDoc {
  const allLines = md.split("\n");

  // 1. Collect citations globally and strip definition lines from everything else.
  const sources: Citation[] = [];
  const lines: string[] = [];
  for (const line of allLines) {
    const cite = parseCitation(line);
    if (cite) {
      sources.push(cite);
    } else {
      lines.push(line);
    }
  }

  // 2. Title and updated label.
  let title = "";
  let updatedLabel = "";
  for (const line of lines) {
    const t = line.match(/^#\s+(.+)$/);
    if (t && !title) {
      title = t[1].trim();
      continue;
    }
    const u = line.match(/^_(.+)_\s*$/);
    if (u && !updatedLabel) {
      updatedLabel = u[1].trim();
    }
  }

  // 3. Split into movements on "## " headings.
  const movements: { heading: string; body: string[] }[] = [];
  let currentMovement: { heading: string; body: string[] } | null = null;
  for (const line of lines) {
    const h = line.match(/^##\s+(.+)$/);
    if (h) {
      currentMovement = { heading: h[1].trim(), body: [] };
      movements.push(currentMovement);
    } else if (currentMovement) {
      currentMovement.body.push(line);
    }
  }

  const bulletLines = (body: string[]): string[] =>
    body
      .map((l) => l.match(/^-\s+(.*)$/))
      .filter((m): m is RegExpMatchArray => m !== null)
      .map((m) => m[1].trim());

  const parseCurrent = (body: string[]): CurrentSection[] => {
    const sections: CurrentSection[] = [];
    let section: CurrentSection | null = null;
    const pushBodyLine = (line: string) => {
      const trimmed = line.trim();
      if (!trimmed) return; // skip blank lines
      if (/^-\s+/.test(trimmed)) return; // skip bullets
      if (!section) {
        section = { heading: undefined, body: [] };
        sections.push(section);
      }
      section.body.push(trimmed);
    };
    for (const line of body) {
      const sub = line.match(/^###\s+(.+)$/);
      if (sub) {
        section = { heading: sub[1].trim(), body: [] };
        sections.push(section);
      } else {
        pushBodyLine(line);
      }
    }
    return sections;
  };

  let whatChanged: string[] = [];
  let current: CurrentSection[] = [];
  let openQuestions: string[] = [];
  let myNotes = "";

  const outline: OutlineNode[] = [];

  for (const mv of movements) {
    outline.push({ id: slugify(mv.heading), label: mv.heading, level: 1 });
    switch (mv.heading) {
      case "What changed":
        whatChanged = bulletLines(mv.body);
        break;
      case "Current understanding":
        current = parseCurrent(mv.body);
        for (const sec of current) {
          if (sec.heading) {
            outline.push({ id: slugify(sec.heading), label: sec.heading, level: 2 });
          }
        }
        break;
      case "Open questions":
        openQuestions = bulletLines(mv.body);
        break;
      case "My notes":
        myNotes = mv.body.join("\n").trim();
        break;
      default:
        // Unknown movement: outline node already added; body ignored.
        break;
    }
  }

  return { title, updatedLabel, sources, outline, whatChanged, current, openQuestions, myNotes };
}
