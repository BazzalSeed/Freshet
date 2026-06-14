//! Reconcile helpers for the watch engine.
//!
//! The agent never sees the user-owned `## My notes` block. Before handing the
//! prior document to `Agent::synthesize`, the engine splits it into the
//! Freshet-owned part and the (optional) My-notes block. After synthesis it
//! re-attaches the My-notes block byte-for-byte via [`store::document::splice_my_notes`].

/// The sentinel that marks the start of the user-owned section.
///
/// Mirrors `store::document::splice_my_notes`: the boundary is the first
/// occurrence of `\n## My notes`.
const MY_NOTES_MARKER: &str = "\n## My notes";

/// Split `doc` into its Freshet-owned prefix and the (optional) `## My notes` block.
///
/// Returns `(freshet_owned, my_notes_block)`:
/// - `freshet_owned` is `doc[..pos]` — everything before the `\n` that precedes
///   `## My notes` (byte-identical, including any `[^id]:` footnote definitions).
/// - `my_notes_block` is `Some(doc[pos+1..])` — from `## My notes` onward (the
///   leading `\n` of the marker is dropped), suitable to pass straight back to
///   `splice_my_notes`. `None` if the document has no `## My notes` section.
///
/// This mirrors `splice_my_notes`'s boundary so a split → splice round-trip is
/// lossless: `splice_my_notes(freshet_owned, &block)` reproduces the original.
pub fn extract_my_notes(doc: &str) -> (String, Option<String>) {
    match doc.find(MY_NOTES_MARKER) {
        Some(pos) => {
            let freshet_owned = doc[..pos].to_string();
            // Drop the single leading `\n` of the marker; keep `## My notes…`.
            let block = doc[pos + 1..].to_string();
            (freshet_owned, Some(block))
        }
        None => (doc.to_string(), None),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::document::splice_my_notes;

    #[test]
    fn extract_splits_at_my_notes_boundary() {
        let doc = concat!(
            "# AI Weekly\n",
            "\n",
            "## What changed\n",
            "\n",
            "- GPT-5 released.[^hn1]\n",
            "\n",
            "[^hn1]: https://example.com\n",
            "\n",
            "## My notes\n",
            "\n",
            "My private thought.\n",
        );

        let (owned, block) = extract_my_notes(doc);

        // Owned prefix keeps everything up to (not including) the \n before ## My notes.
        assert!(owned.contains("## What changed"));
        assert!(owned.contains("[^hn1]: https://example.com"));
        assert!(!owned.contains("## My notes"), "owned must not include notes header");
        assert!(!owned.contains("My private thought."), "owned must not include note body");

        // Block starts exactly at the header and contains the note.
        let block = block.expect("notes block present");
        assert!(block.starts_with("## My notes"));
        assert!(block.contains("My private thought."));
    }

    #[test]
    fn extract_returns_none_when_no_notes() {
        let doc = "# Doc\n\n## What changed\n\n- nothing.\n";
        let (owned, block) = extract_my_notes(doc);
        assert_eq!(owned, doc);
        assert!(block.is_none());
    }

    /// Split then splice must reproduce the original byte-for-byte.
    #[test]
    fn extract_then_splice_round_trips() {
        let doc = concat!(
            "# Doc\n",
            "\n",
            "Body.[^r1]\n",
            "\n",
            "[^r1]: https://example.com\n",
            "\n",
            "## My notes\n",
            "\n",
            "mine\n",
        );

        let (owned, block) = extract_my_notes(doc);
        let block = block.expect("notes present");
        let rebuilt = splice_my_notes(&owned, &block);
        assert_eq!(rebuilt, doc, "split→splice must be lossless");
    }
}
