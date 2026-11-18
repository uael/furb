//! The kinds of what an answer read are private to the client, and no door gives them but the box
//! of the context of a window, which reads them from a machine.

use serde_json::json;

use super::{Content, kinds};

#[test]
fn the_engine_counts_the_system_prompt_at_the_tokens_of_a_character_that_the_first_answer_read() {
  // The first answer read the system prompt and the setup, 1000 characters for 1000 tokens, and
  // the last answer read 500 tokens more, which the turns take, with what the model read again.
  let turns = [
    json!(["user", "x".repeat(700), null, null]),
    json!(["assistant", "", [1000, 0, 0, 0, 0.0], null]),
    json!(["user", "y".repeat(100), null, null]),
    json!(["assistant", "", [1500, 0, 0, 0, 0.0], null]),
  ];
  assert_eq!(kinds(&turns, &"s".repeat(300)), vec![(Content::Engine, 300), (Content::Turns, 1200)]);
}

#[test]
fn a_local_memory_file_counts_as_memory() {
  // The answer read a paragraph of the local memory file of a folder and a paragraph of a file,
  // one token for each character.
  let local = "#read1\nread1_path = '~/w/AGENTS.local.md'\nread1_text = 'x'\n\n";
  let told = format!("{local}#read2\nread2_path = '~/w/a.txt'\nread2_text = 'y'");
  let (local, all) = (u64::try_from(local.len()).unwrap(), u64::try_from(told.len()).unwrap());
  #[expect(clippy::cast_precision_loss, reason = "the turn holds far fewer characters than 2^52")]
  let turns =
    [json!(["user", told, null, null]), json!(["assistant", "", [all as f64, 0, 0, 0, 0.0], null])];
  assert_eq!(kinds(&turns, ""), vec![(Content::Memory, local), (Content::Turns, all - local)]);
}
