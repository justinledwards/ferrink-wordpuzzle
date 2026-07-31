# Persistent Game History and Post-Game Statistics Plan

Status: deployed to KOA3; completed-game and restart verification pending

Last updated: 2026-07-30

## Outcome

Keep a durable, ordered record of every completed game and use that record to
show accurate statistics immediately after a game ends. History must survive
application restarts, Kindle reboots, and application upgrades.

The first release will show:

- games played, won, and lost;
- win percentage;
- current winning streak;
- longest winning streak;
- wins by attempt count from one through six; and
- the attempt count for the game that just ended.

The history is the source of truth. Aggregate counters are derived when history
is loaded or changed rather than stored separately, so counters cannot drift
away from the underlying games.

## Product Rules

1. Record one entry only when a game transitions from `Playing` to `Won` or
   `Lost` in `WordleController::submit_guess`.
2. Count accepted, submitted guesses as attempts. Incomplete guesses, words not
   in the dictionary, and duplicate guesses do not count.
3. A win records `attempts` in the inclusive range `1..=6`.
4. A loss records `attempts = 6`.
5. Quitting or closing an unfinished game is not a loss and creates no record.
6. Starting the next game does not create or modify the previous record.
7. Show the statistics panel after both outcomes. A loss visibly resets the
   current streak to zero.
8. The current streak is the number of consecutive wins at the end of the log.
   The maximum streak is the longest consecutive run of wins anywhere in it.
9. The attempt distribution counts wins only. Losses still contribute to games
   played and win percentage.
10. Round win percentage to the nearest whole percent and show `0%` when there
    are no games.

## Version-One Data Model

Add pure domain models under `src/mvc/models/game_history.rs`:

```rust
struct GameHistory {
    schema_version: u16,
    games: Vec<GameRecord>,
}

struct GameRecord {
    game_id: u64,
    outcome: GameOutcome,
    attempts: u8,
}

enum GameOutcome {
    Won,
    Lost,
}

struct GameStats {
    played: u64,
    won: u64,
    lost: u64,
    win_percent: u8,
    current_streak: u64,
    max_streak: u64,
    wins_by_attempt: [u64; 6],
}
```

`game_id` is a checked, monotonically increasing sequence starting at one. It
provides stable ordering without trusting the Kindle clock.

Do not store timestamps, guesses, answers, or precomputed statistics in version
one. They are not needed for the requested statistics, Kindle wall clocks are
not guaranteed to be reliable, and the extra fields would retain more personal
play data than necessary.

Use a versioned JSON document:

```json
{
  "schema_version": 1,
  "games": [
    {
      "game_id": 1,
      "outcome": "won",
      "attempts": 4
    },
    {
      "game_id": 2,
      "outcome": "lost",
      "attempts": 6
    }
  ]
}
```

Add narrowly configured `serde` and `serde_json` dependencies. Validate the
version, strictly increasing IDs, valid attempt ranges, and loss attempt count
after deserialization rather than trusting file contents.

## Persistent Location

Never resolve history relative to the process working directory; the Ferrink
launcher does not document that directory as stable.

Ferrink clears inherited environment variables, applies only the environment
declared in the application manifest, and launches from the executable
directory. Use this path contract:

1. `SLINT_WORDLE_STATE_DIR` is the explicit override and should be declared in
   the Kindle application manifest.
2. The Kindle default is `/var/local/ferrink/slint-wordle`, consistent with
   Ferrink's existing durable state paths.
3. Desktop development should use
   `${XDG_DATA_HOME}/slint-wordle` when set, otherwise
   `${HOME}/.local/share/slint-wordle`.
4. The file within that directory is `game-history-v1.json`.
5. Unit tests use a unique temporary directory and never a production path.

Never fall back to the executable directory or current working directory. If
the launcher or manifest lives in a different repository, its environment
change is a required deployment task and should land before the history-enabled
binary is installed.

## Repository and Write Design

Follow the existing MVC repository structure:

- add `GameHistoryRepository` under
  `src/mvc/repositories/traits/game_history_repository.rs`;
- add the production file implementation under
  `src/mvc/repositories/game_history_repository_impl.rs`;
- add an in-memory implementation for controller tests and MCP/demo mode; and
- construct and inject the repository from `src/lib.rs`.

The repository API should load the full validated history and atomically save a
replacement. Only one application process is expected to write the file.

For every completed game:

1. Clone the in-memory history and append the new validated record.
2. Serialize the updated history.
3. Create the state directory if it does not exist.
4. Write to a uniquely named temporary file in the same directory with mode
   `0600`.
5. Flush and `sync_all` the temporary file.
6. Rename it over `game-history-v1.json`.
7. Sync the parent directory.
8. Keep the updated history in the controller for the current session. Mark it
   durable on success or unsaved on failure.

This performs one small durable write per completed game, not one per
keystroke. Rewriting the complete file keeps recovery and migration simple; a
16 MiB input guard still permits far more games than a person can reasonably
play while preventing unbounded allocation from a damaged file.

If the file is missing, start with an empty version-one history. If it is
malformed, too large, has an unsupported version, or fails validation:

- preserve the original file without overwriting it;
- continue allowing games to be played;
- report the error to stderr;
- expose a `Stats unavailable` or `Stats not saved` state to the UI; and
- retry only after a new application start or an explicit recovery path.

A transient save failure must leave the last-known-good disk file intact while
the just-finished game remains in the session statistics. A later completed
game may retry the full pending history. A save failure must never turn a win
into a loss or prevent the user from starting another game. History that is
still unsaved when the process exits may be lost, which is why the UI warning
must remain visible.

## Controller Integration

Extend `WordleController` to own the loaded history and repository. The terminal
branch already exists in `submit_guess`, and `current_row + 1` is the correct
accepted-attempt count at that point.

The controller must:

- append only on the `Playing -> Won` or `Playing -> Lost` transition;
- guard against a repeated submit callback recording the terminal game twice;
- expose the current `GameStats` and history persistence status;
- calculate stats with checked integer arithmetic; and
- retain the existing game result even if persistence fails.

Keep `new_game` responsible only for resetting the active board. It must not
reset history or streaks.

The current `bool` return from `submit_guess` may remain if persistence status
is exposed separately. If richer UI feedback is easier with an enum, replace it
with an explicit result such as `Rejected`, `Accepted`, and
`Completed { history_saved: bool }`; avoid inferring completion in the adapter.

## Demo and Test Isolation

`SLINT_MCP_PORT` currently starts an autoplay demo that reaches a real win.
That path must use an in-memory/no-op history repository so screenshots and UI
automation never alter the player's actual statistics.

The current `main` function calls `init()` before it checks `SLINT_MCP_PORT`.
Refactor startup to select production versus demo persistence before controller
construction, then pass the selected repository into `init`. Changing only
`start_demo` is too late because the production history would already be open.

Controller unit tests must also inject in-memory history. File repository tests
must use temporary directories and must not read environment-dependent
production paths.

## Post-Game UI

Extend `src/ui/wordle_adapter.rs` to push a typed stats snapshot into
`src/ui/wordle_view.slint`. Add a dedicated panel between the completed grid and
the controls instead of squeezing all statistics into the existing one-line
message.

Suggested monochrome, ASCII-only content:

```text
WON IN 4/6
PLAYED 12   WON 9   LOST 3
WIN 75%   STREAK 2   BEST 5
GUESSES 1:0  2:1  3:4  4:2  5:2  6:0
```

For a loss, retain the current answer reveal and use `STREAK 0`. Keep the New
Game button visible. Continue ignoring keyboard input after game over.

The panel must:

- remain legible at the current 758 by 1024 reference size;
- use the embedded font and glyphs already known to work on Kindle;
- use black, white, borders, and spacing rather than color alone;
- avoid animation and unnecessary E Ink refreshes; and
- show a short persistence warning without hiding the game result.

## Implementation Sequence

1. [x] Add the domain models, validation, and pure stats calculation.
2. [x] Add the repository trait, in-memory repository, file repository, path
   resolver, atomic writes, and serialization dependencies.
3. [x] Inject history through `src/lib.rs` and record the terminal transition in
   `WordleController`.
4. [x] Add the adapter properties and post-game Slint panel.
5. [x] Isolate MCP/demo mode from production history.
6. [ ] Update the Kindle launcher with `SLINT_WORDLE_STATE_DIR` if the default
   durable path is not suitable for the deployed manifest.
7. [x] Exercise desktop repository persistence and cross-compile the ARM musl
   release.
8. [ ] Verify rendering and persistence across application and device restarts
   on the KOA3.

## Test Matrix

### Model and statistics

- Empty history produces zero values without division by zero.
- A win in each attempt bucket from one through six increments the right bin.
- Losses increment played/lost but not the attempt distribution.
- Consecutive wins grow current and maximum streaks.
- A loss resets current streak but preserves maximum streak.
- A later streak can replace the previous maximum.
- Win percentage rounds as documented.
- Invalid attempts, non-monotonic IDs, overflow, and inconsistent losses fail
  validation.

### Controller

- A first- through sixth-attempt win records exactly that attempt count.
- Six accepted wrong guesses record one loss.
- Incomplete, invalid, and repeated words create no record.
- Repeated submit callbacks after completion create no duplicate.
- Starting another game preserves history.
- A repository failure leaves gameplay functional and exposes a warning.
- A later successful save flushes pending session records exactly once.

### File repository

- Missing file loads as empty history.
- Save and reload round-trip every field.
- An interrupted temporary write leaves the previous file readable.
- Malformed, truncated, oversized, and unknown-version files are preserved and
  rejected.
- Failed writes leave the last-known-good disk file unchanged.
- Temporary files are cleaned up after a failed write where possible.
- Unix file permissions are owner-only.

### UI and device

- Win and loss panels display correct values.
- The attempt distribution matches the underlying records.
- Long numeric values do not overlap at 758 by 1024.
- MCP/demo runs do not change the production file.
- History survives application exit, relaunch, device reboot, and binary
  replacement on Kindle.

## Acceptance Criteria

- Every completed game creates exactly one validated session record and, while
  storage is healthy, exactly one durable record.
- No abandoned or rejected game creates a record.
- Post-game stats include the game that just ended and are mathematically
  correct.
- Current streak resets on a loss and survives application/device restarts.
- Corrupt or unwritable history never crashes the game or destroys the last
  known-good file.
- Tests and demo mode cannot contaminate real history.
- Desktop and Kindle paths are deterministic and independent of the working
  directory.
- `cargo fmt --check`, workspace tests, pedantic Clippy, `cargo audit`, and the
  ARM musl release build pass before deployment.

## Non-Goals for Version One

- Resuming an unfinished board after restart.
- Daily puzzles or calendar-based streak rules.
- Multiple player profiles.
- Cloud synchronization.
- Editing, deleting, importing, or exporting history in the UI.
- Retaining answers or individual guesses.
