//! The task `bench-loop`: the steps that ask no model, which make the tickets and score the arms,
//! on a folder in a yard.

use std::{fs, path::Path, process};

use serde_json::{Value, json};

use crate::{ended, xtask, yard::Yard};

/// The step of the bench on the folder, with these options; a step that fails fails the test.
fn step(yard: &Path, words: &[&str]) {
  let (code, _, err) = ended(&mut xtask(yard, [&["bench-loop"][..], words].concat()));
  assert_eq!(code, Some(0), "{words:?}: {err}");
}

#[test]
fn a_seed_makes_the_same_tickets_of_each_label_and_the_score_counts_the_right_labels() {
  let yard = Yard::new(format!("xtask-loops-{}", process::id()));
  let read = |name: &str| -> Value {
    serde_json::from_str(&fs::read_to_string(yard.join(name)).unwrap_or_default())
      .unwrap_or_default()
  };
  let seeded = ["make", ".", "--seed", "7", "--count", "16"];
  step(&yard, &seeded);
  let (tickets, known) = (read("tickets.json"), read("answers.json"));
  step(&yard, &seeded);
  assert_eq!(read("tickets.json"), tickets, "one seed gives the same tickets");
  let tickets = tickets.as_array().cloned().unwrap_or_default();
  assert_eq!(tickets.len(), 16);
  assert!(
    tickets.iter().all(|one| !one["text"].as_str().unwrap_or("{").contains('{')),
    "each word is filled"
  );
  let known = known.as_object().cloned().unwrap_or_default();
  let refunds = known.values().filter(|label| *label == "refund").count();
  assert_eq!(refunds, 2, "each label in turn");
  let mut given = known;
  let keys: Vec<String> = given.keys().take(2).cloned().collect();
  given.insert(keys[0].clone(), json!("nothing"));
  let label = given[&keys[1]].as_str().unwrap_or_default().to_uppercase() + ".";
  given.insert(keys[1].clone(), json!(label));
  let server = json!({
    "requests": 16, "prompt_tokens": 1600, "reused": 0, "output_tokens": 32, "prompt_ms": 8000,
    "decode_ms": 8000
  });
  let written = fs::create_dir_all(yard.join("furb"))
    .and_then(|()| fs::write(yard.join("furb/labels.json"), Value::Object(given).to_string()))
    .and_then(|()| {
      fs::write(
        yard.join("furb/run.json"),
        json!({"code": 0, "wall_s": 32.0, "server": server}).to_string(),
      )
    });
  written.unwrap_or_else(|no| panic!("the arm furb was not written: {no}"));
  step(&yard, &["score", "."]);
  let score = read("score.json");
  let furb = &score["furb"];
  assert_eq!(
    (furb["right"].clone(), furb["labeled"].clone(), furb["share"].clone()),
    (json!(15), json!(16), json!(0.938)),
    "a label in another case or with a point is right, and another label is not"
  );
  assert_eq!(
    (furb["prompt_tokens_per_ticket"].clone(), furb["server_s_per_ticket"].clone()),
    (json!(100.0), json!(1.0))
  );
  assert!(score.get("claude").is_none(), "an arm that did not run has no score");
}
