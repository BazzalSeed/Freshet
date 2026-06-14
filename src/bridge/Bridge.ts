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
} from "./types";

export interface Bridge {
  // Onboarding / config / agents
  getOnboardingState(): Promise<OnboardingState>;
  getConfig(): Promise<AppConfig>;
  listAgents(): Promise<AgentStatus[]>;
  recheckAgents(): Promise<AgentStatus[]>;
  setRootFolder(path: string): Promise<void>;
  setDefaultAgent(kind: AgentKind): Promise<void>;
  completeOnboarding(): Promise<void>;

  // Streams
  listStreams(): Promise<StreamSummary[]>;
  getStream(id: string): Promise<GetStreamResult>;
  generateFirstDraft(input: DraftInput): Promise<DraftResult>;
  createStream(desc: StreamDescription): Promise<StreamSummary>;
  refreshStream(id: string): Promise<Summary>;
  setStreamStatus(id: string, status: StreamStatus): Promise<void>;
  saveNotes(id: string, markdown: string): Promise<void>;
  onRefreshProgress(cb: (e: RefreshProgress) => void): () => void;

  /** Open a web reference URL in-app (a reusable webview window). */
  openUrl(url: string): Promise<void>;
}
