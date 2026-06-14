import type { OutlineNode } from "../../lib/parseDoc";
import "./Outline.css";

/**
 * Left rail: a jump-list of the document's movements and subsections. Level-2
 * nodes are visually indented (data-level="2"); a node whose content moved
 * since last seen shows a small dot (data-moved). Clicking a node asks the
 * shell to scroll its section into view.
 */
export function Outline({
  outline,
  onJump,
}: {
  outline: OutlineNode[];
  onJump: (id: string) => void;
}) {
  return (
    <nav className="outline" aria-label="Document outline">
      <p className="outline-header">Outline</p>
      <ul className="outline-list">
        {outline.map((node) => (
          <li key={node.id}>
            <button
              type="button"
              className="outline-item"
              data-level={String(node.level)}
              onClick={() => onJump(node.id)}
            >
              {node.moved ? <span className="outline-moved" data-moved aria-hidden /> : null}
              <span className="outline-label">{node.label}</span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
