# Debugging & Manual Testing Guide

How to run Freshet, watch what it's doing, test it by hand, and hand a bug to Claude. Written
for macOS (the only target today).

## TL;DR

```bash
# Browser, fake data — fast UI work (no backend, no agent)
npm run dev                       # → http://localhost:1420  (uses MockBridge)

# Native app, REAL backend, YOUR Claude — the real thing
npm run tauri dev                 # run from a terminal where `claude` is logged in

# Native app, REAL backend, NO Claude needed — deterministic fake agent
FRESHET_FAKE_AGENT=1 npm run tauri dev

# Watch everything the backend does (Rust + JS, one stream):
tail -f ~/Library/Logs/com.seedz.freshet-tmp/freshet.log
```

## The two run modes (this is the #1 source of confusion)

Freshet talks only to a `Bridge`. **The environment decides which implementation runs:**

| You run | Bridge | Backend | Agent | Use it for |
|---|---|---|---|---|
| `npm run dev` (browser) | **MockBridge** | none — canned sample data | none | Fast UI/layout/flow work. "Create" returns fake data. |
| `npm run tauri dev` (native window) | **TauriBridge** | **real Rust** | **your real `claude`** | The actual product. Real fetches + real synthesis. |
| `FRESHET_FAKE_AGENT=1 npm run tauri dev` | TauriBridge | real Rust | **deterministic fake** | Real fetches + file writes, but no Claude auth needed. Great for testing the flow. |

**Key fact:** the browser (`npm run dev`) can *never* exercise the real backend — a browser tab
has no access to the Rust process. Only the native window does. So a bug in real sourcing/agent/
files only reproduces under `npm run tauri dev`.

## Prerequisites

- **Node** + **Rust** (`rustup`) + **Xcode Command Line Tools**.
  - **`cargo` must be on PATH** — `npm run tauri dev` shells out to `cargo`. If you see
    `failed to run 'cargo metadata' … No such file or directory`, your shell hasn't loaded Rust's
    PATH. Fix: `source "$HOME/.cargo/env"` (or open a new terminal). Verify: `cargo --version`.
- For the real agent path: **`claude` (Claude Code) installed and logged in**. Verify in your
  terminal: `claude -p "say hi"` should reply (not "Not logged in"). If it says not logged in,
  run `claude` → `/login` once.
- **Launch the native app from a terminal where that check passes** — the app spawns `claude` and
  inherits that terminal's login/keychain access. (Launching from a different environment is why
  you can see "Not logged in" even though your interactive Claude works.)

## Logs — your main debugging tool

Unified Rust **and** JS logging via `tauri-plugin-log`, written to three places:
- **Terminal** — the `tauri dev` stdout.
- **DevTools console** — see below (frontend + backend logs together).
- **File** — `~/Library/Logs/com.seedz.freshet-tmp/freshet.log` ← the one to `tail -f`.

The log traces every backend step: agent detection, each source fetch (URL + item count, or a
`WARN` when a channel is skipped), the engine (`fetched N, M new` / "nothing new" / synthesize +
doc length / write path), and — crucially — **the exact `claude` command run + its exit code and
stderr on failure** (so e.g. "Not logged in · Please run /login" shows up here instead of a hang).

```bash
tail -f ~/Library/Logs/com.seedz.freshet-tmp/freshet.log
```

## DevTools (the webview / frontend)

- In a debug build (`tauri dev`) DevTools **auto-opens**. Otherwise: right-click → **Inspect**, or **⌘⌥I**.
- On macOS this is **Safari's Web Inspector** (Tauri uses WKWebView). Console shows JS logs +
  (via the Webview log target) the backend logs too, and any failed `invoke(...)` errors.

## Manual test walkthrough (the real app)

1. `claude -p "say hi"` in your terminal → confirms login. Then `npm run tauri dev` from that terminal.
2. **Onboarding:** Welcome → **Choose folder** (pick a fresh empty folder for testing) → it detects
   your `claude` ("Found Claude Code ✓ …") → continue.
3. **Create a stream:** New stream → topic (e.g. "rust async") → **pick `hackernews` and/or `github`**
   (NOT `reddit` — it's currently blocked, see below) → **Preview**. A draft should render. → **Create**.
4. **Reading view:** open the stream — What changed / Current understanding / Open questions; toggle
   the Outline + Sources rails; click a citation chip; edit **My notes** (it persists).
5. **Refresh:** "Refresh now" on the desk; run it twice — the second run should say nothing new.
6. **Theme:** toggle dark/light (top-right); it also follows your system.

If you don't want to involve Claude at all, launch with `FRESHET_FAKE_AGENT=1 npm run tauri dev`
and do the same — synthesis is deterministic and offline.

## Known issues / constraints (current)

- **Reddit is blocked** — it returns HTTP 403 to anonymous requests (needs OAuth). Don't pick it as
  your only source; it's excluded from the defaults. Fix is tracked.
- **Polymarket** parsing was fixed to the real (string-number) API shape.
- **The live agent + live network path is the least-tested mile** — if something fails there, the log
  now tells us exactly where.
- **No automated driving of the native window on macOS** — Apple ships no WKWebView WebDriver, so
  tools like Playwright/Selenium can't drive the Tauri window directly. (We're evaluating
  `tauri-webdriver`, which injects a WebDriver bridge, to close this — see below.) For now: you click,
  the logs tell the story.

## Handing a bug to Claude

The fastest loop:
1. Reproduce it under `npm run tauri dev` (or `FRESHET_FAKE_AGENT=1` if it's not agent-specific).
2. Either **paste the relevant lines** from `~/Library/Logs/com.seedz.freshet-tmp/freshet.log`, or
   just say *"check the log"* — Claude can `tail` it directly.
3. For frontend-only issues, the DevTools console error is enough.

Claude can also reproduce most things **without you**:
- Backend logic → the headless harness: `cargo run --bin freshet_cli -- refresh <root> <stream-id> --fake`.
- UI flows → Playwright against `npm run dev` (mock).
- Only the **real Claude synthesis quality** and the **look-and-feel** genuinely need your eyes.

## Automated e2e (in evaluation)

`tauri-webdriver` is being spiked: a Tauri plugin (debug builds) that exposes a W3C WebDriver
interface over the WKWebView, letting a WebdriverIO script drive the **real app by selector** on
macOS. If it pans out, Claude can run the full onboarding→create→refresh flow automatically with
`FRESHET_FAKE_AGENT=1` — and this section will get the exact run command. (Status: see the spike
report / `NIGHT-SUMMARY`.)
