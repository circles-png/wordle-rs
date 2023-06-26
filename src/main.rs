use std::{
    char,
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
        let guess_prompt = format!("{} ", guesses_taken + 1);
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
                Some(Input::Character('\n')) if guess.len() == 5 /* && words.contains(&guess) */ && !guesses.contains(&guess) =>
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
                Some(Input::Character('\x7f')) => {}
                Some(Input::Character(character)) => {
                    if character.is_ascii_alphabetic()
                        && window.get_cur_x() < (guess_prompt.len() as i32 + WORD_LENGTH)
                    {
                        window.addch(character);
                        guess.push(character);
                    }
                }
                _ => (),
            }
            if let Some(Input::Character(character)) = input {
                display_debug(character, &window);
            }
            window.refresh();
        }
        guesses_taken += 1;
        guesses.push(guess.clone());

        let mut word_characters_done = HashMap::new();
        Extend::<(char, bool)>::extend(
            &mut word_characters_done,
            word.chars()
                .map(|character| (character, false))
                .collect::<Vec<(char, bool)>>(),
        );
        let data: Vec<((i32, i32), char, char)> = guess
            .chars()
            .enumerate()
            .map(|(index, character)| {
                let mut position = guess_position;
                position.1 += index as i32;
                let position = position;
                (position, character, word.chars().nth(index).unwrap())
            })
            .collect();
        for (position, guess_character, word_character) in data.iter() {
            window.mv(position.0, position.1);
            window.color_set(if guess_character == word_character {
                word_characters_done.insert(*guess_character, true);
                1
            } else {
                0
            });
            window.addch(*guess_character);
            window.refresh();
            window.color_set(0);
        }
        'yellow_or_gray: for (position, guess_character, word_character) in data.iter() {
            window.mv(position.0, position.1);
            window.color_set('calculate_color_pair: {
                if *word_characters_done.get(word_character).unwrap() {
                    continue 'yellow_or_gray;
                }
                if !word.contains(*guess_character) {
                    break 'calculate_color_pair 1;
                }

                let index_of_character = word.char_indices().find_map(|(index, character)| {
                    if character == *guess_character
                        && !word_characters_done.get(&character).unwrap()
                    {
                        Some(index)
                    } else {
                        None
                    }
                });

                match index_of_character {
                    Some(index) => {
                        word_characters_done.insert(word.chars().nth(index).unwrap(), true);
                        2
                    }
                    None => 3,
                }
            });
            window.addch(*guess_character);
            window.refresh();
            window.color_set(0);
        }

        window.mv(window.get_cur_y() + 1, 0);
        window.refresh();

        if guess == *word {
            display_alphabet(&window, &alphabet);
            display_win(&window, guesses_taken, start, word, index, words.len())?;
            break;
        }

        if guesses_taken == MAX_GUESSES {
            display_lose(&window, start, word, index, words.len())?;
            break;
        }
    }
    window.getch();
    endwin();
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
        " ".repeat(MAX_DEBUG_LENGTH as usize),
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
    words: usize,
) -> Result<(), SystemTimeError> {
    window.mv(window.get_max_y() - WIN_TEXT_LINES, 0);
    window.attron(A_DIM);
    window.addstr("You took ");
    window.attroff(A_DIM);
    window.addstr(guesses_taken.to_string());
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
    window.addstr(" word in the word list of ");
    window.attroff(A_DIM);
    window.addstr(words.to_string());
    window.attron(A_DIM);
    window.addstr(" words)\nin ~");
    window.attroff(A_DIM);
    window.addstr(format!("{:.2}", start.elapsed()?.as_secs_f64()));
    window.attron(A_DIM);
    window.addstr(" seconds!\n\n");
    window.attroff(A_DIM);
    window.addstr("Press any key to exit!");
    window.refresh();
    Ok(())
}

fn display_lose(
    window: &Window,
    start: SystemTime,
    word: &String,
    index: usize,
    words: usize,
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
    window.addstr(" word in the word list of ");
    window.attroff(A_DIM);
    window.addstr(words.to_string());
    window.attron(A_DIM);
    window.addstr(" words)\nin ~");
    window.attroff(A_DIM);
    window.addstr(format!("{:.2}", start.elapsed()?.as_secs_f64()));
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
