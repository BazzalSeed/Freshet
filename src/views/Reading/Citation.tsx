import * as Popover from "@radix-ui/react-popover";
import type { Citation as CitationType } from "../../lib/parseDoc";
import "./Citation.css";

const SOURCE_LABELS: Record<string, string> = {
  hackernews: "HN",
  reddit: "Reddit",
  github: "GitHub",
  polymarket: "Polymarket",
};

function sourceLabel(source: string): string {
  return SOURCE_LABELS[source] ?? source;
}

export function Citation({ citation }: { citation: CitationType }) {
  const { source, title, score, date, url } = citation;
  const chipLabel = score !== undefined ? `${sourceLabel(source)} ${score}` : sourceLabel(source);

  const meta = [source, score !== undefined ? String(score) : null, date]
    .filter((part): part is string => Boolean(part))
    .join(" · ");

  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <button type="button" className="citation-chip">
          {chipLabel}
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content className="citation-popover" sideOffset={4} collisionPadding={8}>
          <div className="citation-title">{title}</div>
          <div className="citation-meta">{meta}</div>
          <a
            className="citation-link"
            href={url}
            target="_blank"
            rel="noreferrer"
          >
            open ↗
          </a>
          <Popover.Arrow className="citation-arrow" />
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
