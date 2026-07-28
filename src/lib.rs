use slint::{ComponentHandle, Timer, TimerMode};

pub mod callback;
pub mod mvc;
pub mod ui;
mod word_data;

pub use callback::*;

pub fn main() {
    let (main_window, controller) = init();
    if std::env::var("SLINT_MCP_PORT").is_ok() {
        start_demo(&main_window, &controller);
    }
    let _ = main_window.run();
}

fn init() -> (ui::WordleView, mvc::WordleController) {
    let view_handle = ui::WordleView::new().unwrap();

    let wordle_controller =
        mvc::WordleController::new(mvc::WordRepositoryImpl::new());

    ui::wordle_adapter::connect(&view_handle, &wordle_controller);

    (view_handle, wordle_controller)
}

/// Auto-play a demo game. Builds a set of guesses from the real word list
/// to show all game states (absent, present, correct) across several rows.
fn start_demo(view: &ui::WordleView, controller: &mvc::WordleController) {
    use crate::mvc::WordRepository;

    let repo = mvc::WordRepositoryImpl::new();
    let all_words = repo.get_words(5);

    let target = controller.target_word();
    let target_chars: Vec<char> = target.chars().collect();
    let used: std::collections::HashSet<char> = target_chars.iter().copied().collect();

    // Guess 1: all letters NOT in target (all absent)
    let guess1 = all_words
        .iter()
        .find(|w| w.chars().all(|c| !used.contains(&c)))
        .cloned()
        .unwrap_or_else(|| "storm".into());

    // Guess 2: only first letter correct, rest absent
    let guess2 = all_words
        .iter()
        .filter(|w| {
            let wc: Vec<char> = w.chars().collect();
            wc[0] == target_chars[0]
                && wc[1..].iter().all(|c| !used.contains(c) || *c != target_chars[1]
                    && *c != target_chars[2] && *c != target_chars[3] && *c != target_chars[4])
                && w.as_str() != target
        })
        .next()
        .cloned()
        .unwrap_or_else(|| guess1.clone());

    // Guess 3: first two correct, rest wrong (mix of correct + absent)
    let guess3 = all_words
        .iter()
        .filter(|w| {
            let wc: Vec<char> = w.chars().collect();
            wc[0] == target_chars[0]
                && wc[1] == target_chars[1]
                && wc[2] != target_chars[2]
                && wc[3] != target_chars[3]
                && wc[4] != target_chars[4]
                && w.as_str() != target
        })
        .next()
        .cloned()
        .unwrap_or_else(|| guess2.clone());

    // Guess 4: three correct, one present, one wrong
    let guess4 = all_words
        .iter()
        .filter(|w| {
            let wc: Vec<char> = w.chars().collect();
            let mut matches = 0;
            for i in 0..5 {
                if wc[i] == target_chars[i] { matches += 1; }
            }
            matches >= 3 && w.as_str() != target
        })
        .next()
        .cloned()
        .unwrap_or_else(|| guess3.clone());

    // Guess 5: the answer
    let guess5 = target.clone();

    let weak = view.as_weak();
    let timer = Timer::default();
    let state = std::rc::Rc::new(std::cell::RefCell::new(0i32));

    timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        {
            let state = state.clone();
            move || {
                let Some(view) = weak.upgrade() else {
                    return;
                };
                let mut step = state.borrow_mut();

                // Step layout: 5 letters + submit per guess
                // guess1: 0-4 type, 5 submit
                // guess2: 6-10 type, 11 submit
                // guess3: 12-16 type, 17 submit
                // guess4: 18-22 type, 23 submit
                // guess5: 24-28 type, 29 submit
                let (guess_str, next_offset) = match *step {
                    0..=4 => (&guess1, 0),
                    6..=10 => (&guess2, 6),
                    12..=16 => (&guess3, 12),
                    18..=22 => (&guess4, 18),
                    24..=28 => (&guess5, 24),
                    5 | 11 | 17 | 23 | 29 => {
                        view.invoke_submit_guess();
                        *step += 1;
                        return;
                    }
                    _ => {
                        *step += 1;
                        return;
                    }
                };

                if let Some(ch) = guess_str.chars().nth((*step - next_offset) as usize) {
                    view.invoke_guess_letter(slint::SharedString::from(ch.to_string()));
                }

                *step += 1;
            }
        },
    );
    let _leaked = Box::leak(Box::new(timer));
}
