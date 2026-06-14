import { useEffect, useMemo, useState } from "react";
import { useBridge } from "../../bridge/BridgeProvider";
import { parseDoc } from "../../lib/parseDoc";
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
  const [showOutline, setShowOutline] = useState(false);
  const [showSources, setShowSources] = useState(false);

  useEffect(() => {
    let active = true;
    bridge.getStream(streamId).then((r) => {
      if (active) setMarkdown(r.documentMarkdown);
    });
    return () => {
      active = false;
    };
  }, [bridge, streamId]);

  const doc = useMemo(() => (markdown ? parseDoc(markdown) : null), [markdown]);

  const handleRefresh = async () => {
    await bridge.refreshStream(streamId);
    const r = await bridge.getStream(streamId);
    setMarkdown(r.documentMarkdown);
  };

  const handleSaveNotes = async (block: string) => {
    await bridge.saveNotes(streamId, block);
    const r = await bridge.getStream(streamId);
    setMarkdown(r.documentMarkdown);
  };

  const onJump = (id: string) => {
    document.getElementById(id)?.scrollIntoView?.({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="reading">
      <Chrome
        title={doc?.title ?? ""}
        updatedLabel={doc?.updatedLabel}
        showOutline={showOutline}
        showSources={showSources}
        onToggleOutline={() => setShowOutline((v) => !v)}
        onToggleSources={() => setShowSources((v) => !v)}
        onBack={onBack}
        onRefresh={handleRefresh}
      />

      {doc ? (
        <div
          className="reading-body"
          data-outline={showOutline ? "" : undefined}
          data-sources={showSources ? "" : undefined}
        >
          {showOutline ? <Outline outline={doc.outline} onJump={onJump} /> : null}
          <div className="reading-center">
            <Document doc={doc} onSaveNotes={handleSaveNotes} />
          </div>
          {showSources ? <Sources sources={doc.sources} /> : null}
        </div>
      ) : (
        <div className="reading-empty" aria-hidden />
      )}
    </div>
  );
}
