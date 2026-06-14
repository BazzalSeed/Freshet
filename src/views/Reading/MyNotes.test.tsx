import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MyNotes } from "./MyNotes";

test("renders markdown in a textarea and saves a prefixed block on blur", async () => {
  const onSave = vi.fn();
  render(<MyNotes markdown={"- existing note"} onSave={onSave} />);

  const textarea = screen.getByRole("textbox", { name: /my notes/i });
  expect(textarea).toHaveValue("- existing note");

  await userEvent.click(textarea);
  await userEvent.type(textarea, "\n- new note");
  await userEvent.tab(); // blur

  expect(onSave).toHaveBeenCalledTimes(1);
  const block = onSave.mock.calls[0][0] as string;
  expect(block.startsWith("## My notes")).toBe(true);
  expect(block).toContain("new note");
});

test("does not call onSave when content is unchanged", async () => {
  const onSave = vi.fn();
  render(<MyNotes markdown={"- existing note"} onSave={onSave} />);
  const textarea = screen.getByRole("textbox", { name: /my notes/i });
  await userEvent.click(textarea);
  await userEvent.tab();
  expect(onSave).not.toHaveBeenCalled();
});
