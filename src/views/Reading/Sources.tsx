import { useEffect, useRef } from "react";
import type { Citation } from "../../lib/parseDoc";
import { sourceLabel } from "./Citation";
import "./Sources.css";

/**
 * Right panel: the full source set behind the document. Each entry is a clickable
 * name (the source title) that opens the reference in-app; the origin + score sit
 * beneath as quiet metadata. When a citation is clicked in the document,
 * `highlightId` scrolls the matching card into view and flashes it.
 */
export function Sources({
  sources,
  onOpenUrl,
  highlightId,
  onHighlightConsumed,
}: {
  sources: Citation[];
  onOpenUrl: (url: string) => void;
  highlightId?: string | null;
  onHighlightConsumed?: () => void;
}) {
  const refs = useRef<Record<string, HTMLLIElement | null>>({});

  useEffect(() => {
    if (!highlightId) return;
    const el = refs.current[highlightId];
    if (!el) return;
    el.scrollIntoView?.({ behavior: "smooth", block: "center" });
    el.setAttribute("data-flash", "");
    const t = setTimeout(() => {
      el.removeAttribute("data-flash");
      onHighlightConsumed?.();
    }, 1400);
    return () => clearTimeout(t);
  }, [highlightId, onHighlightConsumed]);

  return (
    <aside className="sources" aria-label="Sources">
      <p className="sources-header">Sources · {sources.length}</p>
      <ol className="sources-list">
        {sources.map((s, i) => (
          <li
            key={s.id}
            className="source-card"
            ref={(el) => {
              refs.current[s.id] = el;
            }}
          >
            <span className="source-num" aria-hidden>
              {i + 1}
            </span>
            <div className="source-body">
              <button
                type="button"
                className="source-name"
                onClick={() => onOpenUrl(s.url)}
                title={s.url}
              >
                {s.title || s.url}
              </button>
              <p className="source-meta">
                <span className="source-origin">{sourceLabel(s.source)}</span>
                {s.score !== undefined ? <span className="source-score">{s.score} pts</span> : null}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </aside>
  );
}
