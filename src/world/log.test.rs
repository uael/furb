//! The log of a life, proved where no life reaches it: what a reader takes of it.

use std::fs;

use serde_json::json;

use super::{Line, Log, logged};
use crate::testing::Yard;
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-log-{name}-{}", process::id())).within("record.jsonl")
}

#[test]
fn a_log_agrees_when_it_counts_each_entry_of_its_record_and_a_torn_last_line_stays_out() {
  let at = place("logged");
  let log = format!("{}.log", at.display());
  let record =
    "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n[[\"done\",\"stand1\",\"provider\",1]]\n";
  fs::write(&at, record).unwrap();
  let lines = "{\"entries\":1}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"rung\",\"rung1\",\"operator\",\"chain1\",\"x = 1\",\"\",\"\"]\n[\"keep\",\"\",2]\n[\"do";
  fs::write(&log, lines).unwrap();
  let facts = logged(&at).unwrap().expect("the log counts both entries");
  assert_eq!(facts.len(), 2);
  assert_eq!(
    facts[1].as_ref().py_repr(),
    "['rung', 'rung1', 'operator', 'chain1', 'x = 1', '', '']"
  );
  fs::write(
    &log,
    "{\"entries\":1}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"keep\",\"\",3]\n",
  )
  .unwrap();
  assert!(logged(&at).unwrap().is_none(), "a log that skips an entry does not agree");
  fs::remove_file(&log).unwrap();
  assert!(logged(&at).unwrap().is_none(), "a record with no log has none");
}

#[test]
fn a_log_that_a_drifted_replay_left_short_of_an_act_of_its_record_does_not_agree() {
  let at = place("short");
  let log = format!("{}.log", at.display());
  let record = "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n[[\"bash\",\"bash1\",\"rung1\",\"chain1\",\"ls\",\"\",600]]\n";
  fs::write(&at, record).unwrap();
  fs::write(&log, "{\"entries\":2}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n").unwrap();
  assert!(logged(&at).unwrap().is_none(), "the log shows no bash1");
}

#[test]
fn a_line_reaches_the_file_when_the_run_ends() {
  let record = place("buffered");
  let mut log = Log::open(&record, &[]);
  log.line(&json!(["stand", "stand1", "operator", "chain1"]));
  let fresh = format!("{}.log.new", record.display());
  assert_eq!(fs::read_to_string(&fresh).unwrap(), "", "the lines wait in the buffer");
  log.sink.borrow_mut().flush();
  assert_eq!(
    fs::read_to_string(&fresh).unwrap(),
    "{\"entries\":0}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n"
  );
}

#[test]
fn a_line_that_the_buffer_did_not_take_waits_and_lands_after_the_lines_before_it() {
  let record = place("waiting");
  let mut log = Log::open(&record, &[]);
  log.line(&json!(["stand", "stand1", "operator", "chain1"]));
  log.sink.borrow_mut().waiting.push_str("[\"done\",\"stand1\",\"journal\",[]]\n");
  log.line(&json!(["stand", "stand2", "operator", "chain1"]));
  log.sink.borrow_mut().flush();
  assert_eq!(
    fs::read_to_string(format!("{}.log.new", record.display())).unwrap(),
    "{\"entries\":0}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"done\",\"stand1\",\"journal\",[]]\n[\"stand\",\"stand2\",\"operator\",\"chain1\"]\n"
  );
}

#[test]
fn a_line_is_read_from_its_head_alone() {
  assert!(matches!(Line::of("{\"entries\":1}"), Line::Head));
  assert!(matches!(Line::of("[\"keep\",\"\",7]"), Line::Keep(Some(7))));
  assert!(matches!(Line::of("[\"keep\",\"\",x]"), Line::Keep(None)));
  assert!(
    matches!(Line::of("[\"bash\",\"bash3\",\"rung1\"]"), Line::Question(name) if name == "bash3")
  );
  assert!(matches!(Line::of("[\"done\",\"bash3\",\"journal\"]"), Line::Other));
  assert!(
    matches!(Line::of("[\"b\\u0061sh\",\"bash3\",\"rung1\"]"), Line::Question(name) if name == "bash3")
  );
  assert!(matches!(Line::of("[1,2]"), Line::Other));
}
