# Running audit

- 2026-08-18: Integrated the shared passive magnetic-cover callback from the
  Kindle Slint backend and added the uniform Ferrink sleeping overlay. The app
  leaves suspend and wake authority with Amazon powerd.
- 2026-08-18: Integrated the complete versioned, atomic game-history work from
  GitHub with the grayscale, dictionary, and sleep changes. Updated the
  host-only `webbrowser` lock to 1.2.2 after RUSTSEC-2026-0257; `cargo audit`
  then passed with four transitive unmaintained-crate warnings and no known
  vulnerabilities.

- 2026-07-28: Cloned the initial draft and confirmed the checked-in formatter,
  test, and pedantic-Clippy gates did not pass.
- 2026-07-28: Isolated desktop Slint features from the ARM musl target and
  added the reviewed software-rendered Kindle backend behind target cfgs.
- 2026-07-28: Corrected the six-wrong-guesses regression test and the initial
  pedantic-Clippy findings before device work.
- 2026-07-28: Kept duplicate-letter evaluation in the controller so the UI and
  keyboard render the same bounded Wordle result, and made quit orderly.
- 2026-07-28: `cargo audit` reported no vulnerabilities and four unmaintained
  transitive dependencies in the pinned Slint graph: `bincode`, `paste`,
  `rustybuzz`, and `ttf-parser`.
- 2026-07-28: Produced a stripped, static ARM EABI5 release executable with
  `cargo zigbuild`; after the E Ink corrections its size is 6,684,020 bytes and
  SHA-256 is
  `994a6565a03e2265d17b506b7662da94433874abcf4159e78c3663452d32e32e`.
- 2026-07-28: The first KOA3 capture exposed borders that disappeared under
  bilevel quantization. Changed interactive outlines to black and kept all six
  guess rows visible so the initial game state is legible on E Ink.
- 2026-07-28: Replaced the unsupported checkmark on the submit key with the
  ASCII label `GO` after the KOA3 font rendered the original glyph blank.
- 2026-07-28: Installed the exact release binary through Ferrink, verified its
  `/proc` executable path and SHA-256, observed live guesses and evaluated
  tile/keyboard feedback, and left the application running on the KOA3.
- 2026-07-30: Added `GAME_HISTORY_PLAN.md` for a versioned, atomic log of
  completed games and derived post-game statistics. This change is planning
  only; runtime persistence and UI behavior are not implemented yet.
- 2026-07-30: The documentation-only planning change passed formatting, all 10
  tests, and pedantic Clippy. `cargo audit` found no vulnerabilities and
  repeated the four already documented unmaintained transitive dependencies.
- 2026-07-30: Implemented the version-one game-history model, validation,
  derived statistics, atomic owner-only JSON repository, deterministic desktop
  and Kindle paths, controller recording, demo isolation, and post-game Slint
  statistics panel.
- 2026-07-30: Local validation passed formatting, all 37 tests, and pedantic
  Clippy with zero warnings. `cargo audit` found no vulnerabilities and repeated
  the four allowed unmaintained transitive dependency warnings.
- 2026-07-30: `cargo zigbuild` with Homebrew Zig 0.16.0 produced a stripped,
  statically linked 32-bit ARM EABI5 release binary (6,736,084 bytes, SHA-256
  `64f92ade1bcb34052db2ec899fbd79f93f697058d1433ea120df1cf5d8996c8c`).
  Kindle installation, rendering, and restart persistence verification remain
  pending because no device connection is configured in this checkout.
- 2026-07-30: Restored a strict, pinned-host Ferrink connection using the
  securely transferred KOA3 deployment identity. Re-ran all locked release
  gates, atomically replaced only `/mnt/us/ferrink-wordpuzzle`, synced it, and
  verified the device file is 6,736,084 bytes with SHA-256
  `64f92ade1bcb34052db2ec899fbd79f93f697058d1433ea120df1cf5d8996c8c`.
- 2026-07-30: Launched Word Puzzle from a freshly captured Ferrink frame and
  verified the six five-cell rows, outlined keyboard, `GO`, backspace, and
  upper-right close control on the KOA3. Exactly one process resolved to the
  installed executable. The state directory is intentionally not created until
  the first completed game, so completed-game and restart persistence checks
  remain pending.
- 2026-07-30: Merged the Kindle grayscale and dictionary work into the
  history-enabled checkout: enabled 16-shade rendering, restored the evaluated
  tile/key palette and indicators, expanded accepted guesses to 12,953 words,
  and separated the 2,309 curated target words from accepted guesses.
- 2026-07-30: The merged build passed formatting, all 39 tests, and pedantic
  Clippy with zero warnings. The dependency audit found no vulnerabilities and
  repeated four allowed unmaintained transitive dependency warnings.
- 2026-07-30: `cargo zigbuild` produced a stripped, statically linked 32-bit ARM
  EABI5 release binary (6,759,492 bytes, SHA-256
  `981a176456f8aebcb549a27ca541d9f79f1ff954d036230e6f78904d4c5175e4`).
  It was atomically installed, launched from a fresh Ferrink capture, visually
  verified with grayscale evaluated tiles and keys, and matched one live
  `/mnt/us/ferrink-wordpuzzle` process with the exact device-side hash.
