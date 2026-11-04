//! The keeper of a snapshot: whether a life said something new, and how it writes one.

#![expect(unused_qualifications, reason = "debt: a path where its name is in scope")]
#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]
#![expect(
  clippy::unchecked_time_subtraction,
  reason = "debt: a subtraction of times that can underflow"
)]

use std::{fs, path::PathBuf, time::Instant};

use super::{Keeper, aside, fits, path};
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
  let table: Table = serde_json::from_str(
    r#"{"root":"chain1","ears":[],"kept":[],"opened":{},"given":"","working":[]}"#,
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
  let room = super::free(&place("free")).expect("a disk says its free space");
  assert!(room > 0);
}

#[test]
#[cfg(unix)]
fn the_files_of_a_record_whose_name_is_no_text_keep_its_bytes() {
  use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
  };

  let mut name = place("bytes").into_os_string().into_vec();
  name.extend_from_slice(b"/record\xff.jsonl");
  let record = PathBuf::from(OsString::from_vec(name.clone()));
  let mut snapshot = name.clone();
  snapshot.extend_from_slice(b".snapshot");
  assert_eq!(path(&record).as_os_str().as_bytes(), snapshot.as_slice());
  let mut aside_name = snapshot;
  aside_name.extend_from_slice(b".tmp");
  assert_eq!(aside(&record).as_os_str().as_bytes(), aside_name.as_slice());
  let mut log = name;
  log.extend_from_slice(b".log");
  let log = PathBuf::from(OsString::from_vec(log));
  if fs::write(&log, "one\n").is_err() {
    return;
  }
  let keeper = Keeper::new(record, "build".to_owned());
  assert!(keeper.grew(), "a log that no snapshot covers holds something new");
  assert!(super::log(&keeper.record).is_some(), "the log of a name that is no text is read");
}
