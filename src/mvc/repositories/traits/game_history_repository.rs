use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::mvc::models::{GameHistory, GameHistoryValidationError};

/// Durable storage boundary for the complete game-history document.
pub trait GameHistoryRepository {
    /// Loads and validates the last durable history, or returns an empty
    /// history when storage has not been created yet.
    ///
    /// # Errors
    ///
    /// Returns an error when storage cannot be read, decoded, or validated.
    fn load(&self) -> Result<GameHistory, GameHistoryRepositoryError>;

    /// Replaces the durable history with a validated snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, encoding, or durable storage fails.
    fn save(&self, history: &GameHistory) -> Result<(), GameHistoryRepositoryError>;
}

/// An error encountered while locating, reading, validating, or saving history.
#[derive(Debug)]
pub enum GameHistoryRepositoryError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Json {
        operation: &'static str,
        path: PathBuf,
        source: serde_json::Error,
    },
    Validation(GameHistoryValidationError),
    TooLarge {
        path: PathBuf,
        actual_bytes: u64,
        maximum_bytes: u64,
    },
    InvalidStateDirectory {
        source_name: &'static str,
        path: PathBuf,
        reason: &'static str,
    },
    StateDirectoryUnavailable,
    Unavailable(String),
}

impl fmt::Display for GameHistoryRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "could not {operation} game history at {}: {source}",
                path.display()
            ),
            Self::Json {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "could not {operation} game history at {}: {source}",
                path.display()
            ),
            Self::Validation(source) => {
                write!(formatter, "invalid game history: {source}")
            }
            Self::TooLarge {
                path,
                actual_bytes,
                maximum_bytes,
            } => write!(
                formatter,
                "game history at {} is {actual_bytes} bytes; the limit is \
                 {maximum_bytes} bytes",
                path.display()
            ),
            Self::InvalidStateDirectory {
                source_name,
                path,
                reason,
            } => write!(
                formatter,
                "{source_name} does not name a usable state directory ({}): \
                 {reason}",
                path.display()
            ),
            Self::StateDirectoryUnavailable => formatter.write_str(
                "no game-history state directory is available; set \
                 SLINT_WORDLE_STATE_DIR, XDG_DATA_HOME, or HOME",
            ),
            Self::Unavailable(message) => {
                write!(formatter, "game-history storage is unavailable: {message}")
            }
        }
    }
}

impl Error for GameHistoryRepositoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Validation(source) => Some(source),
            Self::TooLarge { .. }
            | Self::InvalidStateDirectory { .. }
            | Self::StateDirectoryUnavailable
            | Self::Unavailable(_) => None,
        }
    }
}

impl From<GameHistoryValidationError> for GameHistoryRepositoryError {
    fn from(source: GameHistoryValidationError) -> Self {
        Self::Validation(source)
    }
}
