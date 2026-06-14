import { type ReactNode } from "react";
import type { Citation as CitationType } from "../../lib/parseDoc";
import "./Citation.css";

const SOURCE_LABELS: Record<string, string> = {
  hackernews: "Hacker News",
  reddit: "Reddit",
  github: "GitHub",
  polymarket: "Polymarket",
};

export function sourceLabel(source: string): string {
  return SOURCE_LABELS[source] ?? source;
}

/**
 * An in-document citation: a restrained superscript marker (the footnote number).
 * Clicking it reveals and highlights the matching source in the Sources panel
 * (opening the page itself is a deliberate click on the source card). The native
 * tooltip names the source on hover.
 */
export function Citation({
  citation,
  label,
  onCite,
}: {
  citation: CitationType;
  label?: ReactNode;
  onCite: (citationId: string) => void;
}) {
  const { id, source, title, url } = citation;
  return (
    <button
      type="button"
      className="citation-ref"
      title={`${sourceLabel(source)} — ${title || url}`}
      aria-label={`Show source: ${title || sourceLabel(source)}`}
      onClick={() => onCite(id)}
    >
      {label ?? "•"}
    </button>
  );
}
