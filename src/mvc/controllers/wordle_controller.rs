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
    repo: Rc<dyn mvc::WordRepository>,
}

impl WordleController {
    #[must_use]
    pub fn new(repo: impl mvc::WordRepository + 'static) -> Self {
        let controller = Self {
            inner: Rc::new(RefCell::new(WordleState {
                target_word: String::new(),
                guesses: vec![vec![None; WORD_LEN]; MAX_GUESSES],
                evaluations: vec![[LetterStatus::Unknown; WORD_LEN]; MAX_GUESSES],
                current_row: 0,
                game_state: GameState::Playing,
                keyboard: HashMap::new(),
                repo: Rc::new(repo),
            })),
        };
        controller.new_game();
        controller
    }

    pub fn new_game(&self) {
        let mut inner = self.inner.borrow_mut();
        let words = inner.repo.get_words(WORD_LEN);
        let word = words
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
    }

    pub fn guess_letter(&self, letter: char) {
        let mut inner = self.inner.borrow_mut();
        if inner.game_state != GameState::Playing || inner.current_row >= MAX_GUESSES {
            return;
        }
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
        let row = inner.current_row;
        for cell in inner.guesses[row].iter_mut().rev() {
            if cell.is_some() {
                *cell = None;
                break;
            }
        }
    }

    /// Submit the current guess. Returns true if submitted.
    pub fn submit_guess(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if inner.game_state != GameState::Playing {
            return false;
        }
        let row = inner.current_row;
        if inner.guesses[row].iter().any(Option::is_none) {
            return false;
        }
        // Validate guess is a real word in the word list
        let guess: String = inner.guesses[row].iter().filter_map(|cell| *cell).collect();
        let valid_words = inner.repo.get_words(WORD_LEN);
        if !valid_words.contains(&guess) {
            return false;
        }

        // Don't allow guessing the same word twice
        for prev_row in 0..row {
            let prev: String = inner.guesses[prev_row]
                .iter()
                .map(|c| c.unwrap_or_default())
                .collect();
            if prev == guess && prev.len() == WORD_LEN {
                return false;
            }
        }

        let guess_chars: Vec<char> = guess.chars().collect();
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
        if win {
            inner.game_state = GameState::Won;
        } else if inner.current_row >= MAX_GUESSES - 1 {
            inner.game_state = GameState::Lost;
        } else {
            inner.current_row += 1;
        }

        true
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

    const TEST_WORDS: [&str; 9] = [
        "plant", "crane", "brain", "stone", "chair", "ghost", "slate", "cabin", "cacao",
    ];

    struct TestRepo;
    impl WordRepository for TestRepo {
        fn get_words(&self, _length: usize) -> Vec<String> {
            TEST_WORDS.iter().map(ToString::to_string).collect()
        }
        fn unload_words(&self, _length: usize) {}
    }

    fn test_controller() -> WordleController {
        WordleController::new(TestRepo)
    }

    #[test]
    fn test_new_game_has_word() {
        let ctrl = test_controller();
        assert!(!ctrl.target_word().is_empty());
        assert_eq!(ctrl.target_word().len(), 5);
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
        let ctrl = test_controller();
        let target = ctrl.target_word();
        let wrong_words: Vec<_> = TEST_WORDS
            .into_iter()
            .filter(|word| *word != target)
            .take(MAX_GUESSES)
            .collect();
        assert_eq!(wrong_words.len(), MAX_GUESSES);

        for wrong in wrong_words {
            for ch in wrong.chars() {
                ctrl.guess_letter(ch);
            }
            assert!(ctrl.submit_guess());
        }
        assert_eq!(ctrl.game_state(), GameState::Lost);
    }
}
