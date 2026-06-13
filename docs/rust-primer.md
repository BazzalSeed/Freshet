# Rust Primer — for Freshet (Tauri backend)

A practical primer aimed at *exactly* the Rust you'll meet building Freshet's Tauri core —
no more. Freshet's heavy lifting (multi-channel research + synthesis) is delegated to the
local `claude` agent (running the `last30days` skill) as a **subprocess**, so the Rust you
write is mostly **orchestration**: spawn a process, parse its markdown, dedup against saved
state, read/write files, and expose a few commands to the React frontend. That's the
friendly end of Rust. This primer covers that slice and points you at deeper resources.

> If you know Python/TS: the hard new idea is **ownership/borrowing**. Almost everything else
> maps to concepts you already have (`Result` ≈ try/except made explicit, `enum` ≈ tagged
> unions, `serde` ≈ pydantic/JSON.parse, `trait` ≈ interface/Protocol).

---

## 1. What you'll actually touch in Freshet

| Freshet component | Rust you'll use |
| :--- | :--- |
| **Provider** (call local `claude`) | `std::process::Command` — spawn, capture stdout |
| **Store** (read/write the markdown doc + state) | `std::fs`, atomic write via `tempfile` + rename |
| **Stream description / state sidecar** | `serde` + `serde_json` — JSON ↔ structs |
| **Reconciler / Memory** (dedup, "what changed") | structs, `Vec`, `HashSet`, iterators |
| **Bridge** (frontend ↔ core) | `#[tauri::command]` async fns + `emit` events |
| **Errors everywhere** | `Result<T, E>`, the `?` operator, `anyhow`/`thiserror` |

Notably **absent** in v1: hand-rolled HTTP clients, heavy async, threads, unsafe code. The
agent does the network; you orchestrate.

---

## 2. The one big idea: ownership & borrowing

Every value has exactly one **owner**. When the owner goes out of scope, the value is freed —
no garbage collector, no manual `free`. You pass values around three ways:

```rust
fn takes_ownership(s: String) { }      // moves s in; caller can no longer use s
fn borrows(s: &String) { }             // borrows read-only; caller keeps ownership
fn borrows_mut(s: &mut String) { }     // borrows to mutate; exclusive while borrowed
```

The **borrow checker** enforces two rules at compile time:
1. Either **one** mutable borrow (`&mut`) **or** any number of read-only borrows (`&`) — never both at once.
2. A borrow can't outlive the thing it points to.

This is why Rust has no null-pointer or data-race bugs — but it's also what the compiler will
yell at you about. Survival tips when it does:

- **Pass `&T` (borrow) by default.** Only take `T` (ownership) when you truly need to keep it.
- **`.clone()` to escape a fight.** Cloning a `String`/`Vec` is cheap relative to your time;
  optimize later. This is the #1 unblock-yourself move while learning.
- **Read the error fully** — Rust's compiler errors are unusually good and often suggest the fix.

You don't need to *master* this before being productive. Borrow `&`, clone when stuck, move on.

---

## 3. Error handling: `Result`, `Option`, and `?`

No exceptions. Fallible functions return `Result<T, E>`; maybe-absent values are `Option<T>`.

```rust
enum Result<T, E> { Ok(T), Err(E) }
enum Option<T>    { Some(T), None }
```

The `?` operator is the workhorse — "unwrap the `Ok`, or return the `Err` early":

```rust
fn load_state(path: &Path) -> Result<State, anyhow::Error> {
    let text = std::fs::read_to_string(path)?;     // ? returns early on IO error
    let state: State = serde_json::from_str(&text)?; // ? returns early on parse error
    Ok(state)
}
```

`?` ≈ Python's "let the exception propagate," but visible in the type signature. Use the
**`anyhow`** crate for app-level errors (one easy error type, `anyhow::Result<T>`), and
**`thiserror`** when you want named error variants for a library. In Freshet, `anyhow` is fine
almost everywhere.

`.unwrap()` / `.expect("msg")` crash on `Err`/`None`. Fine in tests and quick spikes; avoid on
the real refresh path (we never want to panic the backend).

---

## 4. Structs, enums, traits

```rust
// struct ≈ a dataclass
struct StreamDescription { id: String, title: String, topic: String, source: String }

// enum ≈ a tagged union; great for "one of N" + carrying data
enum RefreshPhase { Fetching, Synthesizing, Done { changed: bool }, Error(String) }

// trait ≈ an interface / Protocol — lets us swap a real LLM for a fake in tests
trait Provider {
    fn synthesize(&self, prompt: &str) -> anyhow::Result<String>;
}

struct ClaudeCli;                       // real impl: shells out to `claude`
impl Provider for ClaudeCli {
    fn synthesize(&self, prompt: &str) -> anyhow::Result<String> { /* … */ Ok(String::new()) }
}
```

Match on enums exhaustively (the compiler makes you handle every case — a feature):

```rust
match phase {
    RefreshPhase::Done { changed } => println!("done, changed={changed}"),
    RefreshPhase::Error(msg)       => eprintln!("failed: {msg}"),
    _                              => {}
}
```

---

## 5. JSON with `serde` (your stream description + state sidecar)

Derive `Serialize`/`Deserialize` and you get JSON ↔ struct for free — like pydantic:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct State {
    seen_item_ids: Vec<String>,
    last_checked_at: String,
    last_changed_at: Option<String>,
    doc_digest: String,
}

let state: State = serde_json::from_str(&text)?;       // parse
let text = serde_json::to_string_pretty(&state)?;      // serialize
```

---

## 6. Calling the local agent: `std::process::Command`

This is the heart of Freshet's Provider — spawn `claude`, hand it a prompt, capture stdout:

```rust
use std::process::Command;

fn run_claude(prompt: &str) -> anyhow::Result<String> {
    let output = Command::new("claude")
        .arg("-p").arg(prompt)            // adjust to the real CLI flags
        .output()?;                       // runs to completion, captures stdout/stderr
    if !output.status.success() {
        anyhow::bail!("claude failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?)
}
```

That's most of your "AI integration" — a subprocess returning markdown.

---

## 7. Files, done safely (atomic write)

Never write the living document in place — a crash mid-write would corrupt it. Write a temp
file, then rename (rename is atomic on the same filesystem):

```rust
use std::fs;
use std::io::Write;

fn atomic_write(path: &std::path::Path, contents: &str) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(contents.as_bytes())?;
    f.sync_all()?;                        // flush to disk
    fs::rename(&tmp, path)?;              // atomic swap
    Ok(())
}
```

(The `tempfile` crate gives you this more robustly; the above shows the idea.)

---

## 8. The Tauri bridge: commands & events

The frontend calls Rust via `#[tauri::command]` functions; Rust pushes updates via events.
Tauri commands are usually `async` and run on Tauri's runtime — you rarely manage async yourself.

```rust
// Rust side
#[tauri::command]
async fn refresh_stream(id: String, app: tauri::AppHandle) -> Result<Summary, String> {
    app.emit("refresh_progress", "fetching").ok();    // event → frontend
    // … do the refresh, mapping anyhow::Error -> String for the bridge …
    Ok(Summary { changed: true, n_new: 3 })
}
```

```ts
// Frontend side (React)
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

await listen("refresh_progress", (e) => setPhase(e.payload as string));
const summary = await invoke("refresh_stream", { id: "ai-agents" });
```

Note the bridge boundary returns `Result<_, String>` — convert your rich `anyhow` errors into a
string message there. Inside the core, keep using `anyhow`.

---

## 9. Cargo (the toolchain) in 8 commands

```bash
cargo new freshet-core         # new crate
cargo add serde serde_json anyhow tempfile   # add deps (writes Cargo.toml)
cargo add tokio --features full              # async runtime (Tauri pulls this in)
cargo build                    # compile
cargo run                      # build + run
cargo test                     # run #[test] functions
cargo clippy                   # lints — run this, it teaches you idiomatic Rust
cargo fmt                      # auto-format
```

Tests live next to code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedup_finds_only_new_items() {
        let seen = vec!["hn_1".to_string()];
        let fetched = vec!["hn_1".to_string(), "hn_2".to_string()];
        let new: Vec<_> = fetched.iter().filter(|id| !seen.contains(id)).collect();
        assert_eq!(new, vec!["hn_2"]);
    }
}
```

---

## 10. Crates you'll likely use in Freshet v1

| Crate | For |
| :--- | :--- |
| `serde` + `serde_json` | JSON ↔ structs (stream description, state) |
| `anyhow` | easy app-level errors + `?` |
| `thiserror` | named error types (optional) |
| `tempfile` | atomic file writes |
| `tauri` | the shell, commands, events |
| `tokio` | async runtime (mostly transitive via Tauri) |
| `chrono` | timestamps for `last_checked_at` etc. |

Probably **not** needed in v1: `reqwest` (the agent does the network), threads, `unsafe`.

---

## 11. Learning path (in order, time-boxed)

1. **The Rust Book**, ch. 1–6 and 9 — <https://doc.rust-lang.org/book/> (ownership, structs/enums, error handling). Skim the rest.
2. **Rustlings** — <https://github.com/rust-lang/rustlings> — small fix-the-compiler exercises; the fastest way to internalize borrowing.
3. **`cargo clippy`** on your own code — it explains idioms as you go.
4. **Tauri docs** — <https://tauri.app> — commands, events, state, the JS API.
5. Reach for the rest (lifetimes `'a`, generics, `Arc`/`Mutex`, real async) only when a task demands it. You can build Freshet's v1 backend without deep lifetimes or hand-written async.

**Mindset:** the borrow checker is a strict pair-programmer, not an enemy. When it blocks you,
borrow with `&`, `.clone()` to move on, and refactor later. Lean on the agent team's backend
teammate to write idiomatic Rust, and use this primer to *read and steer* it confidently.
