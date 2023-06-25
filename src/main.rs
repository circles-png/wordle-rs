use std::{
    error,
    fs::read_to_string,
    io::Result,
    result,
    time::{SystemTime, SystemTimeError},
};

use ordinal::Ordinal;
use pancurses::{initscr, noecho, Input, Window, endwin, A_DIM};
use rand::Rng;

const MAX_DEBUG_LENGTH: i32 = 20;
const WORD_LENGTH: i32 = 5;
const WIN_TEXT_LINES: i32 = 5;

fn main() -> result::Result<(), Box<dyn error::Error>> {
    let words = get_words()?;
    let (word, index) = pick(&words);
    let window = create_window();
    let mut guesses_taken = 0;
    let mut guesses = Vec::new();
    let start = SystemTime::now();
    loop {
        let guess_prompt = format!("guess {}: ", guesses_taken + 1);
        window.attron(A_DIM);
        window.addstr(guess_prompt.clone());
        window.attroff(A_DIM);
        window.refresh();
        let mut guess = String::new();
        loop {
            let input = window.getch();
            match input {
                Some(Input::Character('\n'))
                    if guess.len() == 5 && words.contains(&guess) && !guesses.contains(&guess) =>
                {
                    break;
                }
                Some(Input::Character('\x7f'))
                    if window.get_cur_x() > guess_prompt.len() as i32 =>
                {
                    backspace(&window);
                    guess.pop();
                }
                Some(Input::Character(character))
                    if character.is_ascii_alphabetic()
                        && window.get_cur_x() < (guess_prompt.len() as i32 + WORD_LENGTH) =>
                {
                    window.addch(character);
                    guess.push(character);
                }
                _ => {}
            }
            if let Some(Input::Character(character)) = input {
                display_debug(character, &window);
            }
            window.refresh();
        }
        window.addch('\n');
        window.refresh();
        guesses_taken += 1;
        guesses.push(guess.clone());
        if guess == *word {
            display_win(&window, guesses_taken, start, word, index)?;
            break;
        }
    }
    window.getch();
    endwin();
    Ok(())
}

fn display_debug(character: char, window: &Window) {
    let escaped = &character.escape_debug();
    let (y, x) = window.get_cur_yx();
    window.mvaddstr(
        window.get_max_y() - 1,
        window.get_max_x() - MAX_DEBUG_LENGTH,
        "                    ",
    );
    window.mvaddstr(
        window.get_max_y() - 1,
        window.get_max_x() - escaped.len() as i32 - 1,
        escaped.to_string(),
    );
    window.mv(y, x);
}

fn display_win(
    window: &Window,
    guesses_taken: i32,
    start: SystemTime,
    word: &String,
    index: usize,
) -> result::Result<(), SystemTimeError> {
    window.mv(
        window.get_max_y() - WIN_TEXT_LINES,
        0,
    );

    window.attron(A_DIM);
    window.addstr("You took ");
    window.attroff(A_DIM);
    window.addstr(&guesses_taken.to_string());
    window.attron(A_DIM);
    window.addstr(" guess");
    window.addstr(if guesses_taken == 1 { "" } else { "es" });
    window.addstr("\nto guess the word `");
    window.attroff(A_DIM);
    window.addstr(word);
    window.attron(A_DIM);
    window.addstr("` (the ");
    window.attroff(A_DIM);
    window.addstr(Ordinal(index + 1).to_string());
    window.attron(A_DIM);
    window.addstr(" word in the word list)\nin ~");
    window.attroff(A_DIM);
    window.addstr(&format!("{:.2}", start.elapsed()?.as_secs_f64()));
    window.attron(A_DIM);
    window.addstr(" seconds!\n\n");
    window.attroff(A_DIM);
    window.addstr("Press any key to exit.");
    window.refresh();
    Ok(())
}

fn backspace(window: &Window) {
    window.mv(window.get_cur_y(), window.get_cur_x() - 1);
    window.delch();
}

fn get_words() -> Result<Vec<String>> {
    Ok(read_to_string("words")?
        .split_ascii_whitespace()
        .filter_map(|word| {
            let word = word.to_lowercase().trim().to_string();
            if word.len() == WORD_LENGTH as usize {
                Some(word)
            } else {
                None
            }
        })
        .collect())
}

fn pick<T>(list: &Vec<T>) -> (&T, usize) {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..list.len());
    (&list[index], index)
}

fn create_window() -> Window {
    let window = initscr();
    window.keypad(true);
    noecho();
    window
}
