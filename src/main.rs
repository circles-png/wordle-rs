use std::{
    error,
    fmt::Display,
    fs::read_to_string,
    io::{stdin, stdout, Result, Write},
    result,
};

use rand::Rng;

fn main() -> result::Result<(), Box<dyn error::Error>> {
    let words = get_words()?;
    let word = pick(&words);
    println!("the word is `{}`", word);
    let mut guesses_done = 1;
    let guess = guess(&format!("{guesses_done}: "))?.trim();

    Ok(())
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

fn pick<'a, T>(list: &'a Vec<T>) -> &'a T {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..list.len());
    &list[index]
}

fn guess(prompt: &String) -> Result<String> {
    print!("{}", prompt);
    stdout().flush()?;
    let mut guess = String::new();
    stdin().read_line(&mut guess)?;
    Ok(guess)
}
