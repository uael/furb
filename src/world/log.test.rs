//! The log of a life, proved where no life reaches it: what a reader takes of it.

use std::{fs, path::PathBuf};

use super::logged;

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
  let lines = "{\"entries\":1}\n[\"rung\",\"rung1\",\"operator\",\"chain1\",\"x = 1\",\"\",\"\"]\n[\"keep\",\"\",2]\n[\"do";
  fs::write(&log, lines).unwrap();
  let facts = logged(&at).unwrap().expect("the log counts both entries");
  assert_eq!(facts.len(), 1);
  assert_eq!(
    facts[0].as_ref().py_repr(),
    "['rung', 'rung1', 'operator', 'chain1', 'x = 1', '', '']"
  );
  fs::write(&log, "{\"entries\":1}\n[\"keep\",\"\",3]\n").unwrap();
  assert!(logged(&at).unwrap().is_none(), "a log that skips an entry does not agree");
  fs::remove_file(&log).unwrap();
  assert!(logged(&at).unwrap().is_none(), "a record with no log has none");
}
