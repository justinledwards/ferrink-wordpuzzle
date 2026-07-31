use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The on-disk schema understood by this version of the application.
pub const GAME_HISTORY_SCHEMA_VERSION: u16 = 1;

/// The terminal outcome of a completed game.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GameOutcome {
    Won,
    Lost,
}

/// One completed game in chronological order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameRecord {
    pub game_id: u64,
    pub outcome: GameOutcome,
    pub attempts: u8,
}

/// Statistics derived from the complete game log.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GameStats {
    pub played: u64,
    pub won: u64,
    pub lost: u64,
    pub win_percent: u8,
    pub current_streak: u64,
    pub max_streak: u64,
    pub wins_by_attempt: [u64; 6],
}

/// A versioned, ordered log of completed games.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameHistory {
    schema_version: u16,
    games: Vec<GameRecord>,
}

impl Default for GameHistory {
    fn default() -> Self {
        Self {
            schema_version: GAME_HISTORY_SCHEMA_VERSION,
            games: Vec::new(),
        }
    }
}

impl GameHistory {
    /// Creates an empty history using the current schema.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates and validates a history from existing records.
    ///
    /// This is primarily useful for migrations and deterministic tests. Normal
    /// gameplay should add records through [`Self::append`].
    ///
    /// # Errors
    ///
    /// Returns an error when the records violate the version-one ordering or
    /// outcome-specific attempt rules.
    pub fn from_records(games: Vec<GameRecord>) -> Result<Self, GameHistoryValidationError> {
        let history = Self {
            schema_version: GAME_HISTORY_SCHEMA_VERSION,
            games,
        };
        history.validate()?;
        Ok(history)
    }

    /// Returns the serialized schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Returns completed games in chronological order.
    #[must_use]
    pub fn records(&self) -> &[GameRecord] {
        &self.games
    }

    /// Adds one completed game and assigns its stable sequence ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the existing history is invalid, the new
    /// outcome/attempt pair is invalid, or the sequence ID would overflow.
    pub fn append(
        &mut self,
        outcome: GameOutcome,
        attempts: u8,
    ) -> Result<(), GameHistoryValidationError> {
        self.validate()?;

        let game_id = match self.games.last() {
            Some(last) => last
                .game_id
                .checked_add(1)
                .ok_or(GameHistoryValidationError::GameIdOverflow)?,
            None => 1,
        };
        let record = GameRecord {
            game_id,
            outcome,
            attempts,
        };
        validate_record(&record)?;
        self.games.push(record);
        Ok(())
    }

    /// Verifies the schema, ordering, and outcome-specific attempt rules.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported schema, invalid game ID ordering,
    /// or an outcome with an invalid attempt count.
    pub fn validate(&self) -> Result<(), GameHistoryValidationError> {
        if self.schema_version != GAME_HISTORY_SCHEMA_VERSION {
            return Err(GameHistoryValidationError::UnsupportedSchemaVersion {
                expected: GAME_HISTORY_SCHEMA_VERSION,
                found: self.schema_version,
            });
        }

        let mut previous_id = None;
        for record in &self.games {
            if let Some(previous) = previous_id {
                if record.game_id <= previous {
                    return Err(GameHistoryValidationError::NonMonotonicGameId {
                        previous,
                        current: record.game_id,
                    });
                }
            } else if record.game_id != 1 {
                return Err(GameHistoryValidationError::FirstGameIdMustBeOne {
                    found: record.game_id,
                });
            }

            validate_record(record)?;
            previous_id = Some(record.game_id);
        }
        Ok(())
    }

    /// Derives aggregate statistics without storing counters separately.
    #[must_use]
    pub fn stats(&self) -> GameStats {
        let mut stats = GameStats::default();
        let mut running_streak = 0_u64;

        for record in &self.games {
            stats.played = stats.played.saturating_add(1);
            match record.outcome {
                GameOutcome::Won => {
                    stats.won = stats.won.saturating_add(1);
                    running_streak = running_streak.saturating_add(1);
                    stats.max_streak = stats.max_streak.max(running_streak);

                    if (1..=6).contains(&record.attempts) {
                        let index = usize::from(record.attempts - 1);
                        stats.wins_by_attempt[index] =
                            stats.wins_by_attempt[index].saturating_add(1);
                    }
                }
                GameOutcome::Lost => {
                    running_streak = 0;
                }
            }
        }

        stats.lost = stats.played.saturating_sub(stats.won);
        stats.current_streak = running_streak;
        if stats.played != 0 {
            let numerator = u128::from(stats.won) * 100 + u128::from(stats.played) / 2;
            let rounded = numerator / u128::from(stats.played);
            stats.win_percent = u8::try_from(rounded).unwrap_or(100);
        }
        stats
    }
}

fn validate_record(record: &GameRecord) -> Result<(), GameHistoryValidationError> {
    match record.outcome {
        GameOutcome::Won if !(1..=6).contains(&record.attempts) => {
            Err(GameHistoryValidationError::InvalidWinAttempts {
                game_id: record.game_id,
                attempts: record.attempts,
            })
        }
        GameOutcome::Lost if record.attempts != 6 => {
            Err(GameHistoryValidationError::InvalidLossAttempts {
                game_id: record.game_id,
                attempts: record.attempts,
            })
        }
        GameOutcome::Won | GameOutcome::Lost => Ok(()),
    }
}

/// Why a history document or proposed record is invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameHistoryValidationError {
    UnsupportedSchemaVersion { expected: u16, found: u16 },
    FirstGameIdMustBeOne { found: u64 },
    NonMonotonicGameId { previous: u64, current: u64 },
    InvalidWinAttempts { game_id: u64, attempts: u8 },
    InvalidLossAttempts { game_id: u64, attempts: u8 },
    GameIdOverflow,
}

impl fmt::Display for GameHistoryValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { expected, found } => write!(
                formatter,
                "unsupported game-history schema version {found}; expected {expected}"
            ),
            Self::FirstGameIdMustBeOne { found } => {
                write!(formatter, "first game ID must be 1, found {found}")
            }
            Self::NonMonotonicGameId { previous, current } => write!(
                formatter,
                "game IDs must increase strictly; {current} follows {previous}"
            ),
            Self::InvalidWinAttempts { game_id, attempts } => write!(
                formatter,
                "won game {game_id} has {attempts} attempts; expected 1 through 6"
            ),
            Self::InvalidLossAttempts { game_id, attempts } => write!(
                formatter,
                "lost game {game_id} has {attempts} attempts; expected 6"
            ),
            Self::GameIdOverflow => formatter.write_str("cannot assign a game ID after u64::MAX"),
        }
    }
}

impl Error for GameHistoryValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn append(history: &mut GameHistory, outcome: GameOutcome, attempts: u8) {
        history.append(outcome, attempts).unwrap();
    }

    #[test]
    fn empty_history_has_zero_statistics() {
        let history = GameHistory::empty();

        assert_eq!(history.schema_version(), GAME_HISTORY_SCHEMA_VERSION);
        assert!(history.records().is_empty());
        assert_eq!(history.stats(), GameStats::default());
        assert_eq!(history.validate(), Ok(()));
    }

    #[test]
    fn append_assigns_increasing_ids() {
        let mut history = GameHistory::empty();
        append(&mut history, GameOutcome::Won, 2);
        append(&mut history, GameOutcome::Lost, 6);

        assert_eq!(history.records()[0].game_id, 1);
        assert_eq!(history.records()[1].game_id, 2);
    }

    #[test]
    fn every_winning_attempt_bucket_is_counted() {
        let mut history = GameHistory::empty();
        for attempts in 1..=6 {
            append(&mut history, GameOutcome::Won, attempts);
        }

        let stats = history.stats();
        assert_eq!(stats.played, 6);
        assert_eq!(stats.won, 6);
        assert_eq!(stats.lost, 0);
        assert_eq!(stats.win_percent, 100);
        assert_eq!(stats.current_streak, 6);
        assert_eq!(stats.max_streak, 6);
        assert_eq!(stats.wins_by_attempt, [1; 6]);
    }

    #[test]
    fn losses_reset_current_streak_and_preserve_best() {
        let mut history = GameHistory::empty();
        for outcome in [
            GameOutcome::Won,
            GameOutcome::Won,
            GameOutcome::Lost,
            GameOutcome::Won,
            GameOutcome::Won,
            GameOutcome::Won,
            GameOutcome::Lost,
        ] {
            let attempts = if outcome == GameOutcome::Won { 3 } else { 6 };
            append(&mut history, outcome, attempts);
        }

        let stats = history.stats();
        assert_eq!(stats.played, 7);
        assert_eq!(stats.won, 5);
        assert_eq!(stats.lost, 2);
        assert_eq!(stats.win_percent, 71);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.max_streak, 3);
        assert_eq!(stats.wins_by_attempt, [0, 0, 5, 0, 0, 0]);
    }

    #[test]
    fn win_percentage_rounds_to_nearest_whole_number() {
        let mut one_of_three = GameHistory::empty();
        append(&mut one_of_three, GameOutcome::Won, 1);
        append(&mut one_of_three, GameOutcome::Lost, 6);
        append(&mut one_of_three, GameOutcome::Lost, 6);
        assert_eq!(one_of_three.stats().win_percent, 33);

        let mut two_of_three = GameHistory::empty();
        append(&mut two_of_three, GameOutcome::Won, 1);
        append(&mut two_of_three, GameOutcome::Won, 2);
        append(&mut two_of_three, GameOutcome::Lost, 6);
        assert_eq!(two_of_three.stats().win_percent, 67);
    }

    #[test]
    fn outcome_specific_attempt_rules_are_enforced() {
        let mut history = GameHistory::empty();
        assert!(matches!(
            history.append(GameOutcome::Won, 0),
            Err(GameHistoryValidationError::InvalidWinAttempts { .. })
        ));
        assert!(matches!(
            history.append(GameOutcome::Won, 7),
            Err(GameHistoryValidationError::InvalidWinAttempts { .. })
        ));
        assert!(matches!(
            history.append(GameOutcome::Lost, 5),
            Err(GameHistoryValidationError::InvalidLossAttempts { .. })
        ));
        assert!(history.records().is_empty());
    }

    #[test]
    fn validation_rejects_unknown_schema_and_bad_ids() {
        let unknown: GameHistory =
            serde_json::from_str(r#"{"schema_version":2,"games":[]}"#).unwrap();
        assert!(matches!(
            unknown.validate(),
            Err(GameHistoryValidationError::UnsupportedSchemaVersion { found: 2, .. })
        ));

        let wrong_first: GameHistory = serde_json::from_str(
            r#"{"schema_version":1,"games":[
                {"game_id":2,"outcome":"won","attempts":1}
            ]}"#,
        )
        .unwrap();
        assert!(matches!(
            wrong_first.validate(),
            Err(GameHistoryValidationError::FirstGameIdMustBeOne { found: 2 })
        ));

        let duplicate: GameHistory = serde_json::from_str(
            r#"{"schema_version":1,"games":[
                {"game_id":1,"outcome":"won","attempts":1},
                {"game_id":1,"outcome":"lost","attempts":6}
            ]}"#,
        )
        .unwrap();
        assert!(matches!(
            duplicate.validate(),
            Err(GameHistoryValidationError::NonMonotonicGameId { .. })
        ));
    }

    #[test]
    fn append_detects_game_id_overflow() {
        let records = vec![
            GameRecord {
                game_id: 1,
                outcome: GameOutcome::Won,
                attempts: 1,
            },
            GameRecord {
                game_id: u64::MAX,
                outcome: GameOutcome::Lost,
                attempts: 6,
            },
        ];
        let mut history = GameHistory::from_records(records).unwrap();

        assert_eq!(
            history.append(GameOutcome::Won, 1),
            Err(GameHistoryValidationError::GameIdOverflow)
        );
    }

    #[test]
    fn serde_uses_stable_version_one_shape() {
        let mut history = GameHistory::empty();
        append(&mut history, GameOutcome::Won, 4);

        let value = serde_json::to_value(&history).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["games"][0]["game_id"], 1);
        assert_eq!(value["games"][0]["outcome"], "won");
        assert_eq!(value["games"][0]["attempts"], 4);
        assert!(
            serde_json::from_value::<GameHistory>(value)
                .unwrap()
                .validate()
                .is_ok()
        );
    }
}
