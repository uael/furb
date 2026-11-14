//! `furb-window`: the window of furb on this terminal. `FURB_TUI=furb-window furb` starts it in
//! the place of the window of before.

use std::{env, process::ExitCode};

use furb_window::{Error, run};

fn main() -> ExitCode {
  // furb gives the window of before a record, a folder and a model as words. This window opens no
  // session yet, so it refuses each word and ignores none.
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
