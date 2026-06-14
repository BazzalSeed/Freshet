import type { Citation } from "../../lib/parseDoc";
import "./Sources.css";

/**
 * Right panel: the full source set behind the document, one card per citation.
 * Header reads "Sources · {n}". Each card surfaces the origin (source), the
 * title, and the significance score.
 */
export function Sources({ sources }: { sources: Citation[] }) {
  return (
    <aside className="sources" aria-label="Sources">
      <p className="sources-header">Sources · {sources.length}</p>
      <ul className="sources-list">
        {sources.map((s) => (
          <li key={s.id} className="source-card">
            <p className="source-origin">{s.source}</p>
            <p className="source-title">{s.title}</p>
            {s.score !== undefined ? <p className="source-score">{s.score}</p> : null}
          </li>
        ))}
      </ul>
    </aside>
  );
}
