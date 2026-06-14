import { useEffect, useMemo, useState } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import { asFreshetError } from "../../bridge/types";
import type { FreshetError, StreamDescription } from "../../bridge/types";
import { parseDoc } from "../../lib/parseDoc";
import { AgentNotice } from "../../components/AgentNotice";
import { Chrome } from "./Chrome";
import { Outline } from "./Outline";
import { Sources } from "./Sources";
import { Document } from "./Document";
import "./Reading.css";

/**
 * The reading view shell: a 3-zone layout (left Outline rail · center Document
 * · right Sources panel) under a Chrome top bar. Both rails default collapsed
 * so the document reads as a clean single column; the user reveals them via the
 * Chrome toggles. Refresh and notes round-trip through the bridge and re-fetch
 * the document so the rendered view reflects persisted state.
 */
export function Reading({
  streamId,
  onBack,
}: {
  streamId: string;
  onBack: () => void;
}) {
  const bridge = useBridge();
  const [markdown, setMarkdown] = useState<string | null>(null);
  const [description, setDescription] = useState<StreamDescription | null>(null);
  const [showOutline, setShowOutline] = useState(false);
  const [showSources, setShowSources] = useState(false);
  const [highlightSourceId, setHighlightSourceId] = useState<string | null>(null);
  const [scrolled, setScrolled] = useState(false);
  const [refreshError, setRefreshError] = useState<FreshetError | null>(null);

  useEffect(() => {
    let active = true;
    bridge.getStream(streamId).then((r) => {
      if (active) {
        setMarkdown(r.documentMarkdown);
        setDescription(r.description);
      }
    });
    return () => {
      active = false;
    };
  }, [bridge, streamId]);

  const doc = useMemo(() => (markdown ? parseDoc(markdown) : null), [markdown]);

  // The stream title is authoritative from its description (the document body is
  // the agent-owned movements), so it shows even if the markdown has no header.
  const title = description?.title ?? doc?.title ?? "";

  const handleRefresh = async () => {
    setRefreshError(null);
    try {
      await bridge.refreshStream(streamId);
      const r = await bridge.getStream(streamId);
      setMarkdown(r.documentMarkdown);
      setDescription(r.description);
    } catch (e) {
      setRefreshError(asFreshetError(e));
    }
  };

  const handleRecheckAndRefresh = async () => {
    setRefreshError(null);
    await bridge.recheckAgents();
    await handleRefresh();
  };

  const handleSaveNotes = async (block: string) => {
    await bridge.saveNotes(streamId, block);
    const r = await bridge.getStream(streamId);
    setMarkdown(r.documentMarkdown);
    setDescription(r.description);
  };

  const onJump = (id: string) => {
    document.getElementById(id)?.scrollIntoView?.({ behavior: "smooth", block: "start" });
  };

  // A regular link in the document opens in-app.
  const onOpenUrl = (url: string) => {
    void bridge.openUrl(url);
  };

  // A citation marker reveals the Sources panel and highlights its source —
  // opening the actual page is a deliberate second click on the source card.
  const onCite = (id: string) => {
    setShowSources(true);
    setHighlightSourceId(id);
  };

  return (
    <div className="reading">
      <Chrome
        title={title}
        updatedLabel={doc?.updatedLabel}
        showOutline={showOutline}
        showSources={showSources}
        onToggleOutline={() => setShowOutline((v) => !v)}
        onToggleSources={() => setShowSources((v) => !v)}
        onBack={onBack}
        onRefresh={handleRefresh}
        scrolled={scrolled}
      />

      {refreshError && (
        <div className="reading-agent-error">
          <AgentNotice
            error={refreshError}
            onRecheck={handleRecheckAndRefresh}
            onRetry={handleRefresh}
          />
        </div>
      )}

      {doc ? (
        <div
          className="reading-body"
          data-outline={showOutline ? "" : undefined}
          data-sources={showSources ? "" : undefined}
        >
          {showOutline ? <Outline outline={doc.outline} onJump={onJump} /> : null}
          <div
            className="reading-center"
            onScroll={(e) => setScrolled(e.currentTarget.scrollTop > 4)}
          >
            <Document
              doc={doc}
              title={title}
              onSaveNotes={handleSaveNotes}
              onOpenUrl={onOpenUrl}
              onCite={onCite}
            />
          </div>
          {showSources ? (
            <Sources
              sources={doc.sources}
              onOpenUrl={onOpenUrl}
              highlightId={highlightSourceId}
              onHighlightConsumed={() => setHighlightSourceId(null)}
            />
          ) : null}
        </div>
      ) : (
        <div className="reading-empty" aria-hidden />
      )}
    </div>
  );
}
