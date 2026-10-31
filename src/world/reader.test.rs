//! The reader of a record, proved where no life reaches it: the facts that it says of what a record kept.

use std::{fs, path::PathBuf};

use super::{retold, told};
use crate::{value::Object, wire::parsed};

/// The facts of lines of JSON, as a record keeps them.
fn facts(lines: &[&str]) -> Vec<Object> {
  lines.iter().map(|one| parsed(one).unwrap()).collect()
}

/// Each fact as python shows it.
fn shown(facts: &[Object]) -> Vec<String> {
  facts.iter().map(|one| one.as_ref().py_repr()).collect()
}

/// The facts of the file that a read of a record names, as the file holds them.
fn filed(read: &super::Retold) -> Vec<Object> {
  let text = fs::read_to_string(&read.file).unwrap();
  text
    .lines()
    .filter(|one| one.starts_with('[') && !one.starts_with(r#"["keep","","#))
    .map(|one| parsed(one).unwrap())
    .collect()
}

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-reader-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at.join("record.jsonl")
}

/// The opening of a life: its root chain, a prompt of the operator on it, and the reply that rung1 asked for.
const OPENS: [&str; 3] = [
  r#"["chain", "chain1", "operator", "", "root", ""]"#,
  r#"["prompt", "prompt1", "operator", "chain1", "str", "Go.", ""]"#,
  r#"["reply", "reply1", "rung1", "chain1", "opus"]"#,
];

/// The done of reply1 with a word, as a record keeps a tuple.
fn answered(word: &str) -> String {
  format!(
    r#"["done", "reply1", "provider", {{"is": "tuple", "args": [["assistant", "{word}", [1, 2, 3, 4, 0.5], null]]}}]"#
  )
}

#[test]
fn a_kept_rung_takes_its_place_on_the_open_prompt_with_the_run_of_the_word_that_the_gate_let_run() {
  let done = answered("x = 1");
  let kept = facts(&[
    OPENS[0],
    OPENS[1],
    OPENS[2],
    &done,
    r#"["gate", "gate1", "chain1", "chain1", "x = 1"]"#,
    r#"["done", "gate1", "gate", []]"#,
    r#"["bash", "bash1", "rung1", "chain1", "ls", "", 600]"#,
    r#"["reply", "reply2", "rung2", "chain1", "opus"]"#,
  ]);
  let told = shown(&told(kept.clone()));
  assert_eq!(told[2], "['rung', 'rung1', 'prompt1', 'chain1', '', '', '']");
  assert_eq!(told[3], "['reply', 'reply1', 'rung1', 'chain1', 'opus']");
  assert_eq!(
    told[7..],
    [
      "['run', 'run1', 'chain1', 'chain1', 'rung1', 'x = 1', '']",
      "['bash', 'bash1', 'rung1', 'chain1', 'ls', '', 600]",
      "['done', 'run1', 'reader', None]",
      "['done', 'rung1', 'reader', None]",
      "['rung', 'rung2', 'prompt1', 'chain1', '', '', '']",
      "['reply', 'reply2', 'rung2', 'chain1', 'opus']",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_chain_that_the_record_does_not_hold_is_made_by_the_rung_of_its_first_prompt() {
  let kept = facts(&[
    OPENS[0],
    OPENS[1],
    OPENS[2],
    r#"["prompt", "prompt2", "rung1", "chain2", "str", "Help.", ""]"#,
  ]);
  assert_eq!(
    shown(&told(kept.clone()))[4..],
    [
      "['chain', 'chain2', 'rung1', 'chain1', 'Help.', '']",
      "['prompt', 'prompt2', 'rung1', 'chain2', 'str', 'Help.', '']",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_chain_that_the_record_does_not_hold_takes_the_first_line_of_its_first_prompt_as_its_label() {
  let long = format!("\\n  {}  \\nSecond line.", "a".repeat(100));
  let first = format!(r#"["prompt", "prompt2", "operator", "chain2", "str", "{long}", ""]"#);
  let kept = facts(&[
    OPENS[0],
    &first,
    r#"["prompt", "prompt3", "operator", "chain2", "str", "Later.", ""]"#,
  ]);
  let told = shown(&told(kept.clone()));
  assert_eq!(told[1], format!("['chain', 'chain2', 'operator', '', '{}…', '']", "a".repeat(80)));
}

#[test]
fn a_chain_that_the_record_does_not_hold_and_that_no_prompt_names_takes_its_own_name_as_its_label()
{
  let kept = facts(&[OPENS[0], r#"["bash", "bash1", "rung3", "chain2", "ls", "", 600]"#]);
  assert_eq!(shown(&told(kept.clone()))[1], "['chain', 'chain2', 'operator', '', 'chain2', '']");
}

#[test]
fn a_chain_that_the_record_does_not_hold_stands_under_the_chain_that_its_maker_ran_on() {
  let kept = facts(&[
    OPENS[0],
    r#"["prompt", "prompt2", "rung5", "chain2", "str", "Help.", ""]"#,
    r#"["bash", "bash1", "rung5", "chain3", "ls", "", 600]"#,
  ]);
  assert_eq!(shown(&told(kept.clone()))[1], "['chain', 'chain2', 'rung5', 'chain3', 'Help.', '']");
}

#[test]
fn a_word_that_the_gate_refused_tells_its_findings_and_runs_not() {
  let done = answered("BAD");
  let kept = facts(&[
    OPENS[0],
    OPENS[1],
    OPENS[2],
    &done,
    r#"["gate", "gate1", "chain1", "chain1", "BAD"]"#,
    r#"["done", "gate1", "gate", ["line 1: BAD is unbound"]]"#,
  ]);
  assert_eq!(
    shown(&told(kept.clone()))[7..],
    [
      "['tell', 'rung1', 'reader', [\"#rung1 refused\\nrung1_findings = 'line 1: BAD is unbound'\"]]",
      "['done', 'rung1', 'reader', {'is': 'Refused', 'args': []}]",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_prompt_that_the_operator_closed_is_done_with_its_value_and_ends_its_rung() {
  let kept = facts(&[
    OPENS[0],
    OPENS[1],
    OPENS[2],
    r##"["close", "prompt1", "console", "done", ["#prompt1 closed"]]"##,
    r#"["cancel", "chain1", "operator"]"#,
  ]);
  assert_eq!(
    shown(&told(kept.clone()))[4..],
    [
      "['close', 'prompt1', 'console', 'done', ['#prompt1 closed']]",
      "['done', 'prompt1', 'reader', 'done']",
      "['done', 'rung1', 'reader', None]",
      "['cancel', 'chain1', 'operator']",
      "['done', 'chain1', 'reader', {'is': 'CancelledError', 'args': []}]",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_record_names_its_engine_and_reads_its_log_while_the_log_agrees() {
  let at = place("retold");
  fs::write(
    &at,
    "[[\"made\",\"chain1\",\"store\",\"abc\"]]\n[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n",
  )
  .unwrap();
  let read = retold(&at).unwrap();
  assert_eq!(read.made.as_deref(), Some("abc"));
  assert!(!read.logged);
  assert_eq!(
    shown(&filed(&read)),
    [
      "['made', 'chain1', 'store', 'abc']",
      "['chain', 'chain1', 'operator', '', 'chain1', '']",
      "['stand', 'stand1', 'operator', 'chain1']",
    ]
    .map(str::to_owned)
  );
  fs::write(
    format!("{}.log", at.display()),
    "{\"entries\":2}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"tell\",\"chain1\",\"chain1\",[\"#chain1\"]]\n",
  )
  .unwrap();
  let read = retold(&at).unwrap();
  assert!(read.logged);
  assert_eq!(read.file, PathBuf::from(format!("{}.log", at.display())));
  assert_eq!(
    shown(&filed(&read)),
    ["['stand', 'stand1', 'operator', 'chain1']", "['tell', 'chain1', 'chain1', ['#chain1']]"]
      .map(str::to_owned)
  );
}

#[test]
fn a_kept_rung_with_a_word_runs_it_once_the_gate_lets_it_run() {
  let kept = facts(&[
    OPENS[0],
    r#"["rung", "rung1", "operator", "chain1", "x = 1", "", ""]"#,
    r#"["gate", "gate1", "chain1", "chain1", "x = 1"]"#,
    r#"["done", "gate1", "gate", []]"#,
  ]);
  assert_eq!(
    shown(&told(kept.clone()))[4],
    "['run', 'run1', 'chain1', 'chain1', 'rung1', 'x = 1', '']"
  );
}

#[test]
fn a_rung_takes_no_place_on_a_prompt_that_it_made() {
  let kept = facts(&[
    OPENS[0],
    r#"["prompt", "prompt1", "rung1", "chain1", "str", "Ask.", ""]"#,
    r#"["bash", "bash1", "rung1", "chain1", "ls", "", 600]"#,
  ]);
  assert_eq!(shown(&told(kept.clone()))[2], "['rung', 'rung1', 'chain1', 'chain1', '', '', '']");
}

#[test]
fn a_second_read_of_a_record_that_did_not_change_takes_the_file_of_what_it_tells_as_it_stands() {
  let at = place("cached");
  fs::write(&at, "[[\"made\",\"chain1\",\"store\",\"abc\"]]\n").unwrap();
  let read = retold(&at).unwrap();
  let told = PathBuf::from(format!("{}.told", at.display()));
  assert_eq!(read.file, told);
  let head = fs::read_to_string(&told).unwrap().lines().next().unwrap().to_owned();
  fs::write(&told, format!("{head}\n[\"tell\",\"chain1\",\"chain1\",[\"kept\"]]\n")).unwrap();
  let again = retold(&at).unwrap();
  assert_eq!(again.made.as_deref(), Some("abc"));
  assert_eq!((again.entries, &again.last), (read.entries, &read.last));
  assert_eq!(again.entries, 1);
  assert!(again.last.is_some());
  assert_eq!(shown(&filed(&again)), ["['tell', 'chain1', 'chain1', ['kept']]"]);
  fs::write(&at, "[[\"made\",\"chain1\",\"store\",\"abd\"]]\n").unwrap();
  let changed = retold(&at).unwrap();
  assert_eq!(changed.made.as_deref(), Some("abd"));
  assert_eq!(shown(&filed(&changed)), ["['made', 'chain1', 'store', 'abd']"]);
}

#[test]
fn a_record_that_keeps_a_tuple_and_a_whole_float_tells_them_as_the_engine_gives_them() {
  let at = place("plain");
  let done = answered("x = 1").replace("0.5]", r#"{"is": "float", "args": ["2"]}]"#);
  fs::write(
    &at,
    OPENS.iter().chain([&done.as_str()]).map(|one| format!("[{one}]\n")).collect::<String>(),
  )
  .unwrap();
  let read = retold(&at).unwrap();
  let text = fs::read_to_string(&read.file).unwrap();
  let line = text.lines().find(|one| one.starts_with(r#"["done","reply1""#)).unwrap();
  assert_eq!(line, r#"["done","reply1","provider",["assistant","x = 1",[1,2,3,4,2.0],null]]"#);
}
