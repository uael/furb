use serde_json::{Value, json};

use super::{LONG, Life, TEXT, lean, literal};

/// A small life: the operator asks the root, whose model reads a file, runs a command and asks the operator.
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
    json!(["run", "run1", "chain1", "chain1", "rung1", "x = read('a')", ""]),
    json!(["read", "read1", "rung1", "chain1", "a"]),
    json!(["done", "read1", "files", {"is": "Text", "path": "a", "content": "1\n2"}]),
    json!(["bash", "bash1", "rung1", "chain1", "ls", "", 600.0]),
    json!(["started", "bash1", "bash"]),
    json!(["out", "bash1", "bash", "a\n", "stdout"]),
    json!(["out", "bash1", "bash", "no\n", "stderr"]),
    json!(["prompt", "prompt2", "rung1", "chain1", "str", "Which?", "operator"]),
    json!(["started", "prompt2", "console"]),
  ]
}

#[test]
fn an_act_takes_its_place_in_time_from_one() {
  let life = Life::of(&facts());
  let read = life.act("read1").expect("the read stands");
  assert_eq!((read.began, read.ended), (7, Some(8)));
}

#[test]
fn a_step_of_the_engine_is_no_act() {
  let life = Life::of(&facts());
  assert!(life.act("run1").is_none());
}

#[test]
fn a_command_that_runs_holds_what_it_printed_on_each_stream() {
  let life = Life::of(&facts());
  let live = life.live(life.act("bash1").expect("the command stands"));
  assert_eq!(live["value"]["stdout"]["content"], "a\n");
  assert_eq!(live["value"]["stderr"]["content"], "no\n");
}

#[test]
fn a_pause_of_a_chain_holds_each_open_act_on_it() {
  let mut facts = facts();
  facts.push(json!(["pause", "chain1", "operator"]));
  let life = Life::of(&facts);
  assert!(life.paused("bash1"));
  assert_eq!(life.live(life.act("read1").expect("the read stands"))["paused"], false);
}

#[test]
fn a_wake_after_a_pause_frees_the_acts() {
  let mut facts = facts();
  facts.push(json!(["pause", "chain1", "operator"]));
  facts.push(json!(["wake", "chain1", "operator"]));
  assert!(!Life::of(&facts).paused("bash1"));
}

#[test]
fn a_rung_runs_until_its_run_is_done() {
  let life = Life::of(&facts());
  assert_eq!(life.run(life.act("rung1").expect("the rung stands")).0, "running");
}

#[test]
fn a_rung_whose_word_raised_fails_with_its_exception() {
  let mut facts = facts();
  facts
    .push(json!(["done", "run1", "kernel", {"is": "Refused", "args": ["There is no file at a."]}]));
  let life = Life::of(&facts);
  let run = life.run(life.act("rung1").expect("the rung stands"));
  assert_eq!(run, ("failed", "Refused: There is no file at a.".to_owned()));
}

#[test]
fn a_rung_that_the_gate_refused_fails_with_its_findings() {
  let mut facts = facts();
  facts.push(json!([
    "tell",
    "rung1",
    "chain1",
    ["#rung1 refused\nrung1_findings = 'line 1: no name x'"]
  ]));
  let life = Life::of(&facts);
  let run = life.run(life.act("rung1").expect("the rung stands"));
  assert_eq!(run, ("failed", "line 1: no name x".to_owned()));
}

#[test]
fn the_acts_that_changed_since_a_count_are_given_again() {
  let mut life = Life::of(&facts());
  let count = life.count;
  life.add(&json!(["done", "bash1", "bash", {"is": "Exit", "code": 0}]));
  let changed: Vec<Value> = life.since(count).into_iter().map(|one| one["id"].clone()).collect();
  assert_eq!(changed, vec![json!("bash1")]);
}

#[test]
fn the_measure_of_a_chain_sums_the_dollars_of_its_answers() {
  let mut facts = facts();
  facts.push(json!(["reply", "reply1", "rung1", "chain1", "claude-cli:opus/high"]));
  facts.push(json!([
    "done",
    "reply1",
    "provider",
    ["assistant", "x", [1000, 10, 0, 0, 0.25], null]
  ]));
  let measures = Life::of(&facts).measures();
  assert_eq!(measures["chain1"]["dollars"], 0.25);
  assert_eq!(measures["chain1"]["answers"], 1);
}

#[test]
fn the_turn_of_a_reply_reads_as_a_tuple_of_the_wire() {
  let mut facts = facts();
  facts.push(json!(["reply", "reply1", "rung1", "chain1", "claude-cli:opus/high"]));
  let turn = json!({"is": "tuple", "args": [["assistant", "x", [1000, 10, 0, 0, 0.5], null]]});
  facts.push(json!(["done", "reply1", "provider", turn]));
  assert_eq!(Life::of(&facts).measures()["chain1"]["dollars"], 0.5);
}

#[test]
fn a_rung_takes_the_word_of_its_run_when_no_ready_said_it() {
  let facts = [
    json!(["chain", "chain1", "operator", "", "root", ""]),
    json!(["rung", "rung1", "prompt1", "chain1", "", "", ""]),
    json!(["run", "run1", "reader", "chain1", "rung1", "x = read('a')", ""]),
  ];
  assert_eq!(Life::of(&facts).words.get("rung1").map(String::as_str), Some("x = read('a')"));
}

#[test]
fn the_word_that_a_rung_said_ready_stands_before_the_word_of_its_run() {
  let life = Life::of(&facts());
  let word = life.words.get("rung1").map(String::as_str);
  assert_eq!(word, Some("x = read('a')\nq = await prompt(str, 'Which?', to=OPERATOR)"));
}

/// A record that the store kept, with no log beside it: the operator asks the root, and its model reads a file.
fn kept_record(name: &str) -> std::path::PathBuf {
  let folder = std::env::temp_dir().join(format!("furb-gui-model-{name}-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).expect("the folder of the test is made");
  let path = folder.join("a.jsonl");
  let kept = [
    json!([["chain", "chain1", "operator", "", "root", ""], 1]),
    json!([["prompt", "prompt1", "operator", "chain1", "str", "Do it", ""], 2]),
    json!([["reply", "reply1", "rung1", "chain1", "claude-cli:opus/high"], 3]),
    json!([
      ["done", "reply1", "provider", ["assistant", "x = read('a')", [10, 1, 0, 0, 0.1], null]],
      4
    ]),
    json!([["read", "read1", "rung1", "chain1", "a"], 5]),
    json!([["done", "read1", "files", {"is": "Text", "path": "a", "content": "1"}], 6]),
  ];
  let text: String = kept.iter().map(|one| format!("{one}\n")).collect();
  std::fs::write(&path, text).expect("the record is written");
  path
}

#[test]
fn a_record_that_the_crate_retold_shows_the_rungs_that_it_does_not_keep() {
  let retold = furb::world::retold(kept_record("retold")).expect("the record reads");
  let facts: Vec<Value> =
    retold.facts.iter().map(|one| furb::wire::outward(one.as_ref())).collect();
  let life = Life::of(&facts);
  let rung = life.act("rung1").expect("the reader made the rung");
  assert_eq!(rung.by, "prompt1");
  assert_eq!(life.words.get("rung1").map(String::as_str), Some("x = read('a')"));
  assert_eq!(life.run(rung).0, "running");
}

#[test]
fn a_long_text_of_a_file_is_cut_and_keeps_its_count_of_lines() {
  let content = "line\n\n".repeat(TEXT);
  let kept = lean(json!(["done", "read1", "files", {"is": "Text", "content": content}]));
  assert_eq!(kept[3]["lines"], TEXT);
  assert!(kept[3]["content"].as_str().is_some_and(|one| one.chars().count() == TEXT + 1));
}

#[test]
fn the_text_that_a_write_asks_for_is_cut_as_a_file() {
  let content = "é".repeat(TEXT + 1);
  let kept = lean(
    json!(["write", "write1", "rung1", "chain1", {"is": "Text", "path": "a", "content": content}]),
  );
  assert_eq!(kept[4]["lines"], 1);
  assert_eq!(kept[4]["content"], format!("{}…", "é".repeat(TEXT)));
}

#[test]
fn a_long_text_of_a_fact_is_cut() {
  let kept = lean(json!(["done", "bash1", "bash", "x".repeat(LONG + 5)]));
  assert!(kept[3].as_str().is_some_and(|one| one.chars().count() == LONG + 1));
}

#[test]
fn the_word_of_a_rung_and_the_message_of_a_prompt_stay_whole() {
  let long = "x".repeat(LONG + 5);
  let rung = json!(["rung", "rung1", "prompt1", "chain1", long, "", ""]);
  let prompt = json!(["prompt", "prompt1", "operator", "chain1", "str", long, ""]);
  assert_eq!((lean(rung.clone()), lean(prompt.clone())), (rung, prompt));
}

#[test]
fn a_string_of_python_reads_back_with_its_escapes() {
  assert_eq!(literal(r"'it\'s\nhere'"), "it's\nhere");
}
