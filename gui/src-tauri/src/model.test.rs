use serde_json::{Value, json};

use super::{LONG, Life, lean, literal};

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
fn a_prompt_that_the_console_took_is_owed() {
  assert_eq!(Life::of(&facts()).owed(), vec!["prompt2".to_owned()]);
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
fn a_binding_belongs_to_the_act_its_name_starts_with() {
  let mut facts = facts();
  facts.push(json!([
    "tell",
    "bash1",
    "chain1",
    ["#bash1 exited 0\nbash1_command = 'ls'\nreview = 'fine'"]
  ]));
  let found: Vec<(String, Option<String>)> =
    Life::of(&facts).bindings("chain1").into_iter().map(|one| (one.name, one.act)).collect();
  assert_eq!(
    found,
    vec![("bash1_command".to_owned(), Some("bash1".to_owned())), ("review".to_owned(), None)]
  );
}

#[test]
fn a_long_text_of_a_fact_is_cut_and_keeps_its_count_of_lines() {
  let content = "line\n".repeat(LONG / 4);
  let kept = lean(json!(["done", "read1", "files", {"is": "Text", "content": content}]));
  assert_eq!(kept[3]["lines"], LONG / 4);
  assert!(kept[3]["content"].as_str().is_some_and(|one| one.chars().count() == LONG + 1));
}

#[test]
fn a_string_of_python_reads_back_with_its_escapes() {
  assert_eq!(literal(r"'it\'s\nhere'"), "it's\nhere");
}
