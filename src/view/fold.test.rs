//! The fold of a record with no engine, against the fold of a live life whose engine finds what a
//! pause and a wake cover.

use serde_json::{Value, json};

use super::Fold;
use crate::view::Pause;

/// A paused prompt, a rung born under its pause, and a later wake, with what the engine of the live
/// life finds for each fact: the acts that it covers, and the pause that holds a new act.
fn facts() -> Vec<(Value, Vec<&'static str>, Option<Pause>)> {
  let paused = Pause { by: "operator".to_owned(), why: Some(json!("held")) };
  vec![
    (json!(["chain", "chain1", "operator", "", "root", ""]), vec![], None),
    (json!(["prompt", "prompt1", "operator", "chain1", "int", "count", ""]), vec![], None),
    (json!(["pause", "prompt1", "operator", "held"]), vec!["prompt1"], None),
    (json!(["rung", "rung1", "prompt1", "chain1", "x = 1", "", ""]), vec![], Some(paused)),
    (json!(["bash", "bash1", "chain1", "chain1", "ls", "", 120.0, null]), vec![], None),
    (json!(["wake", "prompt1", "operator"]), vec!["prompt1", "rung1"], None),
  ]
}

#[test]
fn a_record_folds_each_pause_and_wake_and_each_act_born_under_a_pause_as_its_live_life_did() {
  let (mut live, mut recorded) = (Fold::default(), Fold::default());
  for (fact, covered, born) in facts() {
    let covered: Vec<String> = covered.into_iter().map(str::to_owned).collect();
    let lived = live.hear(&fact, &covered, born);
    let read = recorded.recorded(&fact);
    assert_eq!(read, lived, "after {fact}");
  }
  let paused = |fold: &Fold, pausing| fold.open(pausing);
  assert_eq!(paused(&recorded, false), Vec::<String>::new(), "the wake let each act go");
  let mut held = Fold::default();
  for (fact, _, _) in facts().into_iter().take(4) {
    held.recorded(&fact);
  }
  assert_eq!(
    held.open(false),
    ["prompt1", "rung1"],
    "the pause holds the prompt and the rung born under it"
  );
}

#[test]
fn a_chain_with_a_source_takes_each_message_of_the_operator_that_its_prefix_carries() {
  let mut fold = Fold::default();
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "first", ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "str", "a worker", ""]),
    json!(["prompt", "prompt3", "operator/tui", "chain1", "str", "second", ""]),
    json!(["chain", "chain2", "operator", "chain1", "branch", "chain1"]),
    json!([
      "prefix",
      "chain2",
      "chain2",
      [
        ["prompt", "prompt1", "operator", "chain1", "str", "first", ""],
        ["started", "prompt1", "chain1"],
        ["prompt", "prompt2", "rung1", "chain1", "str", "a worker", ""],
        ["prompt", "prompt3", "operator/tui", "chain1", "str", "second", ""],
        ["prompt", "prompt1", "operator", "chain1", "str", "first", ""]
      ]
    ]),
  ];
  let mut last = Vec::new();
  for fact in &facts {
    last = fold.recorded(fact);
  }
  let chain = last.iter().find(|act| act.id == "chain2").expect("the prefix moves its chain");
  assert_eq!(chain.taken, ["prompt1", "prompt3"], "each message of the operator, once, in order");
  assert!(last.iter().all(|act| act.id != "chain1"), "the source takes nothing: {last:?}");
}
