use serde_json::{Value, json};

use super::{Life, State};

/// A small life: the operator asks the root, whose model reads a file and asks the operator a question.
fn facts() -> Vec<Value> {
  vec![
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "Do it", ""]),
    json!(["started", "prompt1", "prompt1"]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", "claude-cli:opus/high"]),
    json!([
      "ready",
      "rung1",
      "rung1",
      "x = read('a')\nq = await prompt(str, 'Which?', to=OPERATOR)"
    ]),
    json!(["read", "read1", "rung1", "chain1", "a"]),
    json!(["done", "read1", "files", {"is": "Text", "path": "a", "content": "1\n2"}]),
    json!(["prompt", "prompt2", "rung1", "chain1", "str", "Which?", "operator"]),
    json!(["started", "prompt2", "console"]),
  ]
}

#[test]
fn a_prompt_that_the_console_took_waits_for_the_operator() {
  let life = Life::of(&facts(), 9);
  assert_eq!(life.debts(), vec!["prompt2".to_owned()]);
  assert_eq!(life.prompt_state("prompt2"), State::Asks);
}

#[test]
fn a_prompt_that_a_rung_made_is_a_thread_under_the_thread_of_that_rung() {
  let life = Life::of(&facts(), 9);
  let threads: Vec<(String, usize)> =
    life.threads().into_iter().map(|one| (one.prompt, one.depth)).collect();
  assert_eq!(threads, vec![("prompt1".to_owned(), 0), ("prompt2".to_owned(), 1)]);
}

#[test]
fn the_life_at_a_cursor_holds_only_the_facts_before_it() {
  let life = Life::of(&facts(), 6);
  assert!(life.act("read1").is_some_and(|act| !act.over()));
  assert!(life.act("prompt2").is_none());
  assert_eq!(life.at, 6);
}

#[test]
fn a_rung_takes_the_word_that_it_said_ready() {
  let life = Life::of(&facts(), 9);
  let rung = life.rung("rung1").expect("the rung stands");
  assert!(rung.word.starts_with("x = read('a')"));
  assert_eq!(rung.acts, vec!["read1".to_owned(), "prompt2".to_owned()]);
}
