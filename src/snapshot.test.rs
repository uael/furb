//! The keeper of a snapshot: when a life asks it for one.

use std::{
  fs,
  path::PathBuf,
  time::{Duration, Instant},
};

use super::{INTERVAL, Keeper, aside, fits, free, path};
use crate::engine::Table;

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

#[test]
fn a_keeper_waits_longer_after_a_snapshot_that_took_long() {
  let mut keeper = Keeper::new(place("paced").join("record.jsonl"), "build".to_owned());
  assert!(keeper.due(false), "a life with no snapshot yet is due one");
  assert!(!keeper.due(false), "a snapshot is not due again before the interval");
  assert!(keeper.due(true), "a snapshot is due now when the life asks now");
  keeper.last = Some(Instant::now() - INTERVAL);
  keeper.spent = Duration::from_secs(2);
  assert!(!keeper.due(false), "a snapshot that took 2 s waits 40 s before the next");
  keeper.last = Some(Instant::now() - 2 * INTERVAL);
  assert!(keeper.due(false));
}

#[test]
#[ignore = "a measure, which a person runs on a large record that RECORD names"]
fn measure_each_step_of_an_open_from_the_snapshot_of_a_record() {
  let record = PathBuf::from(std::env::var("RECORD").unwrap());
  let t = Instant::now();
  let bytes = fs::read(super::path(&record)).unwrap();
  let read = t.elapsed();
  let hashed = (super::record(&record).unwrap(), super::log(&record));
  let hash = t.elapsed() - read;
  let past = crate::world::logged(&record).unwrap().unwrap_or_default();
  let logged = t.elapsed() - read - hash;
  let at = bytes.iter().position(|one| *one == b'\n').unwrap();
  let sand = crate::sand::Sand::loaded(&bytes[at + 1..]).unwrap();
  let loaded = t.elapsed() - read - hash - logged;
  drop(sand);
  eprintln!(
    "snapshot {} bytes, {} entries, {} facts of the log: read {read:?}, record and log hashed {hash:?}, log read {logged:?}, session loaded {loaded:?}",
    bytes.len(),
    hashed.0.0,
    past.len()
  );
}

#[test]
fn a_snapshot_that_fails_to_land_leaves_no_part_of_it_on_the_disk() {
  let record = place("aside").join("record.jsonl");
  fs::write(&record, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n").unwrap();
  fs::write(format!("{}.log", record.display()), "{}\n").unwrap();
  // A folder in the place of the snapshot refuses the new one.
  fs::create_dir_all(path(&record).join("held")).unwrap();
  let mut keeper = Keeper::new(record.clone(), "build".to_owned());
  assert!(keeper.due(true));
  let table: Table = serde_json::from_str(
    r#"{"root":"chain1","ears":[],"kept":[],"opened":[],"given":"","working":[]}"#,
  )
  .unwrap();
  keeper.write(&table, &[0; 4096]);
  assert!(!aside(&record).exists(), "the part of the snapshot is removed");
  assert!(path(&record).is_dir());
}

#[test]
fn a_snapshot_fits_on_a_disk_that_holds_twice_its_size_free_or_that_says_nothing() {
  assert!(fits(None, 1 << 30), "a disk that says nothing is taken to hold it");
  assert!(fits(Some(12), 6));
  assert!(!fits(Some(11), 6), "the record needs room to grow beside the new snapshot");
  assert!(!fits(Some(0), 1));
}

#[test]
#[cfg(unix)]
fn the_free_space_of_a_folder_is_the_space_of_its_disk() {
  let room = free(&place("free")).expect("a disk says its free space");
  assert!(room > 0);
}
