use slint::{ModelRc, SharedString, VecModel};

use crate::mvc::{GameState, LetterStatus, WordleController};
use crate::ui;

/// Connect Slint UI callbacks to the controller.
pub fn connect(view_handle: &ui::WordleView, controller: &WordleController) {
    use slint::ComponentHandle;

    // Guess letter
    let view_weak = view_handle.as_weak();
    let ctrl = controller.clone();
    view_handle.on_guess_letter(move |letter| {
        if let Some(ch) = letter.as_str().chars().next() {
            ctrl.guess_letter(ch);
            if let Some(view) = view_weak.upgrade() {
                refresh_all(&view, &ctrl);
            }
        }
    });

    // Delete letter
    let view_weak = view_handle.as_weak();
    let ctrl = controller.clone();
    view_handle.on_delete_letter(move || {
        ctrl.delete_letter();
        if let Some(view) = view_weak.upgrade() {
            refresh_all(&view, &ctrl);
        }
    });

    // Submit guess
    let view_weak = view_handle.as_weak();
    let ctrl = controller.clone();
    view_handle.on_submit_guess(move || {
        ctrl.submit_guess();
        if let Some(view) = view_weak.upgrade() {
            refresh_all(&view, &ctrl);
        }
    });

    // New game
    let view_weak = view_handle.as_weak();
    let ctrl = controller.clone();
    view_handle.on_new_game(move || {
        ctrl.new_game();
        if let Some(view) = view_weak.upgrade() {
            refresh_all(&view, &ctrl);
        }
    });

    // Quit
    view_handle.on_quit(move || {
        std::process::exit(0);
    });

    // Initial render
    refresh_all(view_handle, controller);
}

fn refresh_all(view_handle: &ui::WordleView, controller: &WordleController) {
    set_cells(view_handle, controller);
    set_keys(view_handle, controller);
    set_game_state(view_handle, controller);
}

fn set_cells(view: &ui::WordleView, controller: &WordleController) {
    let guesses = controller.guesses();
    let current_row = controller.current_row();
    let game_state = controller.game_state();

    let mut cells: Vec<ui::GuessCell> = Vec::with_capacity(30);
    for row in 0..6 {
        for col in 0..5 {
            let letter = guesses[row][col]
                .map(|c| SharedString::from(c.to_uppercase().to_string()))
                .unwrap_or_default();

            let status = if row < current_row || (game_state != GameState::Playing && row <= current_row)
            {
                eval_cell_status(row, col, controller)
            } else {
                0
            };
            cells.push(ui::GuessCell { letter, status });
        }
    }
    view.set_cells(ModelRc::new(VecModel::from(cells)).into());
}

fn eval_cell_status(row: usize, col: usize, controller: &WordleController) -> i32 {
    let guesses = controller.guesses();
    let target = controller.target_word();
    if row >= guesses.len() || col >= guesses[row].len() {
        return 0;
    }
    let Some(ch) = guesses[row][col] else {
        return 0;
    };
    let target_chars: Vec<char> = target.chars().collect();
    if col < target_chars.len() && ch == target_chars[col] {
        return 3;
    }
    if target_chars.contains(&ch) {
        return 2;
    }
    1
}

fn set_keys(view: &ui::WordleView, controller: &WordleController) {
    let kb = controller.keyboard_state();
    let qwerty = [
        'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',
        'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L',
        'Z', 'X', 'C', 'V', 'B', 'N', 'M',
    ];

    let keys: Vec<ui::KeyState> = qwerty
        .iter()
        .map(|&ch| {
            let status = kb
                .get(&ch.to_ascii_lowercase())
                .map(|s| match s {
                    LetterStatus::Absent => 1,
                    LetterStatus::Present => 2,
                    LetterStatus::Correct => 3,
                    LetterStatus::Unknown => 0,
                })
                .unwrap_or(0);
            ui::KeyState {
                letter: SharedString::from(ch.to_string()),
                status,
            }
        })
        .collect();

    view.set_keys(ModelRc::new(VecModel::from(keys)).into());
}

fn set_game_state(view: &ui::WordleView, controller: &WordleController) {
    let state = controller.game_state();
    let current_row = controller.current_row();

    // Show completed rows + current active row (capped at 6)
    let visible = i32::try_from(current_row + 1).unwrap_or(1).min(6);
    view.set_visible_rows(visible);

    let msg = match state {
        GameState::Won => {
            let tries = current_row + 1;
            SharedString::from(format!("You won! ({tries}/6)"))
        }
        GameState::Lost => {
            let answer = controller.target_word();
            SharedString::from(format!("Game over! Answer: {}", answer.to_uppercase()))
        }
        GameState::Playing => SharedString::default(),
    };
    view.set_game_over(state != GameState::Playing);
    view.set_message(msg);
    view.set_answer(SharedString::from(controller.target_word().to_uppercase()));
}
