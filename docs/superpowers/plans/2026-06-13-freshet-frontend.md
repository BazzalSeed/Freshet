# Freshet Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Aesthetic polish is NOT in these steps.** The tasks build correct, tested structure + behavior against mock data. The look-and-feel pass (spacing, micro-interactions, the reconcile motion) is iterated separately with the **frontend-design** skill against `docs/superpowers/specs/2026-06-13-frontend-feel-design.md`. Don't try to pixel-tune in TDD steps.

**Goal:** A functional, tested Freshet frontend — the reading view (3-zone collapsible shell), quiet-desk home, and creation form — running in the browser against a `MockBridge`, styled with the locked design tokens.

**Architecture:** React + Vite + TS SPA inside a Tauri 2 scaffold (the Rust shell is scaffolded but idle this phase). The UI talks to a `Bridge` interface; a `MockBridge` supplies sample data so the whole UI is built and tested with zero backend. A small parser turns the living-document markdown into the structure the reading view renders. Theme via CSS custom properties + `prefers-color-scheme`.

**Tech Stack:** Tauri 2, React 18, Vite, TypeScript, vitest + @testing-library/react + jsdom, Radix primitives (popover), @fontsource (self-hosted Newsreader + IBM Plex Mono).

**Sources of truth:** frontend brief `docs/superpowers/specs/2026-06-13-frontend-feel-design.md` (tokens, type, layout); product spec `docs/superpowers/specs/2026-06-12-freshet-v1-design.md` (bridge §5.5, document schema §5.2, model B). **Backend/Rust/agent work is out of scope — later phase.**

---

## File structure

```
index.html · package.json · vite.config.ts · vitest.config.ts · tsconfig.json
src-tauri/…                         # scaffolded, idle this phase
src/
  main.tsx                          # entry; imports fonts + styles
  App.tsx                           # routes Desk ⇄ Reading ⇄ Create
  styles/
    tokens.css                      # CSS vars: light + [data-theme="dark"]
    base.css                        # reset, body, type defaults
  theme/useTheme.ts                 # prefers-color-scheme → data-theme + manual toggle
  bridge/
    types.ts                        # TS contract types (UI subset of spec §5)
    Bridge.ts                       # Bridge interface
    MockBridge.ts                   # in-memory impl + sample data
    sampleData.ts                   # sample streams + sample living-document markdown
    BridgeProvider.tsx              # React context
  lib/parseDoc.ts                   # markdown → ParsedDoc (movements, outline, sources)
  views/
    Reading/Reading.tsx             # 3-zone shell + collapse/expand state
    Reading/Chrome.tsx              # top bar
    Reading/Outline.tsx             # left rail
    Reading/Sources.tsx             # right panel
    Reading/Document.tsx            # center column (movements)
    Reading/Cited.tsx               # renders text with [^id] → Citation
    Reading/Citation.tsx            # inline chip + Radix popover
    Reading/MyNotes.tsx             # editable notes block
    Desk/Desk.tsx · Desk/StreamRow.tsx
    Create/Create.tsx
```

---

## Phase 0 — Scaffold

### Task 0.1: Scaffold Tauri 2 + React/Vite/TS + vitest

**Files:** create the project skeleton (above).

- [ ] **Step 1:** `npm create tauri-app@latest freshet -- --template react-ts`, then move `src/`, `src-tauri/`, and config files to the repo root (this repo already holds `docs/`).
- [ ] **Step 2:** `npm i -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom` and `npm i @radix-ui/react-popover @fontsource-variable/newsreader @fontsource/ibm-plex-mono`.
- [ ] **Step 3:** Create `vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
export default defineConfig({
  test: { environment: "jsdom", globals: true, setupFiles: "./src/test-setup.ts" },
});
```

and `src/test-setup.ts` with `import "@testing-library/jest-dom";`. Add `"test": "vitest"` to `package.json` scripts.

- [ ] **Step 4:** Run `npm run dev` → confirm the default app renders at `http://localhost:5173`. Run `npm run test` → confirm vitest runs (0 tests).
- [ ] **Step 5:** Commit: `git commit -m "chore: scaffold Tauri 2 + React/Vite + vitest"`.

---

## Phase 1 — Foundation: tokens, fonts, theme

### Task 1.1: Design tokens + base styles

**Files:** Create `src/styles/tokens.css`, `src/styles/base.css`; modify `src/main.tsx`.

- [ ] **Step 1:** Create `src/styles/tokens.css` with the exact tokens from the brief §2:

```css
:root {
  --bg:#f4ecdd; --surface:#ebe1cf; --surface-2:#f7f0e3;
  --ink:#2b2620; --fg:#352d25; --muted:#8a7a63; --muted-2:#a8967b;
  --rule:#e0d6c4; --accent:#9c5b33; --accent-tint:rgba(156,91,51,.11);
  --serif:'Newsreader Variable',Georgia,serif; --mono:'IBM Plex Mono',ui-monospace,monospace;
  color-scheme:light;
}
[data-theme="dark"] {
  --bg:#0d1117; --surface:#161b22; --surface-2:#161b22;
  --ink:#e6edf3; --fg:#c9d1d9; --muted:#8b949e; --muted-2:#8b949e;
  --rule:#21262d; --accent:#3fb950; --accent-tint:rgba(63,185,80,.13);
  color-scheme:dark;
}
```

- [ ] **Step 2:** Create `src/styles/base.css` (reset + body using `--bg`/`--fg`/`--serif`, antialiasing, `@media (prefers-reduced-motion: reduce)` disabling transitions).
- [ ] **Step 3:** In `src/main.tsx` import, in order: `@fontsource-variable/newsreader`, `@fontsource/ibm-plex-mono`, `./styles/tokens.css`, `./styles/base.css`.
- [ ] **Step 4:** Run `npm run dev` → body shows cream bg + serif text. **Step 5:** Commit `"feat: design tokens, self-hosted fonts, base styles"`.

### Task 1.2: Theme hook (system-adaptive + toggle)

**Files:** Create `src/theme/useTheme.ts`, `src/theme/useTheme.test.ts`.

- [ ] **Step 1: Failing test:**

```ts
import { renderHook, act } from "@testing-library/react";
import { useTheme } from "./useTheme";

function mockMatchMedia(matchesDark: boolean) {
  window.matchMedia = (q: string) => ({
    matches: q.includes("dark") ? matchesDark : false,
    media: q, addEventListener(){}, removeEventListener(){},
    addListener(){}, removeListener(){}, onchange:null, dispatchEvent:()=>false,
  }) as MediaQueryList;
}

test("follows system dark and toggles", () => {
  mockMatchMedia(true);
  const { result } = renderHook(() => useTheme());
  expect(result.current.theme).toBe("dark");
  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  act(() => result.current.toggle());
  expect(result.current.theme).toBe("light");
});
```

- [ ] **Step 2:** Run → FAIL. **Step 3:** Implement `useTheme()` — read `matchMedia("(prefers-color-scheme: dark)")`, set `document.documentElement.dataset.theme`, expose `{ theme, toggle }`, persist override to `localStorage`. **Step 4:** Run → PASS. **Step 5:** Commit.

---

## Phase 2 — Bridge, mock, sample data

### Task 2.1: Contract types

**Files:** Create `src/bridge/types.ts`.

- [ ] **Step 1:** Define the UI's view of the contracts (spec §5; camelCase across the bridge):

```ts
export type CadenceMode = "manual" | "on_launch" | "interval";
export interface Cadence { mode: CadenceMode; intervalMinutes?: number }
export type StreamStatus = "active" | "paused" | "retired";
export interface StreamSummary { id:string; title:string; lastCheckedAt?:string; changedSinceSeen:boolean }
export interface StreamDescription { id:string; title:string; topic:string; sources:string[]; cadence:Cadence; status:StreamStatus; createdAt:string }
export interface GetStreamResult { description:StreamDescription; documentMarkdown:string; lastCheckedAt?:string }
export interface Summary { changed:boolean; nNew:number }
export interface DraftInput { topic:string; sources:string[]; cadence:Cadence }
export interface DraftResult { draftMarkdown:string; proposedDescription:StreamDescription }
export type RefreshPhase = "detecting"|"researching"|"synthesizing"|"done"|"error";
export interface RefreshProgress { streamId:string; phase:RefreshPhase }
export const FREE_SOURCES = ["reddit","hackernews","github","polymarket"] as const;
```

- [ ] **Step 2:** `npx tsc --noEmit` → clean. **Step 3:** Commit.

### Task 2.2: Sample data (streams + a real sample living document)

**Files:** Create `src/bridge/sampleData.ts`.

- [ ] **Step 1:** Export `sampleStreams: StreamSummary[]` (3 streams; one with `changedSinceSeen:true`) and `sampleDoc(streamId): string` returning markdown in the model-B shape (spec §5.2) with footnote citations:

```ts
export const SAMPLE_DOC = `# AI Agents
_updated 2 days ago · 4 sources_

## What changed
- Anthropic shipped the Claude Agent SDK v2 — durable workflows now survive restarts mid-task. [^hn1]
- Sentiment is turning against ReAct-style loops for production. [^r1]

## Current understanding
### Durable execution
Surviving restarts mid-task is the live frontier. [^gh1]
### Tool calling
Largely standardized; the open fight is the protocol (MCP vs. bespoke). [^hn1]

## Open questions
- Does MCP become the default tool protocol, or fragment? [^pm1]

## My notes
- Watching the MCP-vs-bespoke fight — revisit before Q3 planning.

[^hn1]: hackernews · Claude Agent SDK v2 · 412 · 2026-06-11 · https://news.ycombinator.com/item?id=1
[^r1]: reddit · Off ReAct loops in prod · 280 · 2026-06-10 · https://reddit.com/r/ml/1
[^gh1]: github · anthropics/agent-sdk v2.0 · 1200 · 2026-06-09 · https://github.com/x
[^pm1]: polymarket · MCP default by EOY · 61 · 2026-06-08 · https://polymarket.com/x
`;
```

- [ ] **Step 2:** Commit (no test; consumed by tested code below).

### Task 2.3: Bridge interface + MockBridge

**Files:** Create `src/bridge/Bridge.ts`, `src/bridge/MockBridge.ts`, `src/bridge/MockBridge.test.ts`.

- [ ] **Step 1:** `Bridge.ts` interface:

```ts
import type { StreamSummary, GetStreamResult, DraftInput, DraftResult, StreamDescription, Summary, StreamStatus, RefreshProgress } from "./types";
export interface Bridge {
  listStreams(): Promise<StreamSummary[]>;
  getStream(id: string): Promise<GetStreamResult>;
  generateFirstDraft(input: DraftInput): Promise<DraftResult>;
  createStream(desc: StreamDescription): Promise<StreamSummary>;
  refreshStream(id: string): Promise<Summary>;
  setStreamStatus(id: string, status: StreamStatus): Promise<void>;
  saveNotes(id: string, markdown: string): Promise<void>;
  onRefreshProgress(cb: (e: RefreshProgress) => void): () => void;
}
```

- [ ] **Step 2: Failing test:**

```ts
import { MockBridge } from "./MockBridge";
test("lists streams; refresh marks changed; saveNotes persists", async () => {
  const b = new MockBridge();
  const list = await b.listStreams();
  expect(list.length).toBeGreaterThan(0);
  const r = await b.refreshStream(list[0].id);
  expect(typeof r.changed).toBe("boolean");
  const after = await b.listStreams();
  expect(after.find(s => s.id === list[0].id)!.changedSinceSeen).toBe(r.changed);
  await b.saveNotes(list[0].id, "## My notes\n- edited");
  const doc = await b.getStream(list[0].id);
  expect(doc.documentMarkdown).toContain("- edited");
});
```

- [ ] **Step 3:** Run → FAIL. **Step 4:** Implement `MockBridge` over `sampleData` (in-memory maps; `refreshStream` toggles `changedSinceSeen` + bumps `lastCheckedAt`; `saveNotes` replaces the `## My notes` block in the stored markdown; `onRefreshProgress` emits `researching→synthesizing→done` via `setTimeout`, returns an unsubscribe). **Step 5:** Run → PASS. **Step 6:** Commit.

### Task 2.4: BridgeProvider context

**Files:** Create `src/bridge/BridgeProvider.tsx`; test `src/bridge/BridgeProvider.test.tsx`.

- [ ] **Step 1: Failing test:** a probe component calls `useBridge().listStreams()` under `<BridgeProvider bridge={new MockBridge()}>` and renders the count.
- [ ] **Step 2–4:** Implement `BridgeProvider` + `useBridge()` (React context; throws if used outside provider). Default export provides a `MockBridge` when none passed. **Step 5:** Commit.

---

## Phase 3 — Document parser

### Task 3.1: `parseDoc` — markdown → ParsedDoc

**Files:** Create `src/lib/parseDoc.ts`, `src/lib/parseDoc.test.ts`.

- [ ] **Step 1:** Define output types in `parseDoc.ts`:

```ts
export interface Citation { id:string; source:string; title:string; score?:number; date?:string; url:string }
export interface OutlineNode { id:string; label:string; level:1|2; moved?:boolean }
export interface CurrentSection { heading?:string; body:string[] }
export interface ParsedDoc {
  title:string; updatedLabel:string; sources:Citation[]; outline:OutlineNode[];
  whatChanged:string[]; current:CurrentSection[]; openQuestions:string[]; myNotes:string;
}
export function parseDoc(md: string): ParsedDoc { /* impl */ throw new Error("ni"); }
```

- [ ] **Step 2: Failing tests** against `SAMPLE_DOC`:

```ts
import { parseDoc } from "./parseDoc";
import { SAMPLE_DOC } from "../bridge/sampleData";
test("parses movements, subsections, sources, outline", () => {
  const d = parseDoc(SAMPLE_DOC);
  expect(d.title).toBe("AI Agents");
  expect(d.updatedLabel).toMatch(/updated/i);
  expect(d.whatChanged.length).toBe(2);
  expect(d.whatChanged[0]).toContain("[^hn1]");           // citation marker preserved for Cited
  expect(d.current.map(s => s.heading)).toEqual(["Durable execution","Tool calling"]);
  expect(d.openQuestions.length).toBe(1);
  expect(d.myNotes).toContain("Q3 planning");
  const hn = d.sources.find(s => s.id === "hn1")!;
  expect(hn.source).toBe("hackernews"); expect(hn.score).toBe(412); expect(hn.url).toContain("http");
  // outline = 4 movements + 2 subsections, in order
  expect(d.outline.map(n => n.label)).toEqual(
    ["What changed","Current understanding","Durable execution","Tool calling","Open questions","My notes"]);
  expect(d.outline.find(n => n.label==="Durable execution")!.level).toBe(2);
});
```

- [ ] **Step 3:** Run → FAIL. **Step 4:** Implement `parseDoc`: split on `^## `, route by heading; within Current split on `^### `; bullet lines (`- `) → arrays keeping `[^id]`; parse `[^id]: source · title · score · date · url` defs into `sources`; `myNotes` keeps raw body; build `outline` (movements level 1 + `###` level 2). **Step 5:** Run → PASS. **Step 6:** Commit.

---

## Phase 4 — Reading view

### Task 4.1: Citation (chip + popover)

**Files:** Create `src/views/Reading/Citation.tsx`; test `Citation.test.tsx`.

- [ ] **Step 1: Failing test:** renders a chip showing the short label (`source` abbrev + score); clicking opens a Radix popover with the citation `title` and an `open ↗` link to `url`.

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Citation } from "./Citation";
const c = { id:"hn1", source:"hackernews", title:"Claude Agent SDK v2", score:412, url:"https://x" };
test("chip opens popover with source", async () => {
  render(<Citation citation={c as any} />);
  expect(screen.getByText(/412/)).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button"));
  expect(await screen.findByText("Claude Agent SDK v2")).toBeInTheDocument();
  expect(screen.getByRole("link", { name: /open/i })).toHaveAttribute("href", "https://x");
});
```

- [ ] **Step 2–4:** Implement with `@radix-ui/react-popover`; chip styled via tokens (`--accent` on `--accent-tint`, `--mono`). **Step 5:** Commit.

### Task 4.2: Cited (text with inline `[^id]`) + Document + MyNotes

**Files:** Create `Cited.tsx`, `Document.tsx`, `MyNotes.tsx` + tests.

- [ ] **Step 1: Failing test (Cited):** given text `"...v2. [^hn1]"` and a `sources` map, renders the prose text and a `Citation` for `hn1` (assert the chip appears, the `[^hn1]` literal does not).
- [ ] **Step 2–3:** Implement `Cited` (split on `/\[\^(\w+)\]/`, interleave text + `<Citation>`). 
- [ ] **Step 4: Failing test (Document):** given a `ParsedDoc`, renders the title, the four movement labels ("What changed" with `data-accent`, others muted), the bullets via `Cited`, the subsection headings, and `<MyNotes>`.
- [ ] **Step 5: Failing test (MyNotes):** renders the notes markdown in an editable area; editing + blur calls `onSave(newMarkdown)`.
- [ ] **Step 6:** Implement `Document` + `MyNotes` (a `contentEditable`/`textarea` that calls `onSave`). **Step 7:** Run all → PASS. **Step 8:** Commit.

### Task 4.3: Outline (left rail)

**Files:** Create `Outline.tsx` + test.

- [ ] **Step 1: Failing test:** given `outline`, renders movements + indented (`level:2`) subsections; a node with `moved:true` shows the "moved" dot (`data-moved`); clicking a node calls `onJump(id)`.
- [ ] **Step 2–4:** Implement (mono nav items, active in `--accent`). **Step 5:** Commit.

### Task 4.4: Sources (right panel)

**Files:** Create `Sources.tsx` + test.

- [ ] **Step 1: Failing test:** given `sources: Citation[]`, renders a `Sources · N` header and one card per source showing `source`, `title`, and `score`.
- [ ] **Step 2–4:** Implement. **Step 5:** Commit.

### Task 4.5: Chrome (top bar)

**Files:** Create `Chrome.tsx` + test.

- [ ] **Step 1: Failing test:** renders the stream title, a back control (`onBack`), `Outline`/`Sources` toggle buttons reflecting `aria-pressed` from props, and a refresh control (`onRefresh`) + the `updatedLabel`.
- [ ] **Step 2–4:** Implement. **Step 5:** Commit.

### Task 4.6: Reading shell (3-zone + collapse state)

**Files:** Create `Reading.tsx` + test.

- [ ] **Step 1: Failing test:** mounted with a `streamId` under a `MockBridge` provider — by default neither rail is shown (clean column); clicking the Outline toggle reveals `Outline`; clicking Sources reveals `Sources`; clicking refresh calls `bridge.refreshStream`; editing My notes calls `bridge.saveNotes`.

```tsx
test("default clean; toggles reveal rails; refresh + notes wired", async () => {
  render(<BridgeProvider bridge={new MockBridge()}><Reading streamId="ai-agents" onBack={()=>{}} /></BridgeProvider>);
  expect(await screen.findByText("AI Agents")).toBeInTheDocument();
  expect(screen.queryByText(/^Outline$/)).not.toBeInTheDocument();      // collapsed by default
  await userEvent.click(screen.getByRole("button", { name: /outline/i }));
  expect(await screen.findByText("Durable execution")).toBeInTheDocument();
});
```

- [ ] **Step 2–4:** Implement: `getStream` → `parseDoc` → render `Chrome` + (conditionally) `Outline`/`Document`/`Sources` in a CSS grid that adapts columns to which rails are open; hold `showOutline`/`showSources` state (both default false). **Step 5:** Commit.

---

## Phase 5 — App routing, Desk, Create

### Task 5.1: App routing

**Files:** Modify `src/App.tsx` + test.

- [ ] **Step 1: Failing test:** `App` under a `MockBridge` renders Desk by default; selecting a stream shows Reading; a back control returns to Desk.
- [ ] **Step 2–4:** Implement a tiny view-state router (`"desk" | {reading:id} | "create"`), wrapped in `BridgeProvider` + theme. **Step 5:** Commit.

### Task 5.2: Desk (quiet-desk home)

**Files:** Create `Desk/Desk.tsx`, `Desk/StreamRow.tsx` + test.

- [ ] **Step 1: Failing test:** lists streams from `listStreams`; a row with `changedSinceSeen` shows the "something moved" mark (`data-moved`); a "Refresh now" control calls `bridge.refreshStream` and the row updates; selecting a row calls `onOpen(id)`; **no spinner role gates the list** while refreshing.
- [ ] **Step 2–4:** Implement (calm list; per-row + global refresh; pause/retire via `setStreamStatus`). **Step 5:** Commit.

### Task 5.3: Create (form + first-draft preview)

**Files:** Create `Create/Create.tsx` + test.

- [ ] **Step 1: Failing test:** the form has a topic input, a multiselect of `FREE_SOURCES`, and a cadence control; "Preview" calls `generateFirstDraft` and shows `draftMarkdown`; "Create" calls `createStream` with the assembled `StreamDescription` and calls `onCreated(summary)`; interval cadence requires `intervalMinutes` (validation blocks Create otherwise).
- [ ] **Step 2–4:** Implement. **Step 5:** Commit.

---

## Phase 6 — Craft pass (frontend-design)

- [ ] **Step 1:** With the structure tested and working against `MockBridge`, run the **frontend-design** skill against the brief to refine spacing, hierarchy, the chrome's scroll-frost, the collapse/expand transitions, and the reading-view rhythm — light and dark. Verify in `npm run dev`.
- [ ] **Step 2:** Add the **reconcile micro-interaction** intent (brief §5): on `refreshStream` `done` with `changed:true`, softly illuminate "What changed" and pulse the desk's moved mark — minimal, `prefers-reduced-motion`-aware.
- [ ] **Step 3:** Commit the polish pass.

---

## Acceptance (frontend phase done when)

- [ ] `npm run dev` renders Desk → open a stream → Reading view with the document; create flow works end-to-end against `MockBridge`.
- [ ] Reading view: clean column by default; Outline + Sources expand; inline chips open source popovers; My notes edits persist via `saveNotes`.
- [ ] Desk: streams list; "something moved" marks; refresh is non-blocking (no gating spinner).
- [ ] Light/dark follow the system and match the brief tokens.
- [ ] `npm run test` green across parser, bridge, theme, and all components.

---

## Plan self-review notes

- **Brief coverage:** §2 tokens → Task 1.1; §3 type → 1.1 + components; §4 layout (3-zone, collapse default, movements, citations, My notes) → Tasks 4.1–4.6; §5 motion → Phase 6; §1 calm/no-spinner → Desk test 5.2 + Reading 4.6.
- **Product-spec coverage:** §5.2 document/model-B → parseDoc (3.1) + MyNotes (4.2); §5.5 bridge commands → Bridge interface (2.3) + MockBridge; surfaces §8 → Phases 4–5. Backend §5–§14 intentionally excluded (later phase).
- **Type consistency:** `Bridge`, `StreamSummary`, `GetStreamResult`, `ParsedDoc`, `Citation`, `OutlineNode` defined once and reused; bridge methods (`listStreams`/`getStream`/`refreshStream`/`saveNotes`/`generateFirstDraft`/`createStream`/`setStreamStatus`/`onRefreshProgress`) named identically across interface, MockBridge, and component tests.
- **Deferred to frontend-design (not placeholders):** exact spacing, transitions, and the reconcile motion — explicitly Phase 6, per the brief.
