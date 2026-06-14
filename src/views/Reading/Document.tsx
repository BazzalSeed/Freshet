import { useMemo, type ReactNode } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { Components } from "react-markdown";
import type { ParsedDoc, Citation as CitationType } from "../../lib/parseDoc";
import { slugify } from "../../lib/parseDoc";
import { Citation } from "./Citation";
import { MyNotes } from "./MyNotes";
import "./Document.css";

/** Flatten a heading's children (usually a single string) into plain text. */
function nodeText(children: ReactNode): string {
  if (typeof children === "string") return children;
  if (Array.isArray(children)) return children.map(nodeText).join("");
  if (children == null || typeof children === "boolean") return "";
  return String(children);
}

/**
 * GFM renders a footnote ref as `<a href="#…fn-<id>" data-footnote-ref>`.
 * Recover the original `[^id]` identifier from that href.
 */
function footnoteIdFromHref(href?: string): string | null {
  if (!href) return null;
  const m = href.match(/#.*?fn-(.+)$/);
  return m ? decodeURIComponent(m[1]) : null;
}

/**
 * The reading view's document column. The Freshet-owned movements render through
 * react-markdown + remark-gfm (real bold/italic/code/lists/links), with inline
 * `[^id]` footnotes swapped for subtle citation chips and the auto footnote
 * section suppressed. The title/updated header and the user-owned My notes editor
 * sit outside that markdown pass.
 */
export function Document({
  doc,
  title,
  onSaveNotes,
  onOpenUrl,
  onCite,
}: {
  doc: ParsedDoc;
  /** Authoritative title from the stream description (falls back to the parsed one). */
  title?: string;
  onSaveNotes: (block: string) => void;
  onOpenUrl: (url: string) => void;
  /** A citation marker was clicked — reveal/highlight its source. */
  onCite: (citationId: string) => void;
}) {
  const sourcesById = useMemo(() => {
    const m = new Map<string, CitationType>();
    for (const s of doc.sources) m.set(s.id, s);
    return m;
  }, [doc.sources]);

  const components: Components = useMemo(
    () => ({
      h2({ children }) {
        const text = nodeText(children).trim();
        return (
          <h2
            id={slugify(text)}
            className="section-label"
            {...(text === "What changed" ? { "data-accent": "" } : {})}
          >
            {children}
          </h2>
        );
      },
      h3({ children }) {
        const text = nodeText(children).trim();
        return (
          <h3 id={slugify(text)} className="subsection-heading">
            {children}
          </h3>
        );
      },
      a({ href, children, node }) {
        const isFootnote = Boolean(
          (node?.properties as Record<string, unknown> | undefined)?.dataFootnoteRef,
        );
        if (isFootnote) {
          const id = footnoteIdFromHref(href);
          const citation = id ? sourcesById.get(id) : undefined;
          if (citation) {
            return <Citation citation={citation} label={children} onCite={onCite} />;
          }
          // Unknown footnote id: drop the bare number rather than show a dead link.
          return <>{children}</>;
        }
        return (
          <a
            href={href}
            className="doc-link"
            onClick={(e) => {
              e.preventDefault();
              if (href) onOpenUrl(href);
            }}
          >
            {children}
          </a>
        );
      },
      // Suppress GFM's auto-appended footnote section; chips carry the info.
      section({ node, children, ...props }) {
        if ((node?.properties as Record<string, unknown> | undefined)?.dataFootnotes) {
          return null;
        }
        return <section {...props}>{children}</section>;
      },
    }),
    [sourcesById, onOpenUrl, onCite],
  );

  return (
    <article className="document">
      <h1 className="document-title">{title ?? doc.title}</h1>
      {doc.updatedLabel ? <p className="document-updated">{doc.updatedLabel}</p> : null}

      <div className="document-body">
        <Markdown remarkPlugins={[remarkGfm]} components={components}>
          {doc.bodyMarkdown}
        </Markdown>
      </div>

      <section className="my-notes-section" aria-label="My notes section">
        <MyNotes markdown={doc.myNotes} onSave={onSaveNotes} />
      </section>
    </article>
  );
}
