//! Document I/O and the model-B "My notes" splice.

use std::path::Path;

use super::{doc_path, read_to_string_opt, write_atomic};

// ── Document I/O ─────────────────────────────────────────────────────────────

/// Read the markdown document for `title` from `<root>/<title>.md`.
/// Returns `None` if the file does not exist.
pub fn read_document(root: &Path, title: &str) -> Option<String> {
    read_to_string_opt(&doc_path(root, title))
}

/// Write `contents` to `<root>/<title>.md` atomically.
pub fn write_document(root: &Path, title: &str, contents: &str) -> anyhow::Result<()> {
    write_atomic(&doc_path(root, title), contents)
}

// ── My-notes splice ───────────────────────────────────────────────────────────

/// The sentinel that marks the start of the user-owned section.
const MY_NOTES_MARKER: &str = "\n## My notes";

/// Replace **only** the `## My notes` section of `doc` with `new_my_notes_block`.
///
/// `new_my_notes_block` must start with `## My notes` (no leading newline).
///
/// The rule: find the first occurrence of `\n## My notes` in `doc`.  Everything
/// before that newline (the Freshet-owned prefix, including any `[^id]:` footnote
/// definitions) is kept **byte-identical**.  Everything from `\n## My notes`
/// onward is replaced with `\n` + `new_my_notes_block`.
///
/// If `## My notes` is absent, the block is appended after a blank line.
pub fn splice_my_notes(doc: &str, new_my_notes_block: &str) -> String {
    match doc.find(MY_NOTES_MARKER) {
        Some(pos) => {
            // `pos` is the index of the `\n` that precedes `## My notes`.
            // Keep doc[..pos] (the prefix, not including that `\n`) then add
            // `\n` + new block.
            let mut out = String::with_capacity(pos + 1 + new_my_notes_block.len());
            out.push_str(&doc[..pos]);
            out.push('\n');
            out.push_str(new_my_notes_block);
            out
        }
        None => {
            // Append: ensure exactly one blank line between existing content and
            // the new block, then append.
            let mut out = String::with_capacity(doc.len() + 2 + new_my_notes_block.len());
            out.push_str(doc);
            // If doc doesn't end with two newlines, add them for a blank line.
            if !out.ends_with("\n\n") {
                if out.ends_with('\n') {
                    out.push('\n');
                } else {
                    out.push_str("\n\n");
                }
            }
            out.push_str(new_my_notes_block);
            out
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    // ── read/write document ────────────────────────────────────────────────

    #[test]
    fn write_read_document_round_trips() {
        let dir = tmp();
        write_document(dir.path(), "Rust news", "# Rust news\n\ncontent").expect("write");
        let loaded = read_document(dir.path(), "Rust news").expect("should exist");
        assert_eq!(loaded, "# Rust news\n\ncontent");
    }

    #[test]
    fn read_document_returns_none_when_absent() {
        let dir = tmp();
        assert!(read_document(dir.path(), "nonexistent").is_none());
    }

    // ── splice_my_notes ────────────────────────────────────────────────────

    /// Full doc with Freshet content, footnote defs, then My notes.
    /// Splicing keeps prefix (incl. footnote defs) byte-identical.
    #[test]
    fn splice_replaces_only_notes_section() {
        let original = concat!(
            "# AI Weekly\n",
            "\n",
            "## What changed\n",
            "\n",
            "- GPT-5 released.[^hn1]\n",
            "\n",
            "[^hn1]: https://news.ycombinator.com/item?id=1\n",
            "\n",
            "## My notes\n",
            "\n",
            "Old note here.\n",
        );

        let new_notes = "## My notes\n\nNew note here.\n";

        let result = splice_my_notes(original, new_notes);

        // The prefix must be byte-identical: everything up to (not including)
        // the \n that precedes "## My notes".
        let expected_prefix = concat!(
            "# AI Weekly\n",
            "\n",
            "## What changed\n",
            "\n",
            "- GPT-5 released.[^hn1]\n",
            "\n",
            "[^hn1]: https://news.ycombinator.com/item?id=1\n",
        );
        // The \n before ## My notes is part of the boundary; the prefix ends before it.
        assert!(
            result.starts_with(expected_prefix),
            "prefix mismatch.\nExpected prefix:\n{expected_prefix:?}\nGot:\n{result:?}"
        );

        // Footnote def must still be present and byte-identical.
        assert!(
            result.contains("[^hn1]: https://news.ycombinator.com/item?id=1\n"),
            "footnote def missing or altered: {result:?}"
        );

        // Old note must be gone.
        assert!(!result.contains("Old note here."), "old note still present: {result:?}");

        // New note must be present.
        assert!(result.contains("New note here."), "new note missing: {result:?}");

        // Section header must appear exactly once.
        assert_eq!(
            result.matches("## My notes").count(),
            1,
            "expected exactly one ## My notes: {result:?}"
        );
    }

    /// Verify the prefix is byte-for-byte identical by direct byte comparison.
    #[test]
    fn splice_prefix_is_byte_identical() {
        let original = concat!(
            "# Doc\n",
            "\n",
            "Body text.\n",
            "\n",
            "[^ref1]: https://example.com\n",
            "\n",
            "## My notes\n",
            "\n",
            "old\n",
        );

        // The prefix is everything before the "\n## My notes".
        // "\n## My notes" first appears at the position after the blank line following [^ref1].
        let marker_pos = original.find("\n## My notes").unwrap();
        let expected_prefix = &original[..marker_pos];

        let new_notes = "## My notes\n\nnew\n";
        let result = splice_my_notes(original, new_notes);

        assert!(result.starts_with(expected_prefix));
        // Byte-identical check
        assert_eq!(result[..marker_pos].as_bytes(), expected_prefix.as_bytes());
    }

    /// When ## My notes is absent, the block is appended after a blank line.
    #[test]
    fn splice_appends_when_absent() {
        let doc = "# Rust news\n\nSome content.\n";
        let new_notes = "## My notes\n\nFirst note.\n";

        let result = splice_my_notes(doc, new_notes);

        // Original content preserved.
        assert!(result.starts_with("# Rust news\n\nSome content.\n"));
        // New section appended.
        assert!(result.contains("## My notes"), "header missing: {result:?}");
        assert!(result.contains("First note."), "note missing: {result:?}");
        // Blank line separating.
        assert!(
            result.contains("Some content.\n\n## My notes"),
            "blank line separator missing: {result:?}"
        );
    }

    /// Append when doc doesn't end with a newline.
    #[test]
    fn splice_appends_no_trailing_newline() {
        let doc = "# Doc\n\nContent.";
        let new_notes = "## My notes\n\nNote.\n";
        let result = splice_my_notes(doc, new_notes);
        assert!(result.contains("Content.\n\n## My notes"));
    }

    /// Append when doc ends with exactly one newline.
    #[test]
    fn splice_appends_single_trailing_newline() {
        let doc = "# Doc\n\nContent.\n";
        let new_notes = "## My notes\n\nNote.\n";
        let result = splice_my_notes(doc, new_notes);
        assert!(result.contains("Content.\n\n## My notes"));
    }
}
