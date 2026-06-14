import { useEffect, useMemo, useRef, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import "./MyNotes.css";

/**
 * A calm markdown highlight for the notes editor: headings are lightly bolded in
 * ink (no underline, no big size jumps) so typing `#` reads as emphasis, not a
 * competing document title. The notes are the user's quiet space, not a doc.
 */
const calmHeading = { fontWeight: "650", color: "var(--ink)", textDecoration: "none" };
const notesHighlight = HighlightStyle.define([
  { tag: tags.heading1, ...calmHeading, fontSize: "1.08em" },
  { tag: tags.heading2, ...calmHeading },
  { tag: tags.heading3, ...calmHeading },
  { tag: tags.heading, ...calmHeading },
  { tag: tags.strong, fontWeight: "700", color: "var(--ink)" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.link, color: "var(--accent)", textDecoration: "underline" },
  { tag: tags.url, color: "var(--muted-2)" },
  { tag: tags.monospace, fontFamily: "var(--mono)", fontSize: "0.9em", color: "var(--ink)" },
  { tag: [tags.list, tags.contentSeparator], color: "var(--muted-2)" },
  { tag: tags.quote, color: "var(--muted)", fontStyle: "italic" },
  { tag: tags.processingInstruction, color: "var(--muted-2)" },
]);

type Mode = "write" | "preview";

/**
 * The user-owned "My notes" surface. A Write/Preview toggle switches between a
 * CodeMirror 6 styled-source editor (Obsidian's engine — lossless, the bytes
 * round-trip unchanged) and a rendered markdown preview. On blur from the
 * editor, if the text changed, onSave gets the full block reconstructed as
 * "## My notes\n" + text (the bridge's saveNotes expects this prefix).
 */
export function MyNotes({
  markdown: md,
  onSave,
}: {
  markdown: string;
  onSave: (block: string) => void;
}) {
  const [value, setValue] = useState(md);
  const [mode, setMode] = useState<Mode>("write");
  const baselineRef = useRef(md);
  const valueRef = useRef(md);

  // Keep local state in sync if the source markdown changes (e.g. doc swap).
  useEffect(() => {
    setValue(md);
    baselineRef.current = md;
    valueRef.current = md;
  }, [md]);

  const save = () => {
    if (valueRef.current === baselineRef.current) return;
    baselineRef.current = valueRef.current;
    onSave(`## My notes\n${valueRef.current}`);
  };

  // Switching to preview should persist any pending edit, just like a blur.
  const switchMode = (next: Mode) => {
    if (next === "preview") save();
    setMode(next);
  };

  const extensions = useMemo(
    () => [
      markdown(),
      syntaxHighlighting(notesHighlight),
      EditorView.lineWrapping,
      EditorView.contentAttributes.of({ "aria-label": "My notes" }),
      EditorView.theme({
        "&": {
          fontFamily: "var(--serif)",
          fontSize: "1.02rem",
          color: "var(--ink)",
          backgroundColor: "transparent",
        },
        "&.cm-focused": { outline: "none" },
        ".cm-content": { padding: "0.2rem 0", caretColor: "var(--accent)" },
        ".cm-line": { padding: "0" },
        ".cm-cursor": { borderLeftColor: "var(--accent)" },
        "&.cm-editor .cm-selectionBackground, & .cm-selectionBackground, & ::selection":
          { backgroundColor: "var(--accent-tint)" },
        ".cm-placeholder": { color: "var(--muted-2)" },
      }),
    ],
    [],
  );

  return (
    <div className="my-notes">
      <div className="my-notes-rail">
        <div className="my-notes-heading">
          <h2 id="my-notes" className="my-notes-label">
            My notes
          </h2>
          <p className="my-notes-hint">Your space — private, never sent to the agent.</p>
        </div>
        <div className="my-notes-tabs" role="tablist" aria-label="Notes mode">
          <button
            type="button"
            role="tab"
            aria-selected={mode === "write"}
            data-active={mode === "write" ? "" : undefined}
            onClick={() => switchMode("write")}
          >
            Write
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={mode === "preview"}
            data-active={mode === "preview" ? "" : undefined}
            onClick={() => switchMode("preview")}
          >
            Preview
          </button>
        </div>
      </div>

      {mode === "write" ? (
        <div className="my-notes-editor">
          <CodeMirror
            value={value}
            onChange={(next) => {
              valueRef.current = next;
              setValue(next);
            }}
            onBlur={save}
            extensions={extensions}
            basicSetup={{
              lineNumbers: false,
              foldGutter: false,
              highlightActiveLine: false,
              highlightActiveLineGutter: false,
              drawSelection: true,
              allowMultipleSelections: false,
              searchKeymap: false,
            }}
            placeholder="Write your own notes here…"
            indentWithTab={false}
          />
        </div>
      ) : (
        <div className="my-notes-preview document-body" aria-label="My notes preview">
          {value.trim() ? (
            <Markdown remarkPlugins={[remarkGfm]}>{value}</Markdown>
          ) : (
            <p className="my-notes-empty">Nothing here yet.</p>
          )}
        </div>
      )}
    </div>
  );
}
