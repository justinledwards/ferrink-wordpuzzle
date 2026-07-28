# Kindle Wordle — Resource References

## Rust Skills (leonardomso/rust-skills)
- **Repo:** https://github.com/leonardomso/rust-skills
- **AGENTS.md:** symlinks to SKILL.md at repo root
- **Rules:** 265 rules across 26 categories in `rules/` directory
- **Check harness:** `checks/` — validates rule structure, links, index, and that examples compile
- **CI:** GitHub Actions — runs `checks/check.sh` (reproducible check)
- **Build:** `mise run check` (or `make check`) — single reproducible check
- **Release:** 1.5.1 (265 rules), Rust 2024 edition
- **Key categories:** own- (ownership), borrow- (borrowing), conc- (concurrency), err- (error handling), pattern- (pattern matching), num- (numeric safety), serde- (serialization), trait- (traits & generics), const- (compile-time), unsafe- (unsafe code)

## Slint UI (slint-ui/slint)
- **Repo:** https://github.com/slint-ui/slint
- **AGENTS.md:** https://raw.githubusercontent.com/slint-ui/slint/master/AGENTS.md
  - Build: `cargo build`, `cargo test`
  - Examples: `cargo run --manifest-path examples/Cargo.toml -p <name>`
  - Backends: winit, Qt, Android, LinuxKMS, Testing/MCP
  - Renderers: femtovg (OpenGL), skia, software (CPU)
  - Key pattern: `slint::slint!` macro or `.slint` files via `slint::include_modules!()`
- **AI Plugins:** `ai-plugins/` — Claude and Cursor plugins for Slint development
- **Skills:** `ai-plugins/skills/slint/` — Slint skill for AI assistants
- **Testing backend with MCP:** `internal/backends/testing/` — includes MCP server for AI-assisted UI introspection
- **MCP server docs:** `docs/development/mcp-server.md`
- **Examples:** `examples/` — gallery, todo-mvc, virtual_keyboard, printerdemo, etc.

## Ferrink (Kindle-specific references)
- **Repo:** https://github.com/justinledwards/ferrink
- **Kindle platform crate:** `crates/ferrink-platform-kindle/`
  - `src/lightbox.rs` — KOA3 lightbox control
  - `src/linux_display.rs` — Kindle framebuffer / EPDC driver
  - `src/linux_lightbox.rs` — Linux ioctl for lightbox
  - `src/input_grab.rs` — Touch input via /dev/input
  - `src/input_loop.rs` — Event loop for touch
  - `src/touch_card.rs` — Touch calibration
  - `src/pixel_card.rs` — Pixel/display characterization
  - `src/stock_repaint.rs` — Stock framebuffer refresh protocol
  - `src/revalidate.rs` — Display revalidation
- **Kindle shell binary:** `crates/ferrink-shell/src/bin/ferrink-shell-kindle.rs`
  - Cross-compile target: `armv7-unknown-linux-musleabihf`
  - Uses `cargo-zigbuild` for cross-compilation
  - Device profiles in `device-profiles/` (reference-landscape.toml, reference-portrait.toml)
  - `tools/device-tool/` — deployment tooling
  - `docs/KINDLE_DEVICE_TOOL.md` — device setup guide
- **Slint backend dependency:** Uses `slint-backend-kindle` crate (slint-kindle-backend repo)
- **Font handling:** Embeds a TTF font (LiberationSans-Regular.ttf), registered via `slint_backend_kindle::install(FONT)`
- **Monochrome patterns:**
  - `set_black_and_white(true)` for flicker-free bilevel mode
  - Software renderer (`renderer-software` feature)
  - No GPU dependencies, no X11
- **Wake/suspend:** `WakeSchedule` with `wake_interval` and `stay_awake` for battery life

## Word Puzzle Slint (existing project reference)
- **Repo:** ~/git/wordpuzzle-slint/
- **MVC architecture:** follows slint-ui examples/todo-mvc pattern exactly
- **MCP server:** `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 SLINT_BACKEND=headless cargo run --features slint/mcp`
- **Word lists:** zstd-compressed in `data/words/words_{N}.zst` (N = 1..20)
- **Slint version:** git master rev 4c9941ea (1.18)
- **Cargo features:** `slint/mcp` for MCP server, `compat-1-2` for compatibility
- **Profiles:** optimize for size (`opt-level = "z"`), LTO, `panic = "abort"`, strip symbols

## SLWF-01 AC Dongle Setup
- Saved as skill: `slwf01-ac-dongle-setup`
- GE/Midea AC → SLWF-01 Pro replacement for full local control
- ESPHome midea climate component with autoconf

## Kindle Jailbreak
- PW1 (Paperwhite 1st gen) — 800×600, E Ink Pearl, touch input via /dev/input
- KOA3 (Oasis 3rd gen) — 1680×1264, E Ink Carta, lightbox ioctl support
- Cross-compile target: `armv7-unknown-linux-musleabihf`
- Deploy via USBNetwork (SSH) or packaging tools
- Requires jailbreak (e.g., WinterBreak, Freespace, etc.)

## Slint MCP Server
- Feature: `slint/mcp`
- Env: `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080 SLINT_BACKEND=headless`
- Endpoint: `http://localhost:8080/mcp` (streamable HTTP)
- Visualization: MCP client can take screenshots via JSON-RPC
- Note: Currently `SLINT_BACKEND=headless` may not be available in all Slint builds. Alternative: use `SLINT_BACKEND=testing` or omit the backend env var and use the testing backend with MCP.
