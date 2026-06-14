import { render, screen } from "@testing-library/react";
import { Document } from "./Document";
import { parseDoc } from "../../lib/parseDoc";
import { SAMPLE_DOC } from "../../bridge/sampleData";

function renderDoc() {
  const doc = parseDoc(SAMPLE_DOC);
  return render(<Document doc={doc} onSaveNotes={() => {}} />);
}

test("renders the title", () => {
  renderDoc();
  expect(screen.getByRole("heading", { level: 1, name: "AI Agents" })).toBeInTheDocument();
});

test("renders the four movement labels", () => {
  renderDoc();
  expect(screen.getByRole("heading", { name: /what changed/i })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: /current understanding/i })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: /open questions/i })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: /my notes/i })).toBeInTheDocument();
});

test("the What changed label carries data-accent", () => {
  renderDoc();
  const label = screen.getByRole("heading", { name: /what changed/i });
  expect(label).toHaveAttribute("data-accent");
});

test("renders a subsection heading", () => {
  renderDoc();
  expect(screen.getByRole("heading", { name: "Durable execution" })).toBeInTheDocument();
});

test("renders a citation chip somewhere", () => {
  renderDoc();
  expect(screen.getAllByRole("button").length).toBeGreaterThan(0);
});

test("renders a My notes textarea", () => {
  renderDoc();
  expect(screen.getByRole("textbox", { name: /my notes/i })).toBeInTheDocument();
});
