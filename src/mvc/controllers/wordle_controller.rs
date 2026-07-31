use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rand::seq::IndexedRandom;

use crate::mvc;

/// Status of a guessed letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetterStatus {
    Unknown,
    Absent,
    Present,
    Correct,
}

/// Overall game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Won,
    Lost,
}

/// Availability and durability of the game history for this session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryStatus {
    Ready,
    Unsaved,
    Unavailable,
}

const WORD_LEN: usize = 5;
const MAX_GUESSES: usize = 6;

/// Controller managing Wordle game logic.
#[derive(Clone)]
pub struct WordleController {
    inner: Rc<RefCell<WordleState>>,
}

struct WordleState {
    target_word: String,
    guesses: Vec<Vec<Option<char>>>,
    evaluations: Vec<[LetterStatus; WORD_LEN]>,
    current_row: usize,
    game_state: GameState,
    keyboard: HashMap<char, LetterStatus>,
    message: Option<String>,
    repo: Rc<dyn mvc::WordRepository>,
    history_repo: Rc<dyn mvc::GameHistoryRepository>,
    history: Option<mvc::GameHistory>,
    history_status: HistoryStatus,
}

impl WordleController {
    #[must_use]
    pub fn new(
        word_repo: impl mvc::WordRepository + 'static,
        history_repo: Rc<dyn mvc::GameHistoryRepository>,
    ) -> Self {
        let (history, history_status) = match history_repo.load() {
            Ok(history) => (Some(history), HistoryStatus::Ready),
            Err(error) => {
                eprintln!("Game history unavailable: {error}");
                (None, HistoryStatus::Unavailable)
            }
        };
        let controller = Self {
            inner: Rc::new(RefCell::new(WordleState {
                target_word: String::new(),
                guesses: vec![vec![None; WORD_LEN]; MAX_GUESSES],
                evaluations: vec![[LetterStatus::Unknown; WORD_LEN]; MAX_GUESSES],
                current_row: 0,
                game_state: GameState::Playing,
                keyboard: HashMap::new(),
                message: None,
                repo: Rc::new(word_repo),
                history_repo,
                history,
                history_status,
            })),
        };
        controller.new_game();
        controller
    }

    pub fn new_game(&self) {
        let mut inner = self.inner.borrow_mut();
        let targets = inner.repo.get_targets(WORD_LEN);
        let word = targets
            .as_slice()
            .choose(&mut rand::rng())
            .cloned()
            .unwrap_or_else(|| "plant".into());
        inner.target_word = word;
        inner.guesses = vec![vec![None; WORD_LEN]; MAX_GUESSES];
        inner.evaluations = vec![[LetterStatus::Unknown; WORD_LEN]; MAX_GUESSES];
        inner.current_row = 0;
        inner.game_state = GameState::Playing;
        inner.keyboard.clear();
        inner.message = None;
    }

    pub fn guess_letter(&self, letter: char) {
        let mut inner = self.inner.borrow_mut();
        if inner.game_state != GameState::Playing || inner.current_row >= MAX_GUESSES {
            return;
        }
        inner.message = None;
        let row = inner.current_row;
        let col = inner.guesses[row].iter().position(Option::is_none);
        if let Some(col) = col {
            inner.guesses[row][col] = Some(letter.to_ascii_lowercase());
        }
    }

    pub fn delete_letter(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.game_state != GameState::Playing {
            return;
        }
        inner.message = None;
        let row = inner.current_row;
        for cell in inner.guesses[row].iter_mut().rev() {
            if cell.is_some() {
                *cell = None;
                break;
            }
        }
    }

    /// Submit the current guess. Returns true if submitted.
    #[allow(
        clippy::too_many_lines,
        clippy::missing_panics_doc,
        clippy::needless_range_loop
    )]
    pub fn submit_guess(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.game_state != GameState::Playing {
            return false;
        }
        let row = inner.current_row;
        if inner.guesses[row].iter().any(Option::is_none) {
            inner.message = Some("Not enough letters".to_string());
            return false;
        }
        // Validate guess is a real word in the word list
        let guess: String = inner.guesses[row].iter().filter_map(|cell| *cell).collect();
        let valid_words = inner.repo.get_words(WORD_LEN);
        if !valid_words.contains(&guess) {
            inner.message = Some("Not in word list".to_string());
            return false;
        }

        // Don't allow guessing the same word twice
        for prev_row in 0..row {
            let prev: String = inner.guesses[prev_row]
                .iter()
                .map(|c| c.unwrap_or_default())
                .collect();
            if prev == guess && prev.len() == WORD_LEN {
                inner.message = Some("Already guessed".to_string());
                return false;
            }
        }

        let guess_chars: Vec<char> = guess.chars().collect();

        // Hard Mode Rule 1: Must use correct letters in exact positions
        for i in 0..WORD_LEN {
            for prev_row in 0..row {
                if inner.evaluations[prev_row][i] == LetterStatus::Correct {
                    let required = inner.guesses[prev_row][i].unwrap();
                    if guess_chars[i] != required {
                        let ord = match i {
                            0 => "1st",
                            1 => "2nd",
                            2 => "3rd",
                            3 => "4th",
                            _ => "5th",
                        };
                        inner.message = Some(format!(
                            "{ord} letter must be {}",
                            required.to_ascii_uppercase()
                        ));
                        return false;
                    }
                }
            }
        }

        // Hard Mode Rule 2: Must contain all present/correct letters
        for prev_row in 0..row {
            let prev_eval = inner.evaluations[prev_row];
            let prev_guess = &inner.guesses[prev_row];
            for (i, &status) in prev_eval.iter().enumerate() {
                if matches!(status, LetterStatus::Correct | LetterStatus::Present) {
                    let req_char = prev_guess[i].unwrap();
                    let count_in_prev = prev_guess
                        .iter()
                        .zip(prev_eval.iter())
                        .filter(|(c, s)| {
                            c.is_some_and(|ch| ch == req_char)
                                && matches!(**s, LetterStatus::Correct | LetterStatus::Present)
                        })
                        .count();
                    let count_in_curr = guess_chars.iter().filter(|&&c| c == req_char).count();
                    if count_in_curr < count_in_prev {
                        inner.message = Some(format!(
                            "Guess must contain {}",
                            req_char.to_ascii_uppercase()
                        ));
                        return false;
                    }
                }
            }
        }

        inner.message = None;

        let target: Vec<char> = inner.target_word.chars().collect();

        let mut result = [LetterStatus::Absent; WORD_LEN];
        let mut target_remaining = target.clone();
        for i in 0..WORD_LEN {
            if guess_chars[i] == target[i] {
                result[i] = LetterStatus::Correct;
                target_remaining[i] = ' ';
            }
        }

        for i in 0..WORD_LEN {
            if result[i] == LetterStatus::Correct {
                continue;
            }
            if let Some(pos) = target_remaining.iter().position(|&c| c == guess_chars[i]) {
                result[i] = LetterStatus::Present;
                target_remaining[pos] = ' ';
            }
        }
        inner.evaluations[row] = result;

        for (i, &letter) in guess_chars.iter().enumerate() {
            let current = inner
                .keyboard
                .get(&letter)
                .copied()
                .unwrap_or(LetterStatus::Unknown);
            let new = result[i];
            let upgrade = !matches!(
                (current, new),
                (LetterStatus::Correct, _) | (LetterStatus::Present, LetterStatus::Absent)
            );
            if upgrade {
                inner.keyboard.insert(letter, new);
            }
        }

        let win = result.iter().all(|&s| s == LetterStatus::Correct);
        let completed_outcome = if win {
            inner.game_state = GameState::Won;
            Some(mvc::GameOutcome::Won)
        } else if inner.current_row >= MAX_GUESSES - 1 {
            inner.game_state = GameState::Lost;
            Some(mvc::GameOutcome::Lost)
        } else {
            inner.current_row += 1;
            None
        };

        if let Some(outcome) = completed_outcome {
            Self::record_completed_game(&mut inner, outcome);
        }

        true
    }

    fn record_completed_game(inner: &mut WordleState, outcome: mvc::GameOutcome) {
        let Some(mut candidate) = inner.history.clone() else {
            return;
        };
        let attempts =
            u8::try_from(inner.current_row + 1).expect("Wordle attempt count always fits in u8");
        if let Err(error) = candidate.append(outcome, attempts) {
            eprintln!("Could not add completed game to history: {error}");
            inner.history_status = HistoryStatus::Unsaved;
            return;
        }

        let save_result = inner.history_repo.save(&candidate);
        inner.history = Some(candidate);
        match save_result {
            Ok(()) => inner.history_status = HistoryStatus::Ready,
            Err(error) => {
                eprintln!("Could not save game history: {error}");
                inner.history_status = HistoryStatus::Unsaved;
            }
        }
    }

    #[must_use]
    pub fn message(&self) -> Option<String> {
        self.inner.borrow().message.clone()
    }

    #[must_use]
    pub fn target_word(&self) -> String {
        self.inner.borrow().target_word.clone()
    }

    #[must_use]
    pub fn current_row(&self) -> usize {
        self.inner.borrow().current_row
    }

    #[must_use]
    pub fn game_state(&self) -> GameState {
        self.inner.borrow().game_state
    }

    #[must_use]
    pub fn stats(&self) -> Option<mvc::GameStats> {
        self.inner
            .borrow()
            .history
            .as_ref()
            .map(mvc::GameHistory::stats)
    }

    #[must_use]
    pub fn history_status(&self) -> HistoryStatus {
        self.inner.borrow().history_status
    }

    #[must_use]
    pub fn keyboard_state(&self) -> HashMap<char, LetterStatus> {
        self.inner.borrow().keyboard.clone()
    }

    #[must_use]
    pub fn guesses(&self) -> Vec<Vec<Option<char>>> {
        self.inner.borrow().guesses.clone()
    }

    #[must_use]
    pub fn cell_status(&self, row: usize, column: usize) -> LetterStatus {
        self.inner
            .borrow()
            .evaluations
            .get(row)
            .and_then(|evaluation| evaluation.get(column))
            .copied()
            .unwrap_or(LetterStatus::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvc::WordRepository;

    const TEST_WORDS: [&str; 11] = [
        "plant", "crane", "brain", "stone", "chair", "ghost", "slate", "cabin", "cacao", "plain",
        "vivid",
    ];
    const NO_CLUE_WORDS: [&str; 6] = ["plant", "crane", "stone", "ghost", "slate", "cacao"];

    struct TestRepo;
    impl WordRepository for TestRepo {
        fn get_targets(&self, _length: usize) -> Vec<String> {
            TEST_WORDS.iter().map(ToString::to_string).collect()
        }
        fn get_words(&self, _length: usize) -> Vec<String> {
            TEST_WORDS.iter().map(ToString::to_string).collect()
        }
        fn unload_words(&self, _length: usize) {}
    }

    fn test_controller() -> WordleController {
        WordleController::new(TestRepo, Rc::new(mvc::InMemoryGameHistoryRepository::new()))
    }

    fn test_controller_with_history() -> (WordleController, Rc<mvc::InMemoryGameHistoryRepository>)
    {
        let history_repo = Rc::new(mvc::InMemoryGameHistoryRepository::new());
        let controller = WordleController::new(TestRepo, history_repo.clone());
        (controller, history_repo)
    }

    fn set_target(controller: &WordleController, target: &str) {
        controller.inner.borrow_mut().target_word = target.to_owned();
    }

    fn enter_guess(controller: &WordleController, guess: &str) {
        for letter in guess.chars() {
            controller.guess_letter(letter);
        }
    }

    fn submit_guess(controller: &WordleController, guess: &str) {
        enter_guess(controller, guess);
        assert!(
            controller.submit_guess(),
            "guess {guess} should be accepted"
        );
    }

    fn lose_game(controller: &WordleController) {
        set_target(controller, "vivid");
        for wrong in NO_CLUE_WORDS {
            submit_guess(controller, wrong);
        }
    }

    #[test]
    fn test_new_game_has_word() {
        let ctrl = test_controller();
        assert!(!ctrl.target_word().is_empty());
        assert_eq!(ctrl.target_word().len(), 5);
        assert_eq!(ctrl.history_status(), HistoryStatus::Ready);
        assert_eq!(ctrl.stats().expect("history should load").played, 0);
    }

    #[test]
    fn test_guess_letter_adds_to_row() {
        let ctrl = test_controller();
        ctrl.guess_letter('a');
        assert_eq!(ctrl.guesses()[0][0], Some('a'));
    }

    #[test]
    fn test_delete_letter_removes_last() {
        let ctrl = test_controller();
        ctrl.guess_letter('a');
        ctrl.guess_letter('b');
        ctrl.delete_letter();
        assert_eq!(ctrl.guesses()[0][0], Some('a'));
        assert!(ctrl.guesses()[0][1].is_none());
    }

    #[test]
    fn test_submit_incomplete_guess_fails() {
        let ctrl = test_controller();
        ctrl.guess_letter('a');
        assert!(!ctrl.submit_guess());
        assert_eq!(ctrl.message(), Some("Not enough letters".to_string()));
    }

    #[test]
    fn test_correct_guess_wins() {
        let ctrl = test_controller();
        let target = ctrl.target_word();
        for ch in target.chars() {
            ctrl.guess_letter(ch);
        }
        assert!(ctrl.submit_guess());
        assert_eq!(ctrl.game_state(), GameState::Won);
    }

    #[test]
    fn duplicate_letters_do_not_exceed_target_letter_count() {
        let ctrl = test_controller();
        ctrl.inner.borrow_mut().target_word = "cabin".to_owned();
        for letter in "cacao".chars() {
            ctrl.guess_letter(letter);
        }
        assert!(ctrl.submit_guess());
        assert_eq!(ctrl.cell_status(0, 0), LetterStatus::Correct);
        assert_eq!(ctrl.cell_status(0, 1), LetterStatus::Correct);
        assert_eq!(ctrl.cell_status(0, 2), LetterStatus::Absent);
        assert_eq!(ctrl.cell_status(0, 3), LetterStatus::Absent);
        assert_eq!(ctrl.cell_status(0, 4), LetterStatus::Absent);
    }

    #[test]
    fn test_six_wrong_guesses_loses() {
        let (ctrl, history_repo) = test_controller_with_history();
        lose_game(&ctrl);
        assert_eq!(ctrl.game_state(), GameState::Lost);

        let stats = ctrl.stats().expect("history should remain available");
        assert_eq!(stats.played, 1);
        assert_eq!(stats.won, 0);
        assert_eq!(stats.lost, 1);
        assert_eq!(stats.current_streak, 0);
        assert_eq!(stats.wins_by_attempt, [0; MAX_GUESSES]);

        let history = history_repo.snapshot();
        assert_eq!(history.records().len(), 1);
        assert_eq!(history.records()[0].outcome, mvc::GameOutcome::Lost);
        assert_eq!(history.records()[0].attempts, 6);
    }

    #[test]
    fn test_invalid_word_message() {
        let ctrl = test_controller();
        for ch in "zzzzz".chars() {
            ctrl.guess_letter(ch);
        }
        assert!(!ctrl.submit_guess());
        assert_eq!(ctrl.message(), Some("Not in word list".to_string()));
    }

    #[test]
    fn test_hard_mode_rule_violations() {
        let ctrl = test_controller();
        ctrl.inner.borrow_mut().target_word = "plant".to_owned();

        // Guess 1: "plain" -> 'p','l','a' correct at 0,1,2; 'n' present at 3
        for ch in "plain".chars() {
            ctrl.guess_letter(ch);
        }
        assert!(ctrl.submit_guess());

        // Guess 2: "crane" -> 1st letter 'c' violates 1st letter 'P' rule
        for ch in "crane".chars() {
            ctrl.guess_letter(ch);
        }
        assert!(!ctrl.submit_guess());
        assert_eq!(ctrl.message(), Some("1st letter must be P".to_string()));
    }

    #[test]
    fn wins_on_every_attempt_record_once_in_the_correct_bucket() {
        for attempts in 1..=MAX_GUESSES {
            let (ctrl, history_repo) = test_controller_with_history();
            set_target(&ctrl, "vivid");
            for wrong in NO_CLUE_WORDS.into_iter().take(attempts - 1) {
                submit_guess(&ctrl, wrong);
            }
            submit_guess(&ctrl, "vivid");
            assert_eq!(ctrl.game_state(), GameState::Won);

            assert!(!ctrl.submit_guess());
            let history = history_repo.snapshot();
            assert_eq!(history.records().len(), 1);
            assert_eq!(history.records()[0].outcome, mvc::GameOutcome::Won);
            assert_eq!(
                usize::from(history.records()[0].attempts),
                attempts,
                "wrong attempt count for a win on guess {attempts}"
            );

            let stats = ctrl.stats().expect("history should remain available");
            assert_eq!(stats.played, 1);
            assert_eq!(stats.won, 1);
            assert_eq!(stats.lost, 0);
            let mut expected_distribution = [0; MAX_GUESSES];
            expected_distribution[attempts - 1] = 1;
            assert_eq!(stats.wins_by_attempt, expected_distribution);
        }
    }

    #[test]
    fn rejected_guesses_do_not_record_games() {
        let (ctrl, history_repo) = test_controller_with_history();
        set_target(&ctrl, "plant");

        ctrl.guess_letter('a');
        assert!(!ctrl.submit_guess());
        for _ in 0..WORD_LEN {
            ctrl.delete_letter();
        }

        enter_guess(&ctrl, "zzzzz");
        assert!(!ctrl.submit_guess());
        for _ in 0..WORD_LEN {
            ctrl.delete_letter();
        }

        submit_guess(&ctrl, "crane");
        enter_guess(&ctrl, "crane");
        assert!(!ctrl.submit_guess());

        assert!(history_repo.snapshot().records().is_empty());
        assert_eq!(
            ctrl.stats()
                .expect("history should remain available")
                .played,
            0
        );
    }

    #[test]
    fn new_game_preserves_history_and_stats() {
        let (ctrl, history_repo) = test_controller_with_history();
        set_target(&ctrl, "plant");
        submit_guess(&ctrl, "plant");
        ctrl.new_game();

        assert_eq!(ctrl.game_state(), GameState::Playing);
        let stats = ctrl.stats().expect("history should remain available");
        assert_eq!(stats.played, 1);
        assert_eq!(stats.won, 1);
        assert_eq!(stats.current_streak, 1);
        assert_eq!(history_repo.snapshot().records().len(), 1);
    }

    #[test]
    fn controller_stats_follow_win_and_loss_streaks() {
        let ctrl = test_controller();
        set_target(&ctrl, "plant");
        submit_guess(&ctrl, "plant");
        ctrl.new_game();
        set_target(&ctrl, "crane");
        submit_guess(&ctrl, "crane");

        let winning_stats = ctrl.stats().expect("history should remain available");
        assert_eq!(winning_stats.current_streak, 2);
        assert_eq!(winning_stats.max_streak, 2);

        ctrl.new_game();
        lose_game(&ctrl);
        let losing_stats = ctrl.stats().expect("history should remain available");
        assert_eq!(losing_stats.played, 3);
        assert_eq!(losing_stats.won, 2);
        assert_eq!(losing_stats.lost, 1);
        assert_eq!(losing_stats.current_streak, 0);
        assert_eq!(losing_stats.max_streak, 2);
    }

    #[test]
    fn failed_save_keeps_pending_stats_and_later_completion_flushes_once() {
        let (ctrl, history_repo) = test_controller_with_history();
        history_repo.set_save_failure("simulated write failure");
        set_target(&ctrl, "plant");
        submit_guess(&ctrl, "plant");

        assert_eq!(ctrl.history_status(), HistoryStatus::Unsaved);
        assert_eq!(
            ctrl.stats()
                .expect("session history should be retained")
                .played,
            1
        );
        assert!(history_repo.snapshot().records().is_empty());

        history_repo.clear_save_failure();
        ctrl.new_game();
        set_target(&ctrl, "crane");
        submit_guess(&ctrl, "crane");

        assert_eq!(ctrl.history_status(), HistoryStatus::Ready);
        let persisted = history_repo.snapshot();
        assert_eq!(persisted.records().len(), 2);
        assert_eq!(persisted.records()[0].game_id, 1);
        assert_eq!(persisted.records()[1].game_id, 2);
        assert_eq!(persisted.stats().played, 2);

        assert!(!ctrl.submit_guess());
        assert_eq!(history_repo.snapshot().records().len(), 2);
    }
}
