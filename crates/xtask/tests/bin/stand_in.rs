//! A program that stands in for a program of the network in the tests of the tasks, as curl or gh:
//! a test copies it under that name into a folder at the head of PATH, with a map beside it, the
//! file of its name with the extension `json`, from each word that a task gives to the file that
//! answers it. The program prints the file of the first word of its command that the map holds, and
//! fails when none does, so a task that asks for more than the test serves fails loudly.

use std::{
  collections::HashMap,
  env, fs,
  io::{self, Write as _},
  process::ExitCode,
};

fn main() -> ExitCode {
  let map = env::current_exe().map(|exe| exe.with_extension("json"));
  let text = map.and_then(fs::read_to_string).unwrap_or_default();
  let answers: HashMap<String, String> = serde_json::from_str(&text).unwrap_or_default();
  let words: Vec<String> = env::args().skip(1).collect();
  let Some(file) = words.iter().find_map(|word| answers.get(word)) else {
    eprintln!("no file answers {words:?}");
    return ExitCode::FAILURE;
  };
  match fs::read(file).and_then(|bytes| io::stdout().write_all(&bytes)) {
    Ok(()) => ExitCode::SUCCESS,
    Err(no) => {
      eprintln!("{file} cannot be read: {no}");
      ExitCode::FAILURE
    }
  }
}
