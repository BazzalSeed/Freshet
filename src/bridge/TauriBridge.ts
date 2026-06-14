import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
import type { Bridge } from "./Bridge";

/**
 * The real backend bridge: maps each `Bridge` method onto its `#[tauri::command]`
 * and the `refresh_progress` event.
 *
 * Wire payloads are already camelCase (serde `rename_all = "camelCase"`), so no
 * remapping is needed. Tauri converts JS camelCase arg keys → Rust snake_case
 * params, so arg keys are passed as the Rust fns name them (`id`, `path`,
 * `kind`, `status`, `markdown`, `input`, `description`).
 */
export class TauriBridge implements Bridge {
  // ── Onboarding / config / agents ───────────────────────────────────────────
  getOnboardingState(): Promise<OnboardingState> {
    return invoke<OnboardingState>("get_onboarding_state");
  }

  getConfig(): Promise<AppConfig> {
    return invoke<AppConfig>("get_config");
  }

  listAgents(): Promise<AgentStatus[]> {
    return invoke<AgentStatus[]>("list_agents");
  }

  recheckAgents(): Promise<AgentStatus[]> {
    return invoke<AgentStatus[]>("recheck_agents");
  }

  setRootFolder(path: string): Promise<void> {
    return invoke<void>("set_root_folder", { path });
  }

  setDefaultAgent(kind: AgentKind): Promise<void> {
    return invoke<void>("set_default_agent", { kind });
  }

  completeOnboarding(): Promise<void> {
    return invoke<void>("complete_onboarding");
  }

  // ── Streams ────────────────────────────────────────────────────────────────
  listStreams(): Promise<StreamSummary[]> {
    return invoke<StreamSummary[]>("list_streams");
  }

  getStream(id: string): Promise<GetStreamResult> {
    return invoke<GetStreamResult>("get_stream", { id });
  }

  generateFirstDraft(input: DraftInput): Promise<DraftResult> {
    return invoke<DraftResult>("generate_first_draft", { input });
  }

  createStream(desc: StreamDescription): Promise<StreamSummary> {
    return invoke<StreamSummary>("create_stream", { description: desc });
  }

  refreshStream(id: string): Promise<Summary> {
    return invoke<Summary>("refresh_stream", { id });
  }

  setStreamStatus(id: string, status: StreamStatus): Promise<void> {
    return invoke<void>("set_stream_status", { id, status });
  }

  saveNotes(id: string, markdown: string): Promise<void> {
    return invoke<void>("save_notes", { id, markdown });
  }

  openUrl(url: string): Promise<void> {
    return invoke<void>("open_url", { url });
  }

  onRefreshProgress(cb: (e: RefreshProgress) => void): () => void {
    // `listen` returns a Promise<UnlistenFn>; we expose a synchronous unlisten
    // that awaits and invokes the real unlistener once it's available.
    const unlistenPromise = listen<RefreshProgress>("refresh_progress", e => cb(e.payload));
    return () => {
      void unlistenPromise.then(unlisten => unlisten());
    };
  }
}
