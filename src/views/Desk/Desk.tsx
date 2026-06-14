import { useEffect, useState, useCallback } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import type { StreamSummary } from "../../bridge/types";
import { AgentStatusChip } from "../../components/AgentStatusChip";
import { StreamRow } from "./StreamRow";
import "./Desk.css";

interface DeskProps {
  onOpen: (id: string) => void;
  onNew: () => void;
}

export function Desk({ onOpen, onNew }: DeskProps) {
  const bridge = useBridge();
  const [streams, setStreams] = useState<StreamSummary[]>([]);

  const fetchStreams = useCallback(() => {
    bridge.listStreams().then(setStreams);
  }, [bridge]);

  useEffect(() => {
    fetchStreams();
  }, [fetchStreams]);

  return (
    <div className="desk">
      <header className="desk-header">
        <h1 className="desk-title">Freshet</h1>
        <div className="desk-header-actions">
          <AgentStatusChip />
          <button
            className="desk-new"
            aria-label="New stream"
            onClick={onNew}
            type="button"
          >
            New stream
          </button>
        </div>
      </header>

      <section className="desk-streams" aria-label="Streams">
        {streams.length === 0 && (
          <p className="desk-empty">No streams yet. Create one to get started.</p>
        )}
        {streams.map((stream) => (
          <StreamRow
            key={stream.id}
            stream={stream}
            onOpen={onOpen}
            onRefreshed={fetchStreams}
          />
        ))}
      </section>
    </div>
  );
}
