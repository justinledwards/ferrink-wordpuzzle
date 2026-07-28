# Kindle Wordle — Handoff Document

## What to Build
A native Wordle game for jailbroken Kindle E Ink readers, built in Rust + Slint. Single-player mode, 6-letter words with standard Wordle rules (6 guesses, color feedback per letter).

## Project Location
`~/git/slint-wordle/`

## Files Already Created
- **REFERENCES.md** — All gathered links, API patterns, and code locations
- **AGENTS.md** — Project instructions for AI coding assistants

## Key References (load all before starting)
1. **REFERENCES.md** — The comprehensive reference document at `~/git/slint-wordle/REFERENCES.md`
2. **Rust Skills** — `https://github.com/leonardomso/rust-skills` (265 rules, use clippy pedantic)
3. **Slint AGENTS.md** — `https://raw.githubusercontent.com/slint-ui/slint/master/AGENTS.md` (build/test/arch guidance)
4. **Slint Skills** — `https://github.com/slint-ui/slint/tree/master/ai-plugins/skills/slint` (Slint-specific AI guidance)
5. **Ferrink** — `~/git/ferrink/` (Kindle display, touch, cross-compile patterns)
   - Key file: `crates/ferrink-platform-kindle/src/linux_display.rs`
   - Key file: `crates/ferrink-platform-kindle/src/input_grab.rs`
   - Key file: `device-profiles/`
   - Build: `cargo zigbuild -p ferrink-shell --bin ferrink-shell-kindle --features kindle-runtime --target armv7-unknown-linux-musleabihf --release`
6. **Word Puzzle Slint** — `~/git/wordpuzzle-slint/` (MVC architecture reference, word data format)
   - MVC pattern from `slint-ui/slint/examples/todo-mvc`
   - Word lists in `data/words/words_{N}.zst` (zstd-compressed)
7. **Slint Examples** — `https://github.com/slint-ui/slint/tree/master/examples`
   - Especially: `todo-mvc`, `virtual_keyboard`, `gallery`
8. **Slint Kindle Backend** — `https://github.com/sverrejb/slint-kindle-backend`
   - `slint_backend_kindle::install(FONT)` — main entry point
   - `set_black_and_white(true)` — flicker-free bilevel mode
   - `WakeSchedule` — battery-saving suspend/wake

## Architecture

### Directory Structure
```
slint-wordle/
├── REFERENCES.md       (already exists)
├── AGENTS.md           (already exists)
├── Cargo.toml
├── build.rs
├── mise.toml
├── data/
│   └── words/
│       └── words_6.zst    (copy from wordpuzzle-slint)
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── callback.rs
│   ├── mvc.rs
│   ├── mvc/
│   │   ├── models.rs
│   │   ├── models/
│   │   │   └── word_model.rs
│   │   ├── repositories.rs
│   │   ├── repositories/
│   │   │   ├── traits.rs
│   │   │   ├── traits/
│   │   │   │   └── word_repository.rs
│   │   │   └── word_repository_impl.rs
│   │   └── controllers.rs
│   │   └── controllers/
│   │       └── wordle_controller.rs
│   ├── ui.rs
│   └── ui/
│       ├── wordle_adapter.rs
│       └── wordle_view.slint
└── fonts/
    └── LiberationSans-Regular.ttf
```

### MVC Architecture (follow slint-ui todo-mvc exactly)
- **`src/callback.rs`** — `Callback<Args, Result>` wrapper (Cell<Option<Box<dyn FnMut>>>)
- **`src/mvc/`** — controllers, models, repositories
- **`src/ui/`** — adapters that wire Slint globals to controllers
- **`src/lib.rs`** — `init()` creates controller, wires adapter, returns view handle
- **`src/main.rs`** — calls `lib::main()`

### Game Logic (WordleController)
- Pick random word from word list
- Track 6 guesses
- Per-guess feedback: correct (green), present (yellow), absent (gray)
- Win/lose detection
- Keyboard state tracking (which letters are used/status)

### Slint UI
- Game grid (6 rows × 6 columns) — each cell shows letter + background color
- On-screen QWERTY keyboard — letters change color based on status
- Monochrome/grayscale palette for Kindle (use patterns from ferrink)
- Touch input via Kindle touchscreen (/dev/input)
- For dev/desktop: standard winit backend works

### Kindle-Specific
- Cross-compile target: `armv7-unknown-linux-musleabihf`
- Use `cargo-zigbuild`
- E Ink rendering via `slint-backend-kindle` crate
- Software renderer only (no GPU)
- Bilevel (black & white) mode for flicker-free updates
- Embed LiberationSans-Regular.ttf font
- Wake/suspend support for battery life

### Desktop Dev
- Same codebase runs on Linux with `cargo run` (uses winit backend)
- MCP server for visual verification: `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 cargo run --features slint/mcp`

## Cargo.toml Skeleton
```toml
[package]
name = "slint-wordle"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "slint-wordle"
path = "src/main.rs"

[build-dependencies]
slint-build = { git = "https://github.com/slint-ui/slint", rev = "4c9941ea2596aad1fbd54b8d4db5bdf73b61c170" }

[dependencies]
slint = { git = "https://github.com/slint-ui/slint", rev = "4c9941ea2596aad1fbd54b8d4db5bdf73b61c170", features = ["std", "compat-1-2", "renderer-software"] }
zstd = "0.13"
rand = "0.8"

[target.'cfg(target_os = "linux")'.dependencies]
slint = { git = "https://github.com/slint-ui/slint", rev = "4c9941ea2596aad1fbd54b8d4db5bdf73b61c170", features = ["std", "compat-1-2", "renderer-software"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = "symbols"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
```

## Build Commands
```bash
# Desktop dev
cargo run

# Desktop with MCP (visual verification)
SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 cargo run --features slint/mcp

# Cross-compile for Kindle
cargo zigbuild -p slint-wordle --target armv7-unknown-linux-musleabihf --release

# Quality gates
cargo clippy --all-targets --all-features -- -Wclippy::pedantic
cargo test --workspace --all-targets --all-features
cargo fmt --check
```

## Word Data
- Copy `data/words/words_6.zst` from `~/git/wordpuzzle-slint/data/words/`
- Format: zstd-compressed, one word per line, newline-terminated
- WordRepositoryImpl loads and caches — same pattern as wordpuzzle-slint

## Quality Gates
- Zero clippy pedantic warnings
- All tests pass
- cargo audit passes (or findings documented)
- MCP screenshot verification of rendered UI
