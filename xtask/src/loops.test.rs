//! The tests of the bench of a loop, on a folder in a yard.

use std::{fs, process::id};

use serde_json::{Value, json};

use super::{made, scored};
use crate::yard::Yard;

#[test]
fn a_seed_makes_the_same_tickets_of_each_label_and_the_score_counts_the_right_labels() {
  let yard = Yard::new(format!("xtask-loops-{}", id()));
  let read = |name: &str| -> Value {
    serde_json::from_str(&fs::read_to_string(yard.join(name)).unwrap()).unwrap()
  };
  made(&yard, 7, 16).unwrap();
  let (tickets, known) = (read("tickets.json"), read("answers.json"));
  made(&yard, 7, 16).unwrap();
  assert_eq!(read("tickets.json"), tickets, "one seed gives the same tickets");
  let tickets = tickets.as_array().unwrap();
  assert_eq!(tickets.len(), 16);
  assert!(
    tickets.iter().all(|one| !one["text"].as_str().unwrap().contains('{')),
    "each word is filled"
  );
  let known = known.as_object().unwrap();
  let refunds = known.values().filter(|label| *label == "refund").count();
  assert_eq!(refunds, 2, "each label in turn");
  let mut given: serde_json::Map<String, Value> = known.clone();
  let first = given.keys().next().unwrap().clone();
  given.insert(first, json!("nothing"));
  let wrongly = given.keys().nth(1).unwrap().clone();
  let label = given[&wrongly].as_str().unwrap().to_uppercase() + ".";
  given.insert(wrongly, json!(label));
  fs::create_dir_all(yard.join("furb")).unwrap();
  fs::write(yard.join("furb/labels.json"), Value::Object(given).to_string()).unwrap();
  let server = json!({"requests": 16, "prompt_tokens": 1600, "reused": 0, "output_tokens": 32, "prompt_ms": 8000, "decode_ms": 8000});
  fs::write(
    yard.join("furb/run.json"),
    json!({"code": 0, "wall_s": 32.0, "server": server}).to_string(),
  )
  .unwrap();
  let score = scored(&yard).unwrap();
  let furb = &score["furb"];
  assert_eq!(
    (furb["right"].clone(), furb["labeled"].clone(), furb["share"].clone()),
    (json!(15), json!(16), json!(0.938))
  );
  assert_eq!(
    (furb["prompt_tokens_per_ticket"].clone(), furb["server_s_per_ticket"].clone()),
    (json!(100.0), json!(1.0))
  );
  assert!(!score.contains_key("claude"), "an arm that did not run has no score");
}
