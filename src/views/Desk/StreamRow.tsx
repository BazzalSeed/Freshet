import { useState } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import type { StreamSummary } from "../../bridge/types";

interface StreamRowProps {
  stream: StreamSummary;
  onOpen: (id: string) => void;
  onRefreshed: () => void;
}

export function StreamRow({ stream, onOpen, onRefreshed }: StreamRowProps) {
  const bridge = useBridge();
  const [refreshing, setRefreshing] = useState(false);

  async function handleRefresh(e: React.MouseEvent) {
    e.stopPropagation();
    if (refreshing) return;
    setRefreshing(true);
    try {
      await bridge.refreshStream(stream.id);
      onRefreshed();
    } finally {
      setRefreshing(false);
    }
  }

  const checkedLabel = stream.lastCheckedAt
    ? new Date(stream.lastCheckedAt).toLocaleString()
    : "never";

  return (
    <div className="desk-row">
      <button
        className="desk-row-main"
        aria-label={stream.title}
        onClick={() => onOpen(stream.id)}
        type="button"
      >
        <span className="desk-row-title">{stream.title}</span>
        <span className="desk-row-meta">
          <span className="desk-row-checked">{checkedLabel}</span>
          {stream.changedSinceSeen && (
            <span className="desk-row-moved" data-moved aria-label="something moved" />
          )}
        </span>
      </button>
      <button
        className="desk-row-refresh"
        aria-label={`Refresh ${stream.title}`}
        onClick={handleRefresh}
        type="button"
      >
        ↻
      </button>
    </div>
  );
}
