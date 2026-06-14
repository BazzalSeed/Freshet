# Freshet — Product Vision, Feel & Design (v1.0 · handoff)
 
*A vision, experience, and design brief — not an implementation plan. It describes
what Freshet is, why it matters, how it should feel and look, and how it might
eventually make money. The data model, stack details, and exact mechanics are
deliberately left to the builder (see §10). Sections §9 (principles) are invariants
the build must honor; §13 (open questions) are decisions that belong to the owner,
not the agent.*
 
---
 
## 1. What it is
 
A local-first desktop app that turns the handful of topics you care about into
**knowledge streams** — standing subscriptions that quietly keep themselves current
and synthesize what they find into a living document you actually want to read.
 
The one-line shift: it converts the chat model — *you pull, it forgets, it always
produces something* — into a **watch model** — *it pushes, it remembers, it only
tells you what changed.* The intelligence inside is commodity (same LLMs as
everyone). The experience around it is the product.
 
## 2. Why it matters
 
The world has no shortage of information about the things you care about. It has a
shortage of *calm* ways to stay current. Feeds, newsletter piles, thirty open tabs,
the low hum of having fallen behind — staying informed has come to mean being
perpetually available to be informed.
 
Freshet's job is the inverse: **stay genuinely current on the few things that matter
to you, without it costing your attention or your peace.** It does the keeping-up so
you don't have to, and it's honest enough to stay quiet when nothing real happened.
 
For: curious people with a few deep interests — developers, researchers, investors,
operators — tired of the feed but unwilling to be uninformed.
 
Why it could have pull: value *accretes*. Each stream gets richer the longer it
runs, so the cost of leaving grows weekly — retention built into the artifact. And
in an attention-saturated market, *calm is itself a differentiator*: people who feel
the relief tell other people who feel it.
 
## 3. The shift: from chatting to watching
 
Everything distinctive follows from one reframing — the felt differences a user
notices:
 
- **It watches for you.** Define a topic once; it keeps an eye on it, unprompted.
- **It remembers, and tells you what *changed*** — an understanding that builds, and
  the rare signal of "the consensus moved since last time."
- **It has the discipline to stay quiet.** When nothing meaningful happened, it says
  so. It never manufactures novelty.
- **It builds something that gets better** — one living document per topic that
  deepens, not a transcript you lose or a list of links to go read.
*Honest framing for the builder:* synthesis quality and "scheduled prompts" are not
the value — the big assistants have or will have both. The value is the
**combination** of watching + remembering + restraint. Build for that; treat raw
generation as a solved input.
 
## 4. The feel  ← the heart of this brief
 
**Freshet should feel calm.** The category it reacts against is engineered to take
your attention; Freshet is engineered to give it back. Its success metric is your
*trust*, not your time-in-app — the rare tool you open seldom and are rewarded by
every time.
 
Experiences to design around:
 
**Creating a stream feels like planting, not querying.** You describe a curiosity
out loud and it becomes a thing that tends itself. The setup is collaborative — it
asks the right narrowing questions, then shows a real first draft *before* you
commit. That draft should feel like being understood: "yes, that's the shape of what
I meant," close enough that refining is a pleasure, not a correction.
 
**Opening the app feels like glancing at a quiet desk, not entering a feed.** A
handful of streams, most still, one or two with a soft mark: *something moved.* No
counts climbing, no red badges, no manufactured urgency. Look away for a week
without guilt; the desk is in order when you return.
 
**Returning to a document feels like a garden that grew without you.** It should be
*better* than you left it — deepened understanding, not a filled inbox. What changed
sits at the top; settled understanding below, organized and cited; open questions
kept honestly open.
 
**The quiet is a feature you can feel.** "Nothing new worth your time" builds more
trust than always finding something. And the rarest moment — a stream telling you
something it once told you is no longer true — is the one that makes you feel
genuinely *ahead*.
 
## 5. Design language — forward and fancy through craft, not ornament
 
§4 is the goal; this is how it shows up on screen. The research is clear and
convenient: in 2026 the premium, *forward* signal comes from **craft** (Linear,
Things, Raycast), while ornamental flash — heavy glass, neon, jelly buttons, kinetic
text — reads as down-market. So Freshet's "fancy" is restraint, speed, motion, and
typography — which is the *same direction* as calm. We don't choose between calm and
forward; right now they're one lane.
 
- **Speed is the feel.** Instant interactions, optimistic UI, no spinners, work in
  the background. Nothing signals premium faster than zero latency.
- **Motion is where the fancy lives.** Spring physics, fluid transitions, a few
  perfectly-tuned micro-interactions — not decoration. Spend the motion budget on
  Freshet's signature moment: **watching a document reconcile** — findings settling
  into place, the "what changed" line illuminating, the soft "something moved" pulse.
  No other app has this; it's the most forward, AI-native interaction in the product.
- **Typography is the hero.** The reading view is a beautifully typeset publication:
  real type scale, generous measure, variable fonts, mono for metadata. The document
  *is* the product — treat it like a designed page.
- **Restrained native depth.** Tasteful macOS vibrancy/translucency and soft layering
  for calm spatial hierarchy — depth, not gloss. Borrow from the Apple direction
  without the noise.
- **Keyboard-first.** A command palette (create a stream, jump between streams,
  refresh now). Forward, power-user, and calm — it removes visible clutter.
- **Restraint as confidence.** What you leave out is part of the craft. A quiet desk
  with three streams and gorgeous type beats a twelve-widget dashboard.
**Palette direction (one taste call — confirm before UI, see §13):** lean
*cool / editorial* — near-black "ink," mono accents, terminal restraint — with a
*warm / paper* alternate (cream, ink, serif body, field-journal) on the table.
Either can be executed forward if the craft/motion/type bar is held.
 
## 6. Building the UI (approach)
 
- **Shell:** Tauri 2 — native window (OS webview, small binaries), bundling, signing,
  updater. Tauri renders nothing itself; it hosts your frontend.
- **Own design language, no stock kit.** A turnkey component library carries a
  recognizable templated look that would erase Freshet's identity. Build the language.
- **Learn structure — not aesthetics — from well-built Tauri apps (Tolaria):** the
  browser-mock dev loop (iterate the whole UI as a web app at localhost first, drop
  into the native window only for shell bits); a clean frontend/native boundary via a
  thin typed command + event bridge; status hooks subscribing to native state; native
  window affordances. Copy techniques; the pixels are yours (Tolaria's are AGPL — keep
  your eyes off them).
- **Bespoke the ~3 signature surfaces** that carry the feel — the reading view, the
  quiet-desk home, the stream-creation chat — and use **headless primitives**
  (Radix/Ark) for the plumbing (dialogs, menus, popovers): accessibility without an
  inherited look.
- **Frontend:** React + Vite is the pragmatic default (plain SPA — not Next.js; no
  server in a desktop app). Svelte/Solid acceptable if the builder prefers.
- **Native Mac touches:** vibrancy, hidden titlebar with custom traffic-light insets,
  native menus, system dark-mode — the difference between "real Mac app" and "webpage
  in a frame."
## 7. The shape (at the level of experience)
 
Three things the user encounters — by what they *are to the user*, not how they're
stored:
 
1. **The stream** — a standing curiosity you own. Create it by talking; pause or
   retire it; it works quietly between visits.
2. **The setup conversation** — collaborative scoping that turns a vague interest into
   a well-aimed stream, ending in a live first draft you react to.
3. **The living document** — one evolving, readable note per stream: what changed, the
   current understanding, what's still open. It accretes; it never resets.
## 8. Cadence & rhythm
 
Cadence is **per-stream and user-configurable** — on app launch (sensible default),
manual ("refresh now"), or an interval (hourly / daily / weekly). Event-driven, never
a rigid global cron.
 
A launch or background refresh **must never block the window.** It runs quietly in the
background; the desk is live immediately and fills in as results land — the soft
"something moved" mark appearing, never a spinner gate. Because "nothing new" is a
valid outcome (§9), frequent checks don't create noise.
 
## 9. Principles the build must honor
 
The "how" is the builder's. These invariants are not:
 
- **Installable local app, never a hosted webapp.** Ships as a desktop app (Tauri or
  equivalent) via Homebrew / signed download. A browser can't detect a local agent,
  can't write to the vault, and makes you the actor for source access — all of which
  break the product. (A pure-Python desktop path like pywebview is an acceptable
  fallback; the invariant is *installable local app*, not a specific framework.)
- **Push, not pull.** It works between visits; the user never has to re-ask.
- **Stateful.** It must know what it has already said — enough to deduplicate and
  detect change over time. (However implemented.)
- **Quiet by default & non-blocking.** "Nothing new" is a valid outcome; never
  manufacture novelty. Refreshes run in the background and never gate the UI.
- **Significance over recency.** Rank by what's worth knowing, not what's newest.
- **Local-first & BYO-LLM.** Runs on the user's machine and keys. Prefer a detected
  local agent (Claude Code, Codex); fall back to a local model, then an API key. No
  accounts, no server holding user data.
- **Don't rebuild the vault.** Produce plain markdown into a folder any vault app can
  read. Reading Freshet's *own* output is fine; general-purpose note management
  (folders, tags, backlinks, graph, freeform editing, global search) is out of scope —
  that's the user's existing vault app's job.
- **Topics, not accounts.** Track public signal on subjects, not private OAuth data.
- **Calm over engagement.** Never optimize for time-in-app. Optimize for trust.
## 10. What this spec deliberately leaves to the builder
 
Design these from the real codebase to honor §9 — treat no prior sketch as binding:
the data model and storage; the desktop framework details and packaging; how research
fan-out and reconciliation actually work; the scheduling mechanism; the precise source
set beyond a sensible API-legitimate start (e.g. Reddit, Hacker News, GitHub,
Polymarket, Bluesky). Defer anything needing a logged-in session (e.g. X via a browser
session) — not a first-cut concern.
 
## 11. Business model & monetization (a phase-two motion)
 
**There is no *easy* monetization path here — that's the deliberate cost of the
architecture, not a flaw.** The frictionless lever (a hosted SaaS subscription) needs
the server, accounts, and data custody Freshet rejects by design. Local-first + BYO
means you can't gate compute, meter unseen usage, or hold vault data.
 
The model that fits is **Obsidian's**: a core app free forever and fully local,
monetized through *optional* cloud services that genuinely require a server, while the
free tier stays serverless and custody-free. (Obsidian: free app, paid Sync ~$4–5/mo
and Publish ~$8–10/mo, no user data on their servers.)
 
The natural first paid service falls out of a desktop limit — **a desktop app only runs
when open.** So the upsell is an **always-on cloud runner**: streams keep updating on
schedule in the cloud and sync down to the vault, so you return to fresh documents.
People pay for "it works while I'm away," and the free local version structurally can't
do it. *Caveat:* a cloud runner can only touch API-legitimate sources; session-based
sources (X) stay local-only. Clean split: free local app = full reach under the user's
identity; paid runner = the API-safe subset, always-on.
 
Lighter levers: a one-time **supporter tier** (beta builds / badge — Obsidian's
"Catalyst" ~$25), a **voluntary commercial license** for teams, **sponsorship** if open
source. The fork that decides which exist: **open-source or not** (open → revenue from
cloud services; closed → sell the binary but lose community distribution).
 
**Stance:** monetization never touches the free local core; it's phase two — earn
adoption and trust first.
 
## 12. Non-goals (hold this line)
 
- Not a note manager / vault — write into one, don't become one.
- Not a hosted SaaS *at the core* — the free product is local-only, user's own keys, no
  central data. Any paid cloud service is a strictly optional add-on, never required.
- Not OAuth-into-your-own-accounts personalization — topics, not private data.
- Not a daily-by-default digest — cadence is per-stream and event-driven.
- Not an engagement product — no streaks, no badges, no pull-to-return.
## 13. Open questions (owner's to decide — not the agent's)
 
1. **How much should a stream hold still vs. evolve?** Pinned to approved sources
   (predictable, can stale) or free to wander (fresh, can drift). Honest middle: a
   stable core that *proposes* new directions rather than silently taking them.
2. **How much shape do you impose vs. let emerge?** Fixed sections you define, or
   structure the stream discovers as a topic grows.
3. **What default rhythm feels alive but not noisy?** On-launch is the default; the
   tuning is how often "something moved" should surface to feel attentive, not demanding.
4. **Open-source or closed?** Decides community distribution vs. which monetization
   levers stay available (§11). Decide consciously, early.
5. **Palette: cool/ink or warm/paper?** (§5) The one taste call to confirm before UI
   work begins.
