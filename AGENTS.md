# Kindle Wordle — AGENTS.md

## Project
A Wordle game written in Rust + Slint, designed to run natively on jailbroken Kindle E Ink readers. Single-player, 6-letter words. Built with `slint-kindle-backend` for hardware rendering.

## Resources
Before editing any file, load the full reference document:
- `REFERENCES.md` — All gathered links, API patterns, and code locations

## Key References
- **Rust Skills:** 265 rules across 26 categories — use `clippy -- -Wclippy::pedantic` to enforce
- **Slint AGENTS.md:** Build, test, and architecture guidance from slint-ui/slint
- **Slint Skills:** `ai-plugins/skills/slint/` in the slint-ui repo
- **Ferrink:** Kindle display, touch input, cross-compilation, and monochrome UI patterns
- **Word Puzzle Slint:** ~/git/wordpuzzle-slint/ — MVC architecture, word data, Slint version pinning
- **Ferrink Platform Kindle:** `crates/ferrink-platform-kindle/` — Kindle framebuffer, lightbox, touch input

## Kindle Display
- Software renderer only (`renderer-software` feature)
- `slint_backend_kindle::install(FONT)` for backend init
- `set_black_and_white(true)` for flicker-free bilevel mode
- No GPU, no X11, no fontconfig — embed a TTF font
- Wake/suspend: `WakeSchedule` for battery life

## Cross-Compilation
- Target: `armv7-unknown-linux-musleabihf`
- Tool: `cargo zigbuild`
- Requirements: `rustup target add armv7-unknown-linux-musleabihf`, `cargo install cargo-zigbuild`

## MVC Architecture
Follow `slint-ui/slint/examples/todo-mvc` pattern (same as wordpuzzle-slint):
- `src/callback.rs` — Callback<Args,Result> wrapper
- `src/mvc/` — controllers, models, repositories
- `src/ui/` — adapters wiring Slint globals to controllers
- `src/lib.rs` — init() creates controller, wires adapter, returns view handle
- `src/main.rs` — calls lib::main()

## MCP Server (Visual Verification)
```bash
SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 SLINT_BACKEND=headless \
  cargo run --features slint/mcp
```

## Quality Gates
- `cargo clippy --all-targets --all-features -- -Wclippy::pedantic` — zero warnings
- `cargo test --workspace --all-targets --all-features`
- `cargo audit` — document any findings
- `cargo fmt --check`

## Word Data
- 6-letter word list in `data/words/words_6.zst` (zstd-compressed, one word per line)
- Word data uses same format as wordpuzzle-slint
- Copy and adapt from ~/git/wordpuzzle-slint/data/

## Conventions
- Always load REFERENCES.md and check AGENTS.md before editing
- Datestamp all log entries to RUNNINGAUDIT.md
- Update TODO.md with checkmarks (never remove lines)
- Batch independent file reads/searches
- Use `patch` for edits, `write_file` for new files
- clippy pedantic + cargo audit required before completion
