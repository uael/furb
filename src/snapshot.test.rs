//! The keeper of a snapshot: when a life asks it for one.

use std::{fs, path::PathBuf};

use super::Keeper;

/// A new folder of the test, empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-snapshot-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

#[test]
fn a_keeper_says_that_the_life_said_nothing_new_from_the_length_of_its_log_alone() {
  let record = place("grew").join("record.jsonl");
  let log = format!("{}.log", record.display());
  let mut keeper = Keeper::new(record, "build".to_owned());
  assert!(keeper.grew(), "a life with no log yet is asked for its snapshot");
  fs::write(&log, "one\n").unwrap();
  assert!(keeper.grew(), "a log that no snapshot covers holds something new");
  keeper.covered = Some(4);
  assert!(!keeper.grew(), "a log of the length that the last snapshot covers holds nothing new");
  fs::write(&log, "one\ntwo\n").unwrap();
  assert!(keeper.grew(), "a log that grew since the last snapshot holds something new");
}
