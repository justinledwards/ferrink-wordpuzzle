# Running audit

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
