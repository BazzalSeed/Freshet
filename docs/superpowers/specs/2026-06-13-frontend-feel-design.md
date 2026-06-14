# Freshet Frontend Feel — Design Brief

*Status: design (2026-06-13). Locks the **palette, type, and reading-view layout** for Freshet's
frontend so the build has a fixed aesthetic foundation. Sits under the product spec
([`2026-06-12-freshet-v1-design.md`](./2026-06-12-freshet-v1-design.md)); honors the design
language in [`product-vision.md`](../../product-vision.md) §5. Derived via a visual brainstorm;
the reference mockups persist in `.superpowers/brainstorm/`.*

> Scope: the **reading view** (the document — the craft centerpiece) is fully specified here. The
> quiet-desk home and creation form **inherit the palette + type**; their specific layouts are
> brainstormed when we build them. Build the reading view first.

---

## 1. Principles

- **The document is the hero** — typography-first; chrome recedes; nothing competes with the text.
- **Calm by default** — no spinners gating views, no badges/counts, no manufactured urgency (vision §4, §9).
- **Progressive disclosure** — power features (outline, full sources) are tucked away and pulled in on demand.
- **System-adaptive** — light/dark follow the OS (`prefers-color-scheme`); "warm paper by day, terminal by night."
- **One voice across modes** — the *type system never changes* between light and dark; only the surface temperature does.
- **Own visual language** — headless primitives (Radix/Ark) for plumbing (popover, toggles); the pixels are ours.

## 2. Color tokens

Two themes, same roles. Implement as CSS custom properties (mirrors the proven token set from
seedzeng.com). The accent **shifts by mode** — warm ochre in light, terminal green in dark.

```css
:root {                       /* LIGHT — premium paper */
  --bg:        #f4ecdd;       /* page / paper */
  --surface:   #ebe1cf;       /* rails, chrome, notes block */
  --surface-2: #f7f0e3;       /* sources panel */
  --ink:       #2b2620;       /* titles */
  --fg:        #352d25;       /* body text */
  --muted:     #8a7a63;       /* metadata */
  --muted-2:   #a8967b;       /* secondary section labels */
  --rule:      #e0d6c4;       /* hairlines */
  --accent:    #9c5b33;       /* ochre — "what changed", citations, active nav */
  --accent-tint: rgba(156,91,51,.11);
  color-scheme: light;
}
[data-theme="dark"] {          /* DARK — terminal */
  --bg:        #0d1117;
  --surface:   #161b22;
  --surface-2: #161b22;
  --ink:       #e6edf3;
  --fg:        #c9d1d9;
  --muted:     #8b949e;
  --muted-2:   #8b949e;
  --rule:      #21262d;
  --accent:    #3fb950;       /* terminal green */
  --accent-tint: rgba(63,185,80,.13);
  color-scheme: dark;
}
```

## 3. Type system

One system, both modes. **Newsreader** (serif, variable, optical sizing) for the document;
**IBM Plex Mono** for everything structural (metadata, section labels, citations, chrome).

```css
--serif: 'Newsreader', Georgia, serif;          /* titles + reading copy */
--mono:  'IBM Plex Mono', ui-monospace, monospace; /* labels, meta, chips, chrome */
```

| Role | Face | Spec |
| :--- | :--- | :--- |
| Document title | serif | 600, ~30px, tracking −0.005em |
| Body / bullets | serif | 400, ~15.5px, line-height 1.65 |
| Subsection heading | serif | 600, ~17px |
| Section label (What changed, etc.) | mono | 600, ~11.5px, uppercase, tracking 0.15em |
| Metadata (updated · sources) | mono | 500, ~10.5px, uppercase, tracking 0.08em |
| Citation chip | mono | 500, ~11px, in `--accent` on `--accent-tint` |
| Chrome / toggles | mono | 500, ~10.5px, uppercase |

(Self-host the two variable fonts for offline-first; no runtime CDN dependency.)

## 4. Reading-view layout

A three-zone shell; the two side zones **collapse**, defaulting to a clean centered column.

```
┌ chrome ─────────────────────────────────────────────┐
│ ‹ Streams   ☰ Outline      AI Agents      ⌗ Sources  ⟳ 2d │
├──────────┬──────────────────────────────┬───────────┤
│ OUTLINE  │   AI Agents            (title)│ SOURCES·4 │
│ (collaps)│   updated 2d · 4 src    (meta)│ (expand)  │
│ ● What…  │   ── What changed             │ HN  412↑  │
│   Curr…  │   • … [HN 412↑]               │ r/ML 280↑ │
│   ‣ Dur. │   ── Current understanding     │ GitHub    │
│   ‣ Tool │     Durable execution …        │ Polymkt   │
│   Open…  │   ── Open questions            │           │
│   Notes  │   ── My notes  [editable]      │           │
└──────────┴──────────────────────────────┴───────────┘
```

- **Default state:** both rails collapsed → just the centered column (~440–460px measure), generous margins. Calmest, vision-true.
- **Chrome (slim):** `‹ Streams` (back to desk) · `☰ Outline` toggle · centered stream title · `⌗ Sources` toggle · `⟳` refresh + last-updated. Frosted/transparent until scrolled (like seedzeng.com's header).
- **Left — Outline (collapsible):** the document's sections **and subsections** as jump-nav; active section in `--accent`; a "moved" dot marks sections changed since last visit. For navigating longer documents.
- **Right — Sources (expandable):** the full citation list grouped by source (title · score · date · link). The "show me everything" view.
- **Center — the document** (the four movements from product spec §5.2):
  - **What changed** — label in `--accent` (it's the delta; it earns primacy).
  - **Current understanding** — muted label; may contain serif subsection headings.
  - **Open questions** — muted label.
  - **My notes** — muted label; **editable** block on `--surface` (model B; saved via `save_notes`).
- **Citations:** inline mono **chips** in the text (calm); hover/click opens a **popover** with the full source (title, score, comments, date, `open ↗`). No permanent margin. The right Sources panel is the aggregate view.

## 5. Motion (intent; build may stub in v1)

Spring physics, restrained. The **signature moment is a document reconciling** (vision §5): when a
refresh lands, the "What changed" section illuminates and new findings settle in; the desk's "something
moved" mark gives a soft pulse. v1 may ship a minimal version; the *intent* is locked here. Always
respect `prefers-reduced-motion`.

## 6. Inherited / deferred

- **Quiet-desk home & creation form:** inherit §2 palette + §3 type. Their layouts get a short visual
  pass when built — not blocked by this brief.
- **Deferred polish:** the full reconcile animation, native macOS vibrancy/translucency depth, and the
  warm/paper texture refinements — the "make it pop" phase.
- **Accent-shift note:** the rust→green mode shift is intentional brand continuity with seedzeng.com; if
  it ever feels jarring, the fallback is a single warm accent in both modes.

## 7. Implementation notes

- CSS custom properties + a `data-theme` attribute set from `prefers-color-scheme` (with a manual toggle later).
- Headless **Radix/Ark** primitives for the popover and rail toggles; styled entirely with our tokens.
- Build in the browser-mock loop (`npm run dev`) against `MockBridge` + the sample living document.
