import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { Chrome } from "./Chrome";

function setup(overrides: Partial<React.ComponentProps<typeof Chrome>> = {}) {
  const props = {
    title: "AI Agents",
    updatedLabel: "updated 2 days ago",
    showOutline: false,
    showSources: false,
    onToggleOutline: vi.fn(),
    onToggleSources: vi.fn(),
    onBack: vi.fn(),
    onRefresh: vi.fn(),
    ...overrides,
  };
  render(<Chrome {...props} />);
  return props;
}

test("renders the title", () => {
  setup();
  // The bar title splits into per-word spans (so it doesn't compete with the
  // Document's canonical <h1>); the accessible name carries the whole title.
  expect(screen.getByLabelText("AI Agents")).toBeInTheDocument();
});

test("clicking back fires onBack", async () => {
  const props = setup();
  await userEvent.click(screen.getByRole("button", { name: /back to streams/i }));
  expect(props.onBack).toHaveBeenCalled();
});

test("clicking refresh fires onRefresh", async () => {
  const props = setup();
  await userEvent.click(screen.getByRole("button", { name: /refresh/i }));
  expect(props.onRefresh).toHaveBeenCalled();
});

test("outline toggle reflects showOutline via aria-pressed", () => {
  setup({ showOutline: true });
  expect(screen.getByRole("button", { name: /toggle outline/i })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("outline toggle has no visible Outline text", () => {
  setup();
  expect(screen.queryByText("Outline")).not.toBeInTheDocument();
});
