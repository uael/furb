//! The fold of a record with no engine, against the fold of a live life whose engine finds what a
//! pause and a wake cover.

use serde_json::{Value, json};

use std::collections::HashMap;

use super::Fold;
use crate::view::{Pause, Role};

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

#[test]
fn a_chain_with_a_source_takes_the_messages_of_the_operator_that_its_fact_names_with_no_prefix() {
  let mut fold = Fold::default();
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["prompt", "prompt1", "operator", "chain1", "str", "first", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "x = 1", "", ""]),
    json!(["prompt", "prompt2", "rung1", "chain1", "str", "a worker", ""]),
    json!(["prompt", "prompt3", "operator", "chain1", "str", "second", ""]),
    json!([
      "chain",
      "chain2",
      "rung1",
      "chain1",
      "branch",
      "chain1",
      ["prompt1", "rung1", "prompt2"]
    ]),
  ];
  let mut last = Vec::new();
  for fact in &facts {
    last = fold.recorded(fact);
  }
  let chain = last.iter().find(|act| act.id == "chain2").expect("the chain");
  assert_eq!(
    chain.taken,
    ["prompt1"],
    "the messages of the operator that the chain carries, as a record holds them"
  );
}

/// The cost of an answer as a float of many digits, and the bits of the float that Python `float()`
/// reads from the same text: the shapes of the costs of the record of a conductor, whose last digit
/// the fast path of `serde_json` reads wrong.
const COSTS: [(&str, u64); 8] = [
  ("0.10028040000000005", 0x3fb9_abf9_ee6e_0748),
  ("0.025178000000000006", 0x3f99_c842_fa50_9398),
  ("0.028432999999999986", 0x3f9d_1d8a_5482_3850),
  ("0.11862220000000001", 0x3fbe_5e06_4594_5ec8),
  ("0.18088700000000002", 0x3fc7_274e_22a2_c238),
  ("0.23607699999999998", 0x3fce_37c5_692b_3cc4),
  ("0.056088600000000044", 0x3fac_b7a5_1d5b_5aa0),
  ("0.12446639999999987", 0x3fbf_dd07_ad73_6b28),
];

#[test]
fn a_record_folds_the_cost_of_each_answer_as_the_float_that_python_reads_from_its_text() {
  let mut fold = Fold::default();
  for (n, (cost, bits)) in (1..).zip(COSTS) {
    let record = [
      format!(r#"["chain","chain{n}","operator","","root",""]"#),
      format!(r#"["reply","reply{n}","operator","chain{n}","m/high"]"#),
      format!(r#"["done","reply{n}","journal",["assistant","",[10,2,0,0,{cost}],[]]]"#),
    ];
    let mut moved = Vec::new();
    for line in record {
      moved = fold.recorded(&serde_json::from_str::<Value>(&line).expect("a fact"));
    }
    let spend = moved.iter().find_map(|act| act.spend.clone()).expect("the answer moves its chain");
    assert_eq!(spend.dollars.to_bits(), bits, "the cost {cost}");
  }
}

#[test]
fn each_act_names_the_role_of_its_maker_live_and_from_its_record_alike() {
  let roster = json!([["claude-cli:opus", ["low", "high"], 200_000], ["operator", ["uael"], 0]]);
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["stand", "stand1", "chain1", "chain1"]),
    json!(["done", "stand1", "journal", [roster, "/tmp", "claude-cli:opus/high"]]),
    json!(["rung", "rung1", "chain1", "chain1", "", "", "claude-cli:opus/high"]),
    json!(["bash", "bash1", "rung1", "chain1", "ls", "", 120.0, null]),
    json!(["rung", "rung2", "operator/uael", "chain1", "x = 1", "", ""]),
    json!(["rung", "rung3", "operator", "chain1", "y = 2", "", ""]),
    json!(["rung", "rung4", "operator/tui", "chain1", "z = 3", "", ""]),
    json!(["rung", "rung5", "extensions", "chain1", "w = 4", "", ""]),
    json!(["rung", "rung6", "chain1", "chain1", "stand()", "", "claude-cli:opus/high"]),
  ];
  let (mut live, mut recorded) = (Fold::default(), Fold::default());
  let (mut lived, mut read) = (HashMap::new(), HashMap::new());
  for fact in &facts {
    lived.extend(live.hear(fact, &[], None).into_iter().map(|act| (act.id.clone(), act.maker)));
    read.extend(recorded.recorded(fact).into_iter().map(|act| (act.id.clone(), act.maker)));
  }
  let opus = Role::Model { model: "claude-cli:opus".to_owned(), effort: "high".to_owned() };
  let want = [
    ("rung1", opus.clone()),
    ("bash1", opus),
    ("rung2", Role::Member { login: "uael".to_owned() }),
    ("rung3", Role::Operator),
    ("rung4", Role::Operator),
    ("rung5", Role::Skill),
    ("rung6", Role::Engine),
    ("chain1", Role::Operator),
  ];
  for (id, role) in want {
    assert_eq!(lived.get(id), Some(&role), "{id} live");
    assert_eq!(read.get(id), Some(&role), "{id} from the record");
  }
}

#[test]
fn an_actor_names_its_model_and_its_effort_unless_the_roster_holds_its_whole_name() {
  let model =
    |model: &str, effort: &str| Role::Model { model: model.to_owned(), effort: effort.to_owned() };
  assert_eq!(Role::of("claude-cli:org/plain", &[], &[]), model("claude-cli:org/plain", "off"));
  assert_eq!(
    Role::of("claude-cli:org/plain/high", &[], &[]),
    model("claude-cli:org/plain", "high")
  );
  let whole = ["claude-cli:org/high".to_owned()];
  assert_eq!(Role::of("claude-cli:org/high", &whole, &[]), model("claude-cli:org/high", "off"));
  assert_eq!(Role::of("", &[], &[]), Role::Engine);
}
