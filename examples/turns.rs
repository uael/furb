//! Every turn of a file, as the crate reads it to a model, one to a line.
//!
//! `script/turns.py` writes the turns that crossed the boundary in one run of the suite, one plain turn to a
//! line, and holds what this prints against what the python World makes of the same turns.
//!
//!     cargo run --quiet --example turns target/turns.jsonl

use std::{
  fs,
  io::{self, BufWriter, Write},
};

use furb::{Turn, Value};

fn main() {
  let at = std::env::args().nth(1).expect("the file of turns to read");
  let held = fs::read_to_string(&at).expect("the file of turns");
  let mut out = BufWriter::new(io::stdout().lock());
  for line in held.lines().filter(|one| !one.trim().is_empty()) {
    let got: serde_json::Value = serde_json::from_str(line).expect("one turn, plain");
    let said = Turn::of(&Value::of_record(&got)).map(|one| one.rendered()).unwrap_or_default();
    writeln!(out, "{}", serde_json::to_string(&said).expect("the reading of one turn")).expect("the reading");
  }
}
