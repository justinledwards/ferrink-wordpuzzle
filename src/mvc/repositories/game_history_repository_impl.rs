use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use super::traits::{GameHistoryRepository, GameHistoryRepositoryError};
use crate::mvc::models::GameHistory;

pub const GAME_HISTORY_FILE_NAME: &str = "game-history-v1.json";
pub const MAX_GAME_HISTORY_BYTES: u64 = 16 * 1024 * 1024;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// JSON file-backed history storage with atomic replacement.
#[derive(Clone, Debug)]
pub struct FileGameHistoryRepository {
    path: PathBuf,
}

impl FileGameHistoryRepository {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn parent_directory(&self) -> Result<&Path, GameHistoryRepositoryError> {
        let Some(parent) = self.path.parent() else {
            return Err(invalid_file_path(&self.path));
        };
        if parent.as_os_str().is_empty() {
            return Err(invalid_file_path(&self.path));
        }
        Ok(parent)
    }
}

impl GameHistoryRepository for FileGameHistoryRepository {
    fn load(&self) -> Result<GameHistory, GameHistoryRepositoryError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Ok(GameHistory::empty());
            }
            Err(source) => {
                return Err(io_error("open", &self.path, source));
            }
        };

        let metadata = file
            .metadata()
            .map_err(|source| io_error("inspect", &self.path, source))?;
        if metadata.len() > MAX_GAME_HISTORY_BYTES {
            return Err(GameHistoryRepositoryError::TooLarge {
                path: self.path.clone(),
                actual_bytes: metadata.len(),
                maximum_bytes: MAX_GAME_HISTORY_BYTES,
            });
        }

        let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
        Read::by_ref(&mut file)
            .take(MAX_GAME_HISTORY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| io_error("read", &self.path, source))?;
        let actual_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual_bytes > MAX_GAME_HISTORY_BYTES {
            return Err(GameHistoryRepositoryError::TooLarge {
                path: self.path.clone(),
                actual_bytes,
                maximum_bytes: MAX_GAME_HISTORY_BYTES,
            });
        }

        let history: GameHistory =
            serde_json::from_slice(&bytes).map_err(|source| GameHistoryRepositoryError::Json {
                operation: "decode",
                path: self.path.clone(),
                source,
            })?;
        history.validate()?;
        Ok(history)
    }

    fn save(&self, history: &GameHistory) -> Result<(), GameHistoryRepositoryError> {
        history.validate()?;
        let mut encoded = serde_json::to_vec_pretty(history).map_err(|source| {
            GameHistoryRepositoryError::Json {
                operation: "encode",
                path: self.path.clone(),
                source,
            }
        })?;
        encoded.push(b'\n');
        let encoded_len = u64::try_from(encoded.len()).unwrap_or(u64::MAX);
        if encoded_len > MAX_GAME_HISTORY_BYTES {
            return Err(GameHistoryRepositoryError::TooLarge {
                path: self.path.clone(),
                actual_bytes: encoded_len,
                maximum_bytes: MAX_GAME_HISTORY_BYTES,
            });
        }

        let parent = self.parent_directory()?;
        fs::create_dir_all(parent)
            .map_err(|source| io_error("create state directory for", parent, source))?;

        let (temporary_path, mut temporary_file) = create_temporary_file(parent, &self.path)?;
        let write_result = (|| -> io::Result<()> {
            #[cfg(unix)]
            temporary_file.set_permissions(fs::Permissions::from_mode(0o600))?;
            temporary_file.write_all(&encoded)?;
            temporary_file.flush()?;
            temporary_file.sync_all()
        })();

        if let Err(source) = write_result {
            drop(temporary_file);
            remove_temporary_file(&temporary_path);
            return Err(io_error("write temporary", &temporary_path, source));
        }
        drop(temporary_file);

        if let Err(source) = fs::rename(&temporary_path, &self.path) {
            remove_temporary_file(&temporary_path);
            return Err(io_error("atomically replace", &self.path, source));
        }

        sync_directory(parent).map_err(|source| io_error("sync state directory", parent, source))
    }
}

fn invalid_file_path(path: &Path) -> GameHistoryRepositoryError {
    GameHistoryRepositoryError::InvalidStateDirectory {
        source_name: "game-history file path",
        path: path.to_path_buf(),
        reason: "the file must have an explicit parent directory",
    }
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> GameHistoryRepositoryError {
    GameHistoryRepositoryError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

fn create_temporary_file(
    parent: &Path,
    destination: &Path,
) -> Result<(PathBuf, File), GameHistoryRepositoryError> {
    let file_name = destination
        .file_name()
        .unwrap_or_else(|| OsStr::new(GAME_HISTORY_FILE_NAME));

    for _ in 0..128 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary_name = format!(
            ".{}.tmp-{}-{sequence}",
            file_name.to_string_lossy(),
            std::process::id()
        );
        let temporary_path = parent.join(temporary_name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);

        match options.open(&temporary_path) {
            Ok(file) => return Ok((temporary_path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(io_error("create temporary", &temporary_path, source));
            }
        }
    }

    Err(io_error(
        "create temporary",
        parent,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary filename",
        ),
    ))
}

fn remove_temporary_file(path: &Path) {
    if let Err(source) = fs::remove_file(path)
        && source.kind() != io::ErrorKind::NotFound
    {
        eprintln!(
            "could not remove temporary game-history file {}: {source}",
            path.display()
        );
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Resolves the production history file without consulting the working
/// directory.
///
/// # Errors
///
/// Returns an error when an explicit state directory is empty or relative, or
/// when desktop startup has neither an XDG data directory nor a home directory.
pub fn resolve_game_history_path() -> Result<PathBuf, GameHistoryRepositoryError> {
    if let Some(directory) = env::var_os("SLINT_WORDLE_STATE_DIR") {
        return history_path_from_directory("SLINT_WORDLE_STATE_DIR", PathBuf::from(directory));
    }

    #[cfg(all(target_arch = "arm", target_os = "linux", target_env = "musl"))]
    {
        history_path_from_directory(
            "Kindle default",
            PathBuf::from("/var/local/ferrink/slint-wordle"),
        )
    }

    #[cfg(not(all(target_arch = "arm", target_os = "linux", target_env = "musl")))]
    {
        resolve_desktop_history_path(
            env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            env::var_os("HOME").map(PathBuf::from),
        )
    }
}

#[cfg(any(
    test,
    not(all(target_arch = "arm", target_os = "linux", target_env = "musl"))
))]
fn resolve_desktop_history_path(
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, GameHistoryRepositoryError> {
    if let Some(directory) = xdg_data_home.filter(|path| !path.as_os_str().is_empty()) {
        return history_path_from_directory("XDG_DATA_HOME", directory.join("slint-wordle"));
    }

    if let Some(directory) = home.filter(|path| !path.as_os_str().is_empty()) {
        return history_path_from_directory("HOME", directory.join(".local/share/slint-wordle"));
    }

    Err(GameHistoryRepositoryError::StateDirectoryUnavailable)
}

fn history_path_from_directory(
    source_name: &'static str,
    directory: PathBuf,
) -> Result<PathBuf, GameHistoryRepositoryError> {
    if directory.as_os_str().is_empty() {
        return Err(GameHistoryRepositoryError::InvalidStateDirectory {
            source_name,
            path: directory,
            reason: "the value is empty",
        });
    }
    if !directory.is_absolute() {
        return Err(GameHistoryRepositoryError::InvalidStateDirectory {
            source_name,
            path: directory,
            reason: "the value must be absolute",
        });
    }
    Ok(directory.join(GAME_HISTORY_FILE_NAME))
}

#[derive(Clone, Debug)]
struct InMemoryState {
    history: GameHistory,
    save_failure: Option<String>,
}

/// Shared in-memory history storage for tests and demo mode.
#[derive(Clone, Debug)]
pub struct InMemoryGameHistoryRepository {
    state: Arc<Mutex<InMemoryState>>,
}

impl InMemoryGameHistoryRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::with_history(GameHistory::empty())
    }

    #[must_use]
    pub fn with_history(history: GameHistory) -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemoryState {
                history,
                save_failure: None,
            })),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> GameHistory {
        self.lock_state().history.clone()
    }

    pub fn set_save_failure(&self, message: impl Into<String>) {
        self.lock_state().save_failure = Some(message.into());
    }

    pub fn clear_save_failure(&self) {
        self.lock_state().save_failure = None;
    }

    pub fn set_save_should_fail(&self, should_fail: bool) {
        if should_fail {
            self.set_save_failure("injected in-memory save failure");
        } else {
            self.clear_save_failure();
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, InMemoryState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Default for InMemoryGameHistoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl GameHistoryRepository for InMemoryGameHistoryRepository {
    fn load(&self) -> Result<GameHistory, GameHistoryRepositoryError> {
        let history = self.lock_state().history.clone();
        history.validate()?;
        Ok(history)
    }

    fn save(&self, history: &GameHistory) -> Result<(), GameHistoryRepositoryError> {
        history.validate()?;
        let mut state = self.lock_state();
        if let Some(message) = &state.save_failure {
            return Err(GameHistoryRepositoryError::Unavailable(message.clone()));
        }
        state.history = history.clone();
        Ok(())
    }
}

/// A repository used when startup could not resolve or initialize persistence.
#[derive(Clone, Debug)]
pub struct UnavailableGameHistoryRepository {
    message: String,
}

impl UnavailableGameHistoryRepository {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl GameHistoryRepository for UnavailableGameHistoryRepository {
    fn load(&self) -> Result<GameHistory, GameHistoryRepositoryError> {
        Err(GameHistoryRepositoryError::Unavailable(
            self.message.clone(),
        ))
    }

    fn save(&self, _history: &GameHistory) -> Result<(), GameHistoryRepositoryError> {
        Err(GameHistoryRepositoryError::Unavailable(
            self.message.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvc::models::{GameOutcome, GameRecord};

    static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "slint-wordle-history-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn history_path(&self) -> PathBuf {
            self.path.join(GAME_HISTORY_FILE_NAME)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_history() -> GameHistory {
        let mut history = GameHistory::empty();
        history.append(GameOutcome::Won, 3).unwrap();
        history.append(GameOutcome::Lost, 6).unwrap();
        history
    }

    #[test]
    fn missing_file_loads_empty_history() {
        let directory = TestDirectory::new();
        let repository = FileGameHistoryRepository::new(directory.history_path());

        assert_eq!(repository.load().unwrap(), GameHistory::empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let directory = TestDirectory::new();
        let repository = FileGameHistoryRepository::new(directory.history_path());
        let history = sample_history();

        repository.save(&history).unwrap();

        assert_eq!(repository.load().unwrap(), history);
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_owner_only() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let repository = FileGameHistoryRepository::new(path.clone());

        repository.save(&sample_history()).unwrap();

        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn malformed_file_is_rejected_without_modification() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let original = b"{\"schema_version\":1,\"games\":[";
        fs::write(&path, original).unwrap();
        let repository = FileGameHistoryRepository::new(path.clone());

        assert!(matches!(
            repository.load(),
            Err(GameHistoryRepositoryError::Json { .. })
        ));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn unsupported_version_is_rejected_without_modification() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let original = br#"{"schema_version":99,"games":[]}"#;
        fs::write(&path, original).unwrap();
        let repository = FileGameHistoryRepository::new(path.clone());

        assert!(matches!(
            repository.load(),
            Err(GameHistoryRepositoryError::Validation(_))
        ));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn oversized_file_is_rejected_without_being_read_or_modified() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let file = File::create(&path).unwrap();
        file.set_len(MAX_GAME_HISTORY_BYTES + 1).unwrap();
        drop(file);
        let repository = FileGameHistoryRepository::new(path.clone());

        assert!(matches!(
            repository.load(),
            Err(GameHistoryRepositoryError::TooLarge { .. })
        ));
        assert_eq!(
            fs::metadata(path).unwrap().len(),
            MAX_GAME_HISTORY_BYTES + 1
        );
    }

    #[test]
    fn invalid_save_leaves_last_known_good_file_unchanged() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let repository = FileGameHistoryRepository::new(path.clone());
        repository.save(&sample_history()).unwrap();
        let original = fs::read(&path).unwrap();
        let invalid: GameHistory = serde_json::from_str(
            r#"{"schema_version":1,"games":[
                {"game_id":1,"outcome":"lost","attempts":5}
            ]}"#,
        )
        .unwrap();

        assert!(matches!(
            repository.save(&invalid),
            Err(GameHistoryRepositoryError::Validation(_))
        ));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn unrelated_temporary_file_does_not_replace_good_history() {
        let directory = TestDirectory::new();
        let path = directory.history_path();
        let repository = FileGameHistoryRepository::new(path.clone());
        repository.save(&sample_history()).unwrap();
        let original = fs::read(&path).unwrap();
        fs::write(
            directory.path.join(".game-history-v1.json.tmp-interrupted"),
            b"partial",
        )
        .unwrap();

        assert_eq!(repository.load().unwrap(), sample_history());
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn in_memory_clones_share_saved_state_and_failure_control() {
        let repository = InMemoryGameHistoryRepository::new();
        let clone = repository.clone();
        let history = sample_history();
        clone.save(&history).unwrap();
        assert_eq!(repository.snapshot(), history);

        repository.set_save_failure("disk full");
        let mut newer = history.clone();
        newer.append(GameOutcome::Won, 2).unwrap();
        assert!(matches!(
            clone.save(&newer),
            Err(GameHistoryRepositoryError::Unavailable(message))
                if message == "disk full"
        ));
        assert_eq!(repository.snapshot(), history);

        clone.clear_save_failure();
        clone.save(&newer).unwrap();
        assert_eq!(repository.snapshot(), newer);
    }

    #[test]
    fn in_memory_with_history_loads_snapshot() {
        let history = GameHistory::from_records(vec![GameRecord {
            game_id: 1,
            outcome: GameOutcome::Won,
            attempts: 4,
        }])
        .unwrap();
        let repository = InMemoryGameHistoryRepository::with_history(history.clone());

        assert_eq!(repository.load().unwrap(), history);
    }

    #[test]
    fn unavailable_repository_reports_load_and_save_errors() {
        let repository = UnavailableGameHistoryRepository::new("HOME is missing");

        assert!(repository.load().is_err());
        assert!(repository.save(&GameHistory::empty()).is_err());
    }

    #[test]
    fn desktop_resolution_prefers_xdg_then_home() {
        let xdg = resolve_desktop_history_path(
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/home")),
        )
        .unwrap();
        assert_eq!(
            xdg,
            PathBuf::from("/tmp/xdg/slint-wordle").join(GAME_HISTORY_FILE_NAME)
        );

        let home = resolve_desktop_history_path(None, Some(PathBuf::from("/tmp/home"))).unwrap();
        assert_eq!(
            home,
            PathBuf::from("/tmp/home/.local/share/slint-wordle").join(GAME_HISTORY_FILE_NAME)
        );
    }

    #[test]
    fn path_resolution_rejects_relative_and_missing_directories() {
        assert!(matches!(
            history_path_from_directory("SLINT_WORDLE_STATE_DIR", PathBuf::from("relative")),
            Err(GameHistoryRepositoryError::InvalidStateDirectory { .. })
        ));
        assert!(matches!(
            resolve_desktop_history_path(None, None),
            Err(GameHistoryRepositoryError::StateDirectoryUnavailable)
        ));
    }
}
