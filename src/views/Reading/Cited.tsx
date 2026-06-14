import { Fragment } from "react";
import type { Citation as CitationType } from "../../lib/parseDoc";
import { Citation } from "./Citation";

const MARKER_RE = /\[\^([A-Za-z0-9_]+)\]/;

/**
 * Renders body text, replacing inline `[^id]` markers with <Citation> chips.
 * Splitting with a capturing group yields alternating [text, id, text, id, …].
 * If an id has no matching source, the literal `[^id]` text is preserved.
 */
export function Cited({ text, sources }: { text: string; sources: CitationType[] }) {
  // Split on the capturing group so captured ids land at odd indices.
  const parts = text.split(new RegExp(MARKER_RE.source, "g"));

  return (
    <>
      {parts.map((part, i) => {
        // Even indices are plain text between markers.
        if (i % 2 === 0) {
          return part ? <Fragment key={i}>{part}</Fragment> : null;
        }
        // Odd indices are captured ids.
        const citation = sources.find((s) => s.id === part);
        if (!citation) {
          // Unknown id: render the original literal marker.
          return <Fragment key={i}>{`[^${part}]`}</Fragment>;
        }
        return <Citation key={i} citation={citation} />;
      })}
    </>
  );
}
