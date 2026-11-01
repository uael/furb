//! The file of lines, proved where no life reaches it: the line it keeps of an entry, and the record it reads again.

use std::{fs, io::Write, path::PathBuf};

use serde_json::json;

use super::{Lines, heads, kept, stamped};
use crate::world::store::Store;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-lines-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at.join("record.jsonl")
}

#[test]
fn a_line_holds_the_entry_and_after_its_fact_the_time_the_store_kept_it() {
  let entry = json!([["done", "clock1", "time", 1.5]]);
  assert_eq!(
    stamped(entry, 1_760_000_000_000),
    "[[\"done\",\"clock1\",\"time\",1.5],1760000000000]\n"
  );
}

#[test]
fn a_record_reads_the_same_with_a_time_after_each_fact_and_with_none() {
  let timed = place("timed");
  let plain = place("plain");
  fs::write(&timed, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],1760000000000]\n").unwrap();
  fs::write(&plain, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n").unwrap();
  let one = kept(&timed).unwrap();
  let two = kept(&plain).unwrap();
  assert_eq!(one.len(), 1);
  assert_eq!(
    format!("{:?}", one[0].as_ref().py_repr()),
    format!("{:?}", two[0].as_ref().py_repr())
  );
}

#[test]
fn a_line_whose_word_after_the_fact_is_no_time_is_invalid() {
  let at = place("invalid");
  fs::write(&at, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],\"noon\"]\n").unwrap();
  assert!(kept(&at).is_err());
}

#[test]
fn the_heads_of_a_record_are_its_entries_of_some_kinds_in_order_and_no_other_line_is_parsed() {
  let at = place("heads");
  fs::write(
    &at,
    "[[\"made\",\"chain1\",\"store\",\"abc\",1],1]\n[[\"done\",\"bash1\",\"bash\", not json\n[[\"checkpoint\",\"chain1\",\"store\",{}],2]\n[[\"made\",\"chain1\",\"store\",\"abd\",1]]\n",
  )
  .unwrap();
  let read = heads(&at, &["made", "checkpoint"]).unwrap();
  let shown: Vec<String> = read.iter().map(|one| one.as_ref().py_repr()).collect();
  assert_eq!(
    shown,
    [
      "[['made', 'chain1', 'store', 'abc', 1]]",
      "[['checkpoint', 'chain1', 'store', {}]]",
      "[['made', 'chain1', 'store', 'abd', 1]]",
    ]
  );
}

#[test]
fn a_line_that_the_store_failed_to_write_whole_is_cut_before_the_next_line() {
  let source = place("source");
  fs::write(
    &source,
    "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n[[\"stand\",\"stand2\",\"operator\",\"chain1\"]]\n",
  )
  .unwrap();
  let entries = kept(&source).unwrap();
  let at = place("torn");
  let mut store = Lines::open(&at).unwrap();
  store.keep(entries[0].as_ref()).unwrap();
  // A write that failed on a full disk left a part of its line.
  fs::OpenOptions::new().append(true).open(&at).unwrap().write_all(b"[[\"stand\",\"sta").unwrap();
  store.torn = true;
  store.keep(entries[1].as_ref()).unwrap();
  store.sync().unwrap();
  let read = kept(&at).unwrap();
  let shown: Vec<String> = read.iter().map(|one| one.as_ref().py_repr()).collect();
  let want: Vec<String> = entries.iter().map(|one| one.as_ref().py_repr()).collect();
  assert_eq!(shown, want);
}

#[test]
fn a_record_whose_last_line_is_torn_or_not_ended_is_mended_at_the_first_keep_and_not_before() {
  let one = "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n";
  for (tail, name) in [
    ("[[\"stand\",\"sta", "torn-tail"),
    ("[[\"stand\",\"stand2\",\"operator\",\"chain1\"]]", "unended"),
  ] {
    let at = place(name);
    fs::write(&at, format!("{one}{tail}")).unwrap();
    let before = fs::read(&at).unwrap();
    let mut store = Lines::open(&at).unwrap();
    let entries = store.entries().unwrap();
    store.sync().unwrap();
    assert_eq!(fs::read(&at).unwrap(), before, "a store that keeps nothing changes nothing");
    store.keep(entries[0].as_ref()).unwrap();
    store.sync().unwrap();
    let held = kept(&at).unwrap().len();
    assert_eq!(held, if name == "torn-tail" { 2 } else { 3 });
    assert!(fs::read_to_string(&at).unwrap().ends_with('\n'));
  }
}
