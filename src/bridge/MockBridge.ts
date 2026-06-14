import type {
  StreamSummary,
  GetStreamResult,
  DraftInput,
  DraftResult,
  StreamDescription,
  Summary,
  StreamStatus,
  RefreshProgress,
  OnboardingState,
  AppConfig,
  AgentStatus,
  AgentKind,
  FreshetError,
} from "./types";
import type { Bridge } from "./Bridge";
import { sampleStreams, sampleDescriptions, sampleDocFor } from "./sampleData";

const STORAGE_KEY = "freshet-mock";

const MOCK_AGENT: AgentStatus = { kind: "claude_code", available: true, version: "mock" };

/** Simulate agent state in MockBridge for testing error surfaces. */
export type MockAgentState = "ok" | "not_logged_in" | "none";

export interface MockBridgeOptions {
  /** Override the onboarding state returned by getOnboardingState/listAgents/recheckAgents.
   *  Default: onboarded=true (keeps all existing tests unaffected). */
  onboardingState?: {
    onboarded: boolean;
    hasRoot: boolean;
    agent?: AgentStatus | null;
  };
  /**
   * Simulate a specific agent state. When set:
   * - "ok" (default): normal behavior, existing tests unaffected.
   * - "not_logged_in": generateFirstDraft/refreshStream/createStream throw a
   *   FreshetError with code "not_logged_in"; listAgents returns the agent as
   *   available (it exists but isn't authed).
   * - "none": generateFirstDraft/refreshStream/createStream throw a FreshetError
   *   with code "no_agent"; listAgents returns [].
   */
  agentState?: MockAgentState;
}

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
  private _onboardingState: { onboarded: boolean; hasRoot: boolean; agent?: AgentStatus | null };
  private _agentState: MockAgentState;

  constructor(options?: MockBridgeOptions) {
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

    this._agentState = options?.agentState ?? "ok";

    // Build default onboardingState from agentState if not explicitly provided.
    if (options?.onboardingState) {
      this._onboardingState = options.onboardingState;
    } else if (this._agentState === "none") {
      this._onboardingState = { onboarded: true, hasRoot: true, agent: null };
    } else if (this._agentState === "not_logged_in") {
      // Agent exists but is not authenticated — still show it as available.
      this._onboardingState = { onboarded: true, hasRoot: true, agent: { ...MOCK_AGENT } };
    } else {
      // Default: onboarded=true so all existing tests/surfaces are unaffected.
      this._onboardingState = { onboarded: true, hasRoot: true, agent: { ...MOCK_AGENT } };
    }
  }

  /** Throw the appropriate FreshetError for the current agentState, if any. */
  private _throwIfAgentError(): void {
    if (this._agentState === "not_logged_in") {
      const err: FreshetError = {
        code: "not_logged_in",
        message: "The agent is not logged in.",
        hint: "Open your terminal, run `claude` then `/login`, then re-check.",
      };
      throw err;
    }
    if (this._agentState === "none") {
      const err: FreshetError = {
        code: "no_agent",
        message: "No agent detected or selected.",
        hint: "Install Claude Code or Codex, then re-check.",
      };
      throw err;
    }
  }

  /** Test helper: override the onboarding state after construction. */
  __setOnboarding(state: { onboarded: boolean; hasRoot: boolean; agent?: AgentStatus | null }): void {
    this._onboardingState = state;
  }

  private persist(): void {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.state));
  }

  reset(): void {
    this.state = seedState();
    this.persist();
  }

  // ── Onboarding / config / agents ─────────────────────────────────────────
  async getOnboardingState(): Promise<OnboardingState> {
    const { onboarded, hasRoot, agent } = this._onboardingState;
    return {
      onboarded,
      hasRoot,
      ...(agent != null ? { agent: { ...agent } } : {}),
    };
  }

  async getConfig(): Promise<AppConfig> {
    return {
      root: this._onboardingState.hasRoot ? "/mock/vault" : undefined,
      selectedAgent: this._onboardingState.agent?.kind ?? undefined,
      onboarded: this._onboardingState.onboarded,
    };
  }

  async listAgents(): Promise<AgentStatus[]> {
    if (this._agentState === "none") return [];
    const { agent } = this._onboardingState;
    // Return the agent status (available or not) so callers can distinguish
    // "agent found but unavailable" from "no agent at all".
    if (agent != null) return [{ ...agent }];
    return [];
  }

  async recheckAgents(): Promise<AgentStatus[]> {
    return this.listAgents();
  }

  async setRootFolder(_path: string): Promise<void> {
    this._onboardingState = { ...this._onboardingState, hasRoot: true };
  }

  async setDefaultAgent(_kind: AgentKind): Promise<void> {}

  async completeOnboarding(): Promise<void> {
    this._onboardingState = { ...this._onboardingState, onboarded: true };
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
    this._throwIfAgentError();
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
    this._throwIfAgentError();
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
    this._throwIfAgentError();
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
