import "@testing-library/jest-dom";

// jsdom lacks ResizeObserver, which Radix UI primitives (e.g. Popover) rely on.
if (!("ResizeObserver" in globalThis)) {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}
