use std::sync::mpsc;

use serde_json::json;

use super::{Event, Teller, read_record};

/// What the app hears when it reads the record at a path.
fn heard(path: &std::path::Path) -> Vec<Event> {
  let (events, told) = mpsc::channel();
  read_record(path, &Teller { events }).expect("the record reads");
  told.try_iter().collect()
}

/// A record with no entry in a folder of its own, and the path of its log.
fn record(name: &str) -> (std::path::PathBuf, String) {
  let folder = std::env::temp_dir().join(format!("furb-gui-{name}-{}", std::process::id()));
  std::fs::create_dir_all(&folder).expect("the folder is made");
  let path = folder.join("record.jsonl");
  std::fs::write(&path, "").expect("the record is written");
  let log = format!("{}.log", path.display());
  (path, log)
}

#[test]
fn a_record_whose_log_agrees_shows_every_fact_of_the_log() {
  let (path, log) = record("logged");
  std::fs::write(
    &log,
    "{\"entries\": 0}\n[\"rung\", \"rung1\", \"chain1\", \"chain1\", \"x = 1\"]\n",
  )
  .expect("the log is written");
  let events = heard(&path);
  assert!(matches!(&events[0], Event::Opened { works: false, logged: true, .. }));
  let Event::Facts(facts) = &events[1] else { panic!("no facts") };
  assert_eq!(facts, &vec![json!(["rung", "rung1", "chain1", "chain1", "x = 1"])]);
}

#[test]
fn a_record_whose_log_does_not_agree_shows_what_the_record_kept_and_its_end() {
  let (path, log) = record("unlogged");
  std::fs::write(&log, "{\"entries\": 3}\n").expect("the log is written");
  let events = heard(&path);
  assert!(matches!(&events[0], Event::Opened { works: false, logged: false, .. }));
  assert!(matches!(events.last(), Some(Event::Read)));
}
