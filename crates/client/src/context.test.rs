use serde_json::{Value, json};

use super::{Content, kinds};

fn turn(role: &str, python: &str, read: Option<f64>) -> Value {
  json!([role, python, read.map(|tokens| [tokens, 1.0, 0.0, 0.0, 0.1]), []])
}

#[test]
fn the_kinds_of_an_answer_sum_to_what_it_read_and_the_engine_takes_what_no_turn_holds() {
  let doctrine = format!("#read1\nread1_path = '/x/DOCTRINE.md'\n{}", "d".repeat(400));
  let kit = format!("NAMES = 1\n{}", "e".repeat(200));
  // A heading of markdown in a memory file starts no paragraph of its own.
  let told = format!(
    "#chain1\nchain1_label = 'root'\n\n#read2\nread2_path = '/p/AGENTS.md'\n# Title\n\n# Heading\n{}",
    "m".repeat(100)
  );
  // The turns cost a quarter of a token for each character, over an engine of 1000 tokens.
  let turns = [
    turn("user", "chain()", None),
    turn("assistant", "read('/x/DOCTRINE.md')", None),
    turn("user", &doctrine, None),
    turn("assistant", "rung(read('/x/memory.py', HIDDEN).content)", None),
    turn("user", &kit, None),
    turn("user", &told, None),
    turn("assistant", "close('a')", Some(1225.75)),
    turn("user", "#prompt2\nprompt2_message = 'more'", None),
    turn("assistant", "close('b')", Some(1236.5)),
  ];
  let parted = [
    (Content::Engine, 1000),
    (Content::Doctrine, 109),
    (Content::Kits, 53),
    (Content::Memory, 39),
    (Content::Turns, 36),
  ];
  assert_eq!(kinds(&turns), parted.to_vec());
}
