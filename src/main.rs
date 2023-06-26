use std::{
    collections::HashMap,
    error,
    fs::{read_to_string, write},
    io,
    path::Path,
    result::Result,
    time::{SystemTime, SystemTimeError},
};

use itertools::Itertools;
use ordinal::Ordinal;
use pancurses::{
    curs_set, endwin, init_pair, initscr, noecho, start_color, use_default_colors, Input, Window,
    A_DIM,
};
use rand::Rng;
use reqwest::blocking::get;

const MAX_DEBUG_LENGTH: i32 = 20;
const WORD_LENGTH: i32 = 5;
const WIN_TEXT_LINES: i32 = 5;
const MAX_GUESSES: i32 = 6;

fn main() -> Result<(), Box<dyn error::Error>> {
    if !Path::new("words").exists() {
        download_words()?;
    }
    let words = get_words()?;
    let (word, index) = pick(&words);
    let window = create_window();
    let mut guesses_taken = 0;
    let mut guesses = Vec::new();
    let start = SystemTime::now();
    let mut alphabet = HashMap::new();
    alphabet.extend(
        "abcdefghijklmnopqrstuvwxyz"
            .chars()
            .map(|character| (character, 0i16)),
    );

    loop {
        let guess_prompt = format!("guess {}: ", guesses_taken + 1);
        window.attron(A_DIM);
        window.addstr(guess_prompt.clone());
        window.attroff(A_DIM);

        display_alphabet(&window, &alphabet);

        window.refresh();
        let mut guess = String::new();
        let guess_position;
        loop {
            let input = window.getch();
            match input {
                Some(Input::Character('\n'))
                    if guess.len() == 5 && words.contains(&guess) && !guesses.contains(&guess) =>
                {
                    guess_position = {
                        let mut position = window.get_cur_yx();
                        position.1 -= WORD_LENGTH;
                        position
                    };
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
        guesses_taken += 1;
        guesses.push(guess.clone());

        for (position, guess_character, index, word_character) in
            guess.chars().enumerate().map(|(index, character)| {
                let mut position = guess_position;
                position.1 += index as i32;
                let position = position;
                (position, character, index, word.chars().nth(index).unwrap())
            })
        {
            window.mv(position.0, position.1);
            window.color_set({
                let mut color = 3;
                if word.contains(guess_character)
                    && index_by_character(word, index).unwrap() + 1
                        <= word
                            .chars()
                            .filter(|character| character == &guess_character)
                            .count()
                {
                    color = 2;
                }
                if word_character == guess_character {
                    color = 1;
                }
                let color_pair = alphabet.get_mut(&guess_character).unwrap();
                *color_pair = match color {
                    1 => 1,
                    2 if color_pair != &1 => 2,
                    2 => 2,
                    3 if color_pair == &0 => 3,
                    _ => 3,
                };
                color
            });
            window.addch(guess_character);
            window.color_set(0);
        }

        window.addch('\n');
        window.refresh();

        if guess == *word {
            display_alphabet(&window, &alphabet);
            display_win(&window, guesses_taken, start, word, index)?;
            break;
        }

        if guesses_taken == MAX_GUESSES {
            display_lose(&window, start, word, index)?;
            break;
        }
    }
    window.getch();
    endwin();
    Ok(())
}

fn display_lose(
    window: &Window,
    start: SystemTime,
    word: &String,
    index: usize,
) -> Result<(), SystemTimeError> {
    window.mv(window.get_max_y() - WIN_TEXT_LINES, 0);
    window.attron(A_DIM);
    window.addstr("You ran out of guesses\ntrying to guess the word `");
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
    window.addstr("Press any key to exit!");
    window.refresh();
    Ok(())
}

fn display_alphabet(window: &Window, alphabet: &HashMap<char, i16>) {
    let position = window.get_cur_yx();
    for (index, (character, color_pair)) in alphabet
        .iter()
        .map(|(character, color_pair)| (character, color_pair))
        .sorted_unstable()
        .enumerate()
    {
        window.color_set(*color_pair);
        window.mvaddch(MAX_GUESSES + 2, index as i32, *character);
        window.color_set(0);
    }
    window.mv(position.0, position.1);
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
) -> Result<(), SystemTimeError> {
    window.mv(window.get_max_y() - WIN_TEXT_LINES, 0);
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
    window.addstr("Press any key to exit!");
    window.refresh();
    Ok(())
}

fn backspace(window: &Window) {
    window.mv(window.get_cur_y(), window.get_cur_x() - 1);
    window.delch();
}

fn get_words() -> io::Result<Vec<String>> {
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

    start_color();
    use_default_colors();

    init_pair(1, -1, 2);
    init_pair(2, -1, 3);
    init_pair(3, -1, 0);

    curs_set(1);
    window
}

fn download_words() -> Result<(), Box<dyn error::Error>> {
    let words = get("https://raw.githubusercontent.com/dwyl/english-words/master/words_alpha.txt")?
        .text()?;
    let words = words
        .split_ascii_whitespace()
        .filter(|word| word.len() == WORD_LENGTH as usize)
        .collect::<Vec<&str>>()
        .join("\n");
    write("words", words)?;
    Ok(())
}

fn index_by_character(text: &String, index: usize) -> Option<usize> {
    if index >= text.len() {
        return None;
    }
    text.char_indices()
        .filter(|(_, character)| character == &text.chars().nth(index).unwrap())
        .enumerate()
        .filter_map(|(character_index, (word_index, _))| {
            if word_index == index {
                Some(character_index)
            } else {
                None
            }
        })
        .at_most_one()
        .unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use crate::index_by_character;

    #[test]
    fn test_index_by_character() {
        assert_eq!(index_by_character(&"abcda".to_string(), 4), Some(1));
        assert_eq!(index_by_character(&"ww".to_string(), 0), Some(0));
        assert_eq!(index_by_character(&"djka".to_string(), 5), None);
        assert_eq!(index_by_character(&"aoidwa".to_string(), 5), Some(1));
        assert_eq!(index_by_character(&"bbbbb".to_string(), 3), Some(3));
        assert_eq!(index_by_character(&"".to_string(), 0), None);
    }
}
