use std::{error, fmt::Display, fs::read_to_string, io::Result, result};

use pancurses::{initscr, noecho, Input, Window};
use rand::Rng;

fn main() -> result::Result<(), Box<dyn error::Error>> {
    let words = get_words()?;
    let word = pick(&words);
    let window = create_window();
    let mut guesses_done = 1;
    let mut guesses = Vec::new();
    loop {
        window.addstr(format!("{guesses_done}: "));
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
                Some(Input::Character('\x7f')) if window.get_cur_x() > 3 => {
                    backspace(&window);
                    guess.pop();
                }
                Some(Input::Character(character))
                    if character.is_ascii_alphabetic() && window.get_cur_x() < 8 =>
                {
                    window.addch(character);
                    guess.push(character);
                }
                _ => {}
            }
            if let Some(Input::Character(character)) = input {
                let escaped = &character.escape_debug();
                let (y, x) = window.get_cur_yx();
                window.mvaddstr(
                    window.get_max_y() - 1,
                    window.get_max_x() - 20,
                    "                    ",
                );
                window.mvaddstr(
                    window.get_max_y() - 1,
                    window.get_max_x() - escaped.len() as i32 - 1,
                    escaped.to_string(),
                );
                window.mv(y, x);
            }
            window.refresh();
        }
        window.addch('\n');
        window.refresh();
        guesses_done += 1;
        guesses.push(guess.clone());

        if guess == *word {
            window.addstr("You win!\n");
            window.refresh();
            break;
        }
    }

    Ok(())
}

fn backspace(window: &Window) {
    window.mv(window.get_cur_y(), window.get_cur_x() - 1);
    window.delch();
}

#[derive(Debug)]
struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.0
    }
}

fn get_words() -> Result<Vec<String>> {
    Ok(read_to_string("words")?
        .split_ascii_whitespace()
        .filter_map(|word| {
            let word = word.to_lowercase().trim().to_string();
            if word.len() == 5 {
                Some(word)
            } else {
                None
            }
        })
        .collect())
}

fn pick<T>(list: &Vec<T>) -> &T {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..list.len());
    &list[index]
}

fn create_window() -> Window {
    let window = initscr();
    window.keypad(true);
    noecho();
    window
}
