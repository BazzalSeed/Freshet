import { render, screen } from "@testing-library/react";
import { Document } from "./Document";
import { parseDoc } from "../../lib/parseDoc";
import { SAMPLE_DOC } from "../../bridge/sampleData";

function renderDoc() {
  const doc = parseDoc(SAMPLE_DOC);
  return render(
    <Document doc={doc} onSaveNotes={() => {}} onOpenUrl={() => {}} onCite={() => {}} />,
  );
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

test("renders inline markdown (bold) rather than literal asterisks", () => {
  const doc = parseDoc(
    "# T\n\n## What changed\n- Anthropic shipped **the SDK** today.\n",
  );
  render(
    <Document doc={doc} onSaveNotes={() => {}} onOpenUrl={() => {}} onCite={() => {}} />,
  );
  // The bold text renders as a <strong>, and no raw ** survives.
  expect(screen.getByText("the SDK").tagName).toBe("STRONG");
  expect(screen.queryByText(/\*\*/)).not.toBeInTheDocument();
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

test("renders citation markers as buttons (chips)", () => {
  renderDoc();
  expect(screen.getAllByRole("button").length).toBeGreaterThan(0);
});

test("renders a My notes editor textbox", () => {
  renderDoc();
  expect(screen.getByRole("textbox", { name: /my notes/i })).toBeInTheDocument();
});
