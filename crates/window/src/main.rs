//! `furb-window`: the window of furb on this terminal, which `furb` with no command starts.

use std::{env, process::ExitCode};

use furb_window::{Error, run};

fn main() -> ExitCode {
  // The window starts a session only from its input, so it takes no word but the one that makes
  // it only read, and ignores none.
  let mut words = env::args().skip(1);
  let done = match (words.next(), words.next()) {
    (None, _) => run(false),
    (Some(word), None) if word == "--read" => run(true),
    (Some(word), _) => Err(Error::Word(word)),
  };
  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("furb-window: {fault}");
      ExitCode::FAILURE
    }
  }
}
