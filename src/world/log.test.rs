//! The log of a life where no door reaches it: the buffer that holds a line until the run ends,
//! and the read of a line by its head alone, which is a cost that no door shows. The suite proves
//! what a reader takes of a log, and a log after a crossing, through the doors of a host.

use std::fs;

use serde_json::json;

use super::{Line, Log};
use crate::testing::Yard;
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-log-{name}-{}", process::id())).within("record.jsonl")
}

#[test]
fn a_line_reaches_the_file_when_the_run_ends() {
  let record = place("buffered");
  let mut log = Log::open(&record, &[], None);
  log.line(&json!(["stand", "stand1", "operator", "chain1"]));
  let fresh = format!("{}.log.new", record.display());
  assert_eq!(fs::read_to_string(&fresh).unwrap(), "", "the lines wait in the buffer");
  log.sink.flush();
  assert_eq!(
    fs::read_to_string(&fresh).unwrap(),
    "{\"entries\":0}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n"
  );
}

#[test]
fn a_line_that_the_buffer_did_not_take_waits_and_lands_after_the_lines_before_it() {
  let record = place("waiting");
  let mut log = Log::open(&record, &[], None);
  log.line(&json!(["stand", "stand1", "operator", "chain1"]));
  log.sink.waiting.push_str("[\"done\",\"stand1\",\"journal\",[]]\n");
  log.line(&json!(["stand", "stand2", "operator", "chain1"]));
  log.sink.flush();
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
