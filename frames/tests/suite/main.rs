//! The program `furb-frames` as a person and a model run it: on recordings and frames that each
//! test writes in a yard of its own, through its commands and the files that they write.

#[path = "../../../tests/common/yard.rs"]
mod yard;

mod compare;
mod image;
mod motion;
mod probe;
mod replay;

use std::{
  fs,
  path::Path,
  process::{self, Command, Output},
};

use serde_json::{Value, json};

pub(crate) use yard::Yard;

/// The yard of a test.
pub(crate) fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-frames-{name}-{}", process::id()))
}

/// What the program says on its words, in a folder.
pub(crate) fn frames(words: &[&str], at: &Path) -> Output {
  let mut command = Command::new(env!("CARGO_BIN_EXE_furb-frames"));
  command.args(words).current_dir(at);
  command.output().unwrap_or_else(|no| panic!("furb-frames did not start: {no}"))
}

/// What the program wrote on its stdout.
pub(crate) fn said(done: &Output) -> String {
  assert!(done.status.success(), "{}", String::from_utf8_lossy(&done.stderr));
  String::from_utf8_lossy(&done.stdout).into_owned()
}

/// A recording of asciinema of a size, written at a path: each event a delay in seconds after the
/// one before, a code and its data, as format 3 writes them.
pub(crate) fn cast(path: &Path, (cols, rows): (u16, u16), events: &[(f64, &str, &str)]) {
  let head = json!({ "version": 3, "term": { "cols": cols, "rows": rows } });
  let lines = events.iter().map(|(delay, code, data)| json!([delay, code, data]).to_string());
  let text = [head.to_string()].into_iter().chain(lines).collect::<Vec<_>>().join("\n") + "\n";
  put(path, &text);
}

/// A file of a test at its path, with its folders.
pub(crate) fn put(path: &Path, text: &str) {
  let made = path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(path, text));
  made.unwrap_or_else(|no| panic!("{} was not written: {no}", path.display()));
}

/// The text of a file that a command wrote.
pub(crate) fn read(path: &Path) -> String {
  fs::read_to_string(path).unwrap_or_else(|no| panic!("{} was not written: {no}", path.display()))
}

/// The JSON of a file that a command wrote.
pub(crate) fn json_of(path: &Path) -> Value {
  serde_json::from_str(&read(path))
    .unwrap_or_else(|no| panic!("{} is no JSON: {no}", path.display()))
}
