import { useEffect, useRef, useState } from "react";
import "./MyNotes.css";

/**
 * Editable "My notes" block. Renders the raw body under `## My notes` as a
 * textarea. On blur, if changed, calls onSave with the FULL block reconstructed
 * as "## My notes\n" + currentText (the bridge's saveNotes expects this prefix).
 */
export function MyNotes({
  markdown,
  onSave,
}: {
  markdown: string;
  onSave: (block: string) => void;
}) {
  const [value, setValue] = useState(markdown);
  // Track the last-saved baseline so we only fire onSave on real changes.
  const baselineRef = useRef(markdown);

  // Keep local state in sync if the source markdown changes (e.g. doc swap).
  useEffect(() => {
    setValue(markdown);
    baselineRef.current = markdown;
  }, [markdown]);

  const handleBlur = () => {
    if (value === baselineRef.current) return;
    baselineRef.current = value;
    onSave(`## My notes\n${value}`);
  };

  return (
    <textarea
      className="my-notes-textarea"
      aria-label="My notes"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onBlur={handleBlur}
      rows={4}
      spellCheck={false}
    />
  );
}
