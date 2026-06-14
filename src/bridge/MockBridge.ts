import type {
  StreamSummary,
  GetStreamResult,
  DraftInput,
  DraftResult,
  StreamDescription,
  Summary,
  StreamStatus,
  RefreshProgress,
} from "./types";
import type { Bridge } from "./Bridge";
import { sampleStreams, sampleDescriptions, sampleDocFor } from "./sampleData";

const STORAGE_KEY = "freshet-mock";

interface StoredState {
  summaries: StreamSummary[];
  descriptions: Record<string, StreamDescription>;
  documents: Record<string, string>;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

function replaceNotesSection(doc: string, newNotes: string): string {
  // Split on "\n## My notes" to isolate the prefix (everything before My notes)
  const marker = "\n## My notes";
  const idx = doc.indexOf(marker);
  if (idx === -1) {
    // No existing notes section — append
    return doc + marker + "\n" + newNotes.replace(/^## My notes\n?/, "");
  }
  const prefix = doc.slice(0, idx);
  // newNotes already starts with "## My notes"
  return prefix + "\n" + newNotes;
}

function seedState(): StoredState {
  const summaries = sampleStreams.map(s => ({ ...s }));
  const descriptions: Record<string, StreamDescription> = {};
  const documents: Record<string, string> = {};

  for (const id of Object.keys(sampleDescriptions)) {
    descriptions[id] = { ...sampleDescriptions[id] };
    documents[id] = sampleDocFor(id);
  }

  return { summaries, descriptions, documents };
}

export class MockBridge implements Bridge {
  private state: StoredState;
  private progressListeners: Array<(e: RefreshProgress) => void> = [];

  constructor() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        this.state = JSON.parse(stored) as StoredState;
      } catch {
        this.state = seedState();
      }
    } else {
      this.state = seedState();
    }
  }

  private persist(): void {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.state));
  }

  reset(): void {
    this.state = seedState();
    localStorage.removeItem(STORAGE_KEY);
  }

  async listStreams(): Promise<StreamSummary[]> {
    return this.state.summaries.map(s => ({ ...s }));
  }

  async getStream(id: string): Promise<GetStreamResult> {
    const description = this.state.descriptions[id];
    if (!description) throw new Error(`Stream not found: ${id}`);
    const summary = this.state.summaries.find(s => s.id === id);
    return {
      description: { ...description },
      documentMarkdown: this.state.documents[id] ?? "",
      lastCheckedAt: summary?.lastCheckedAt,
    };
  }

  async generateFirstDraft(input: DraftInput): Promise<DraftResult> {
    const id = slugify(input.topic);
    const title = input.topic
      .split(" ")
      .map(w => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ");

    const proposedDescription: StreamDescription = {
      id,
      title,
      topic: input.topic,
      sources: input.sources,
      cadence: input.cadence,
      status: "active",
      createdAt: new Date().toISOString(),
    };

    const draftMarkdown = `# ${title}
_draft · ${input.sources.length} sources_

## What changed
- No data yet — first refresh will populate this.

## Current understanding
### Overview
Tracking: ${input.topic}.

## Open questions
- What are the key developments to watch?

## My notes
- (add your notes here)
`;

    return { draftMarkdown, proposedDescription };
  }

  async createStream(desc: StreamDescription): Promise<StreamSummary> {
    const summary: StreamSummary = {
      id: desc.id,
      title: desc.title,
      changedSinceSeen: false,
    };
    this.state.summaries.push(summary);
    this.state.descriptions[desc.id] = { ...desc };
    this.state.documents[desc.id] = sampleDocFor(desc.id);
    this.persist();
    return { ...summary };
  }

  async refreshStream(id: string): Promise<Summary> {
    const summary = this.state.summaries.find(s => s.id === id);
    if (!summary) throw new Error(`Stream not found: ${id}`);

    // Deterministic: first refresh sets changed=true, subsequent ones toggle to false
    const changed = !summary.changedSinceSeen;
    summary.changedSinceSeen = changed;
    summary.lastCheckedAt = new Date().toISOString();

    // Emit progress events
    const phases: RefreshProgress["phase"][] = ["detecting", "researching", "synthesizing", "done"];
    for (const phase of phases) {
      const event: RefreshProgress = { streamId: id, phase };
      setTimeout(() => {
        for (const listener of this.progressListeners) {
          listener(event);
        }
      }, 0);
    }

    this.persist();
    return { changed, nNew: changed ? 1 : 0 };
  }

  async setStreamStatus(id: string, status: StreamStatus): Promise<void> {
    const desc = this.state.descriptions[id];
    if (!desc) throw new Error(`Stream not found: ${id}`);
    desc.status = status;
    this.persist();
  }

  async saveNotes(id: string, markdown: string): Promise<void> {
    const current = this.state.documents[id];
    if (current === undefined) throw new Error(`Stream not found: ${id}`);
    this.state.documents[id] = replaceNotesSection(current, markdown);
    this.persist();
  }

  onRefreshProgress(cb: (e: RefreshProgress) => void): () => void {
    this.progressListeners.push(cb);
    return () => {
      this.progressListeners = this.progressListeners.filter(l => l !== cb);
    };
  }
}
