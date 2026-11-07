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
