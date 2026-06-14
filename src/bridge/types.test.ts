/**
 * Tests for asFreshetError() — the parse helper for Tauri invoke errors.
 */
import { describe, it, expect } from "vitest";
import { asFreshetError } from "./types";
import type { FreshetError } from "./types";

describe("asFreshetError", () => {
  it("parses a well-formed FreshetError object", () => {
    const raw: FreshetError = {
      code: "not_logged_in",
      message: "Not logged in.",
      hint: "Run /login",
    };
    const result = asFreshetError(raw);
    expect(result.code).toBe("not_logged_in");
    expect(result.message).toBe("Not logged in.");
    expect(result.hint).toBe("Run /login");
  });

  it("parses a FreshetError object without hint", () => {
    const raw = { code: "no_sources", message: "No results." };
    const result = asFreshetError(raw);
    expect(result.code).toBe("no_sources");
    expect(result.message).toBe("No results.");
    expect(result.hint).toBeUndefined();
  });

  it("falls back to agent_failed for a plain string", () => {
    const result = asFreshetError("something went wrong");
    expect(result.code).toBe("agent_failed");
    expect(result.message).toBe("something went wrong");
  });

  it("falls back to agent_failed for an Error object", () => {
    const result = asFreshetError(new Error("network fail"));
    expect(result.code).toBe("agent_failed");
    expect(result.message).toBe("network fail");
  });

  it("falls back gracefully for null", () => {
    const result = asFreshetError(null);
    expect(result.code).toBe("agent_failed");
    expect(result.message).toBeTruthy();
  });

  it("falls back gracefully for undefined", () => {
    const result = asFreshetError(undefined);
    expect(result.code).toBe("agent_failed");
    expect(result.message).toBeTruthy();
  });

  it("falls back gracefully for a number", () => {
    const result = asFreshetError(42);
    expect(result.code).toBe("agent_failed");
  });

  it("falls back gracefully for object missing message", () => {
    const result = asFreshetError({ code: "not_logged_in" });
    // Missing message — falls back to generic
    expect(result.code).toBe("agent_failed");
  });
});
