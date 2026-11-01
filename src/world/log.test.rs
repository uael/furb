//! The log of a life, proved where no life reaches it: what a reader takes of it.

use std::{fs, io::Write, path::PathBuf};

use serde_json::json;

use super::{Log, logged};

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-log-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at.join("record.jsonl")
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
fn a_line_that_the_ear_could_not_write_waits_and_lands_after_what_the_failed_write_left_is_cut() {
  let record = place("torn");
  let mut log = Log::open(&record, &[]);
  log.line(&json!(["stand", "stand1", "operator", "chain1"]));
  let fresh = format!("{}.log.new", record.display());
  // A write that failed on a full disk left a part of its line, and its line waits.
  fs::OpenOptions::new().append(true).open(&fresh).unwrap().write_all(b"[\"done\",\"sta").unwrap();
  log.torn = true;
  log.waiting.push_str("[\"done\",\"stand1\",\"journal\",[]]\n");
  log.behind.set(true);
  log.line(&json!(["stand", "stand2", "operator", "chain1"]));
  assert!(!log.behind.get());
  assert_eq!(
    fs::read_to_string(&fresh).unwrap(),
    "{\"entries\":0}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"done\",\"stand1\",\"journal\",[]]\n[\"stand\",\"stand2\",\"operator\",\"chain1\"]\n"
  );
}
