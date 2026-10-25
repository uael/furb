//! The file of lines, proved where no life reaches it: the line it keeps of an entry, and the record it reads again.

use std::{fs, path::PathBuf};

use serde_json::json;

use super::{heads, kept, stamped};

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
