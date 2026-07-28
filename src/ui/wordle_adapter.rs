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
    view_handle.on_quit(|| {
        if let Err(error) = slint::quit_event_loop() {
            eprintln!("failed to stop the Slint event loop: {error}");
        }
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

    let mut cells: Vec<ui::GuessCell> = Vec::with_capacity(30);
    for (row, guess) in guesses.iter().enumerate().take(6) {
        for (col, cell) in guess.iter().enumerate().take(5) {
            let letter = cell
                .map(|c| SharedString::from(c.to_uppercase().to_string()))
                .unwrap_or_default();

            let status = letter_status_code(controller.cell_status(row, col));
            cells.push(ui::GuessCell { letter, status });
        }
    }
    view.set_cells(ModelRc::new(VecModel::from(cells)));
}

const fn letter_status_code(status: LetterStatus) -> i32 {
    match status {
        LetterStatus::Unknown => 0,
        LetterStatus::Absent => 1,
        LetterStatus::Present => 2,
        LetterStatus::Correct => 3,
    }
}

fn set_keys(view: &ui::WordleView, controller: &WordleController) {
    let kb = controller.keyboard_state();
    let qwerty = [
        'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', 'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K',
        'L', 'Z', 'X', 'C', 'V', 'B', 'N', 'M',
    ];

    let keys: Vec<ui::KeyState> = qwerty
        .iter()
        .map(|&ch| {
            let status = kb
                .get(&ch.to_ascii_lowercase())
                .map_or(0, |status| letter_status_code(*status));
            ui::KeyState {
                letter: SharedString::from(ch.to_string()),
                status,
            }
        })
        .collect();

    view.set_keys(ModelRc::new(VecModel::from(keys)));
}

fn set_game_state(view: &ui::WordleView, controller: &WordleController) {
    let state = controller.game_state();
    let current_row = controller.current_row();

    view.set_visible_rows(6);

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
