//! `furb-window`: the window of furb on this terminal, which `furb` with no command starts.

use std::{env, process::ExitCode};

use furb_window::{Error, run};

fn main() -> ExitCode {
  // The window starts a session only from its input, so it refuses each word and ignores none.
  let done = match env::args().nth(1) {
    Some(word) => Err(Error::Word(word)),
    None => run(),
  };
  match done {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("furb-window: {fault}");
      ExitCode::FAILURE
    }
  }
}
