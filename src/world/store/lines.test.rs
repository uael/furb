//! The file of lines, proved where no life reaches it: the line it keeps of an entry, and the record it reads again.

use std::{fs, io::Write};

use serde_json::json;

use super::{Lines, backwards, heads, stamped};
use crate::world::{kept, store::Store};
use crate::{testing::Yard, value::Object};

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-lines-{name}-{}", std::process::id())).within("record.jsonl")
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
  let starts = [r#"[["made","#.to_owned(), r#"[["checkpoint","#.to_owned()];
  let read = heads(&at, &starts).unwrap();
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

/// A record whose lines cross every size of chunk: heads of several kinds, a line that does not
/// parse, a line much longer than the chunks, a letter past ASCII, and a last line that no end of
/// line follows.
fn crossing(name: &str) -> (Yard, String) {
  let at = place(name);
  let long = "x".repeat(300);
  let text = format!(
    "[[\"done\",\"stand1\",\"operator\",[\"wanted-1\"]],1]\n\
     [[\"tell\",\"rung2\",\"chain1\",[\"{long}\"]],2]\n\
     [[\"done\",\"stand2\",\"operator\",[\"other \u{2665}\"]],3]\n\
     [[\"done\",\"stand3\", not json\n\
     \n\
     [[\"done\",\"stand4\",\"operator\",[\"wanted-4\"]]]\n\
     [[\"done\",\"stand5\",\"operator\",[\"wanted-\"",
  );
  fs::write(&at, &text).unwrap();
  (at, text)
}

#[test]
fn the_last_entry_taken_from_the_end_is_the_one_taken_from_the_head_at_every_size_of_chunk() {
  let (at, _) = crossing("backwards");
  let starts = [r#"[["done","stand"#.to_owned()];
  let pick = |entry: Object| {
    let shown = entry.as_ref().py_repr();
    shown.contains("wanted").then_some(shown)
  };
  let head = heads(&at, &starts).unwrap().into_iter().rev().find_map(pick);
  assert_eq!(head.as_deref(), Some("[['done', 'stand4', 'operator', ['wanted-4']]]"));
  for chunk in (1..=64).chain([1 << 20]) {
    assert_eq!(backwards(&at, &starts, chunk, pick).unwrap(), head, "a chunk of {chunk}");
  }
  let never = |_: Object| None::<()>;
  assert_eq!(backwards(&at, &starts, 8, never).unwrap(), None);
  let first = |entry: Object| entry.as_ref().py_repr().contains("wanted-1").then_some(());
  assert_eq!(backwards(&at, &starts, 8, first).unwrap(), Some(()));
}

#[test]
fn a_read_from_the_end_stops_at_the_entry_it_takes_and_a_record_that_stands_not_holds_none() {
  let (at, _) = crossing("stops");
  let starts = [r#"[["done","stand"#.to_owned()];
  let mut seen = 0;
  let found = backwards(&at, &starts, 8, |entry: Object| {
    seen += 1;
    Some(entry.as_ref().py_repr())
  })
  .unwrap();
  assert_eq!(found.as_deref(), Some("[['done', 'stand4', 'operator', ['wanted-4']]]"));
  assert_eq!(seen, 1);
  let gone = place("gone");
  assert_eq!(backwards(&gone, &starts, 8, |_: Object| Some(())).unwrap(), None);
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
