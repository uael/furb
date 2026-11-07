//! The ladder of a chain as the crate folds it from the acts of a life.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{Rung, SHOWN, Shown, ladder};
use crate::view::{Act, Fold};

fn folded(facts: impl IntoIterator<Item = Value>) -> Vec<Act> {
  let mut fold = Fold::default();
  let mut acts: HashMap<String, Act> = HashMap::new();
  for fact in facts {
    for act in fold.hear(&fact, &[], None) {
      acts.insert(act.id.clone(), act);
    }
  }
  let mut all: Vec<Act> = acts.into_values().collect();
  all.sort_by_key(|act| act.began);
  all
}

/// Each line of a ladder as a short text: a step as its id, an ear as its rungs, and a fold as
/// its count and key, each after its depth.
fn lines(rungs: &[Rung]) -> Vec<String> {
  rungs
    .iter()
    .map(|rung| match rung {
      Rung::Step { id, depth } => format!("{depth} {id}"),
      Rung::Ear { rungs, depth } => format!("{depth} ear {}", rungs.join(",")),
      Rung::More { depth, count, key, heads, .. } => {
        format!("{depth} more {count} {key} {}", heads.join(","))
      }
    })
    .collect()
}

/// A chain with eight prompts of the operator, each answered by one rung that ran.
fn eight() -> Vec<Value> {
  let mut facts = vec![json!(["chain", "chain1", "operator", "", "root", ""])];
  for n in 1..=8 {
    facts.push(json!(["prompt", format!("prompt{n}"), "operator", "chain1", "str", "Go.", ""]));
    facts.push(json!([
      "rung",
      format!("rung{n}"),
      format!("prompt{n}"),
      "chain1",
      "x = 1",
      "",
      ""
    ]));
    facts.push(json!([
      "run",
      format!("run{n}"),
      "chain1",
      "chain1",
      format!("rung{n}"),
      "x = 1",
      ""
    ]));
    facts.push(json!(["done", format!("run{n}"), "chain1", null]));
    facts.push(json!(["done", format!("rung{n}"), "chain1", null]));
    facts.push(json!(["done", format!("prompt{n}"), "", "ok"]));
  }
  facts
}

#[test]
fn the_ladder_shows_the_last_prompts_and_folds_the_earlier_ones_until_the_window_opens_them() {
  let acts = folded(eight());
  let none: Vec<String> = Vec::new();
  let got = lines(&ladder(&acts, "chain1", &none, &none, None, SHOWN));
  assert_eq!(got[0], "0 more 2 prompts:chain1 prompt");
  assert_eq!(&got[1..3], ["0 prompt3", "1 rung3"]);
  assert_eq!(got.len(), 13);
  let opened = vec!["prompts:chain1".to_owned()];
  let all = lines(&ladder(&acts, "chain1", &opened, &none, None, SHOWN));
  assert_eq!(all.len(), 16);
  assert_eq!(&all[..2], ["0 prompt1", "1 rung1"]);
}

#[test]
fn a_step_under_which_the_operator_is_asked_stays_shown_and_a_fold_holds_two_steps_at_least() {
  let mut facts = eight();
  facts.push(json!(["prompt", "prompt20", "rung2", "chain1", "bool", "Keep it?", "operator"]));
  facts.push(json!(["started", "prompt20", "console"]));
  let acts = folded(facts);
  let owed = vec!["prompt20".to_owned()];
  let shown = Shown { prompts: 5, rungs: 4 };
  let got = lines(&ladder(&acts, "chain1", &[], &owed, None, shown));
  // The prompt over the question shows with its rung and the question, and the lone prompts
  // around it show too, since a fold holds two steps in a row at least.
  assert_eq!(&got[..4], ["0 prompt1", "1 rung1", "0 prompt2", "1 rung2"]);
  assert_eq!(got[4], "2 prompt20");
  // The last five prompts show, and nothing folds, since each run of earlier ones is one step.
  assert_eq!(&got[5..8], ["0 prompt3", "1 rung3", "0 prompt4"]);
  assert!(got.iter().all(|line| !line.contains("more")));
  // A branch shows only the messages that it took.
  let taken = vec!["prompt7".to_owned()];
  let branch = lines(&ladder(&acts, "chain1", &[], &owed, Some(taken.as_slice()), shown));
  assert_eq!(branch, ["0 prompt7", "1 rung7"]);
}
