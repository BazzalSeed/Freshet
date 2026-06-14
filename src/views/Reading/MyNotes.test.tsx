import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MyNotes } from "./MyNotes";

// CodeMirror renders a contenteditable (role="textbox"), not a real <textarea>.
// Interactive editing through CodeMirror is unreliable under jsdom, so we cover
// rendering + labelling here; the save-on-blur round-trip is covered by the
// parseDoc round-trip test and the Rust save_notes test.

test("renders the notes as a textbox labelled My notes", () => {
  render(<MyNotes markdown={"- existing note"} onSave={() => {}} />);
  const box = screen.getByRole("textbox", { name: /my notes/i });
  expect(box).toBeInTheDocument();
});

test("shows the initial markdown content", () => {
  render(<MyNotes markdown={"- existing note"} onSave={() => {}} />);
  expect(screen.getByText(/existing note/)).toBeInTheDocument();
});

test("Preview tab renders the notes as markdown (bold, not asterisks)", async () => {
  render(<MyNotes markdown={"a **bold** note"} onSave={() => {}} />);
  await userEvent.click(screen.getByRole("tab", { name: /preview/i }));
  expect(screen.getByText("bold").tagName).toBe("STRONG");
  // The editor textbox is gone in preview mode.
  expect(screen.queryByRole("textbox", { name: /my notes/i })).not.toBeInTheDocument();
});
