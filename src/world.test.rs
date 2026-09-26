//! The parts of the ears of the World that stand apart from a life: how a path resolves, how the store holds and
//! mends its record, and how the stream of a command is read whole. The tests of the engine drive each ear in a life.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::{bash::whole, kept, resolved, store};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-world-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

#[test]
fn a_path_resolves_against_its_directory_with_every_dot_read_and_an_absolute_path_stands_as_it_is()
{
  let base = std::env::temp_dir().join("furb").join("w");
  assert_eq!(resolved(&base, "a/../b/./c"), base.join("b").join("c"));
  assert_eq!(resolved(&base, "../x"), std::env::temp_dir().join("furb").join("x"));
  let elsewhere = std::env::temp_dir().join("elsewhere");
  assert_eq!(resolved(&base, &elsewhere.display().to_string()), elsewhere);
  assert_eq!(resolved(Path::new(&base), ""), base);
}

#[test]
fn one_store_at_a_time_holds_a_record_and_a_read_with_no_lease_reads_it_all_the_same() {
  let at = place("lease");
  let path = at.join("record.jsonl");
  let (record, ear) = store(&path).unwrap();
  assert!(record.is_empty());
  let refused = store(&path).err().unwrap();
  assert_eq!(refused.to_string(), format!("Refused: Another process owns {}.", path.display()));
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]\n").unwrap();
  assert_eq!(kept(&path).unwrap().len(), 1);
  drop(ear);
  let (record, _ear) = store(&path).unwrap();
  assert_eq!(record.len(), 1);
}

#[test]
fn the_store_removes_a_torn_last_line_ends_a_whole_one_and_refuses_a_damaged_line() {
  let at = place("mend");
  let path = at.join("record.jsonl");
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]\n[\"partial\"").unwrap();
  let (record, ear) = store(&path).unwrap();
  drop(ear);
  assert_eq!(record.len(), 1);
  assert_eq!(fs::read_to_string(&path).unwrap(), "[[\"done\",\"x\",\"world\",null]]\n");
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]").unwrap();
  let (record, ear) = store(&path).unwrap();
  drop(ear);
  assert_eq!(record.len(), 1);
  assert_eq!(fs::read_to_string(&path).unwrap(), "[[\"done\",\"x\",\"world\",null]]\n");
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]\nbroken\n").unwrap();
  let refused = store(&path).err().unwrap();
  let at = "[[\"done\",\"x\",\"world\",null]]\n".len();
  assert_eq!(
    refused.to_string(),
    format!("Refused: Invalid record entry at byte {at} in {}.", path.display())
  );
  // An entry is one fact, so an entry that holds a fact and an answer is no entry.
  fs::write(&path, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],[[],\"/w\",\"m/low\"]]\n")
    .unwrap();
  assert_eq!(
    store(&path).err().unwrap().to_string(),
    format!("Refused: Invalid record entry at byte 0 in {}.", path.display())
  );
  fs::write(&path, "[\"torn\"").unwrap();
  assert!(kept(&path).unwrap().is_empty());
  assert_eq!(fs::read_to_string(&path).unwrap(), "[\"torn\"");
}

#[test]
fn a_stream_is_read_whole_in_utf8_and_a_character_cut_at_a_read_waits_for_the_next() {
  let heart = "\u{2665}".as_bytes();
  let mut held = [b"a".as_slice(), &heart[..2]].concat();
  assert_eq!(whole(&mut held, false), "a");
  assert_eq!(held, &heart[..2]);
  held.extend_from_slice(&heart[2..]);
  assert_eq!(whole(&mut held, false), "\u{2665}");
  assert!(held.is_empty());
  let mut bad = vec![b'x', 0xff, b'y'];
  assert_eq!(whole(&mut bad, false), "x\u{fffd}y");
  let mut cut = heart[..2].to_vec();
  assert_eq!(whole(&mut cut, true), "\u{fffd}");
  assert!(cut.is_empty());
}
