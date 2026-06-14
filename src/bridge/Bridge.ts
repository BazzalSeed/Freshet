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

export interface Bridge {
  listStreams(): Promise<StreamSummary[]>;
  getStream(id: string): Promise<GetStreamResult>;
  generateFirstDraft(input: DraftInput): Promise<DraftResult>;
  createStream(desc: StreamDescription): Promise<StreamSummary>;
  refreshStream(id: string): Promise<Summary>;
  setStreamStatus(id: string, status: StreamStatus): Promise<void>;
  saveNotes(id: string, markdown: string): Promise<void>;
  onRefreshProgress(cb: (e: RefreshProgress) => void): () => void;
}
