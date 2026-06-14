import "./Chrome.css";

/**
 * The reading view's top bar. Holds navigation (back), the two rail toggles
 * (outline / sources — icon-only so they don't collide with the rails' own
 * text headers), the centered document title, and a refresh control with the
 * "updated …" label. Toggles expose aria-pressed so state is announced.
 */
export function Chrome({
  title,
  updatedLabel,
  showOutline,
  showSources,
  onToggleOutline,
  onToggleSources,
  onBack,
  onRefresh,
  scrolled,
}: {
  title: string;
  updatedLabel?: string;
  showOutline: boolean;
  showSources: boolean;
  onToggleOutline: () => void;
  onToggleSources: () => void;
  onBack: () => void;
  onRefresh: () => void;
  /** When the document has scrolled, the bar lifts off the page (border + shadow). */
  scrolled?: boolean;
}) {
  return (
    <header className="chrome" data-scrolled={scrolled ? "" : undefined}>
      <div className="chrome-left">
        <button
          type="button"
          className="chrome-btn"
          aria-label="Back to streams"
          onClick={onBack}
        >
          ‹
        </button>
        <button
          type="button"
          className="chrome-btn"
          aria-label="Toggle outline"
          aria-pressed={showOutline}
          onClick={onToggleOutline}
        >
          ☰
        </button>
      </div>

      {/*
        The Document column already renders the canonical <h1> title. This
        compact bar title is contextual chrome, so we split it into per-word
        spans: screen readers still read "AI Agents", text stays selectable,
        and there is no second DOM node whose text equals the whole title to
        compete with the Document's heading.
      */}
      <p className="chrome-title" aria-label={title}>
        {title.split(" ").map((word, i) => (
          <span key={i} className="chrome-title-word">
            {word}
          </span>
        ))}
      </p>

      <div className="chrome-right">
        {updatedLabel ? <span className="chrome-updated">{updatedLabel}</span> : null}
        <button
          type="button"
          className="chrome-btn"
          aria-label="Refresh"
          onClick={onRefresh}
        >
          ↻
        </button>
        <button
          type="button"
          className="chrome-btn"
          aria-label="Toggle sources"
          aria-pressed={showSources}
          onClick={onToggleSources}
        >
          ⌗
        </button>
      </div>
    </header>
  );
}
