//! The file of lines, where no door reaches it: the read that a store takes from the open of a
//! life, which only that open hands over; the heads of the lines, whose law is what a read parses;
//! and a line that a write left torn, which only a write that failed midway leaves. The suite of
//! the stores proves the rest.

use std::{fs, io::Write};

use super::{HEAD, Last, Lines, heads, last};
use crate::testing::Yard;
use crate::value::Object;
use crate::world::{kept, store::Store};
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-lines-{name}-{}", process::id())).within("record.jsonl")
}

/// The lines of a record with entries before a byte, one of which does not parse, and that byte.
fn parted() -> (String, u64) {
  let before = "[[\"made\",\"chain1\",\"store\",\"abc\",1],1]\n\
                [[\"done\",\"bash1\",\"bash\", not json\n\
                [[\"checkpoint\",\"chain1\",\"store\",{\"schema\":\"1.0\"}],2]\n\
                \n\
                [[\"tell\",\"rung2\",\"chain1\",[\"[[\\\"checkpoint\\\", in a text\"]],3]\n";
  let last = "[[\"checkpoint\",\"chain1\",\"store\",{\"schema\":\"2.0\"}],4]\n\
              [[\"made\",\"chain1\",\"store\",\"abd\",1]]\n\
              \n\
              [[\"stand\",\"stand4\",\"operator\",\"chain1\"],5]\n";
  (format!("{before}{last}"), before.len() as u64)
}

/// The entries that a read gave, as python shows them.
fn shown(entries: &[Object]) -> Vec<String> {
  entries.iter().map(|one| one.as_ref().py_repr()).collect()
}

#[test]
fn a_store_takes_the_entries_of_a_read_with_no_lease_until_the_record_changes() {
  let at = place("taken");
  let text = "[[\"made\",\"chain1\",\"store\",\"abd\",1]]\n\n[[\"stand\",\"stand4\",\"operator\",\"chain1\"],5]\n";
  // The last line is whole and not ended, which the store that takes the read mends at its keep.
  let unended = text.trim_end();
  fs::write(&at, unended).unwrap();
  let read = last(&at, &mut |_| {}).unwrap();
  let again = last(&at, &mut |_| {}).unwrap();
  let mut passed = Vec::new();
  let mut store = Lines::open(&at).unwrap();
  let Last { entries, .. } = store.last(Some(read), &mut |byte| passed.push(byte)).unwrap();
  assert_eq!((entries.len(), passed.len()), (2, 0), "the store reads no line");
  store.keep(entries[1].as_ref()).unwrap();
  let grown = fs::read_to_string(&at).unwrap();
  assert_eq!(grown.strip_prefix(unended).map(|rest| rest.starts_with("\n[[")), Some(true));
  // The record grew after the second read, so the store reads it again.
  let Last { entries, .. } = store.last(Some(again), &mut |byte| passed.push(byte)).unwrap();
  assert_eq!((entries.len(), passed.len()), (3, 3));
}

#[test]
fn the_heads_of_a_record_are_the_start_of_each_line_before_a_byte_and_only_a_wanted_line_is_parsed()
{
  let at = place("heads");
  let (text, start) = parted();
  let long = format!("[[\"tell\",\"rung9\",\"chain1\",[\"{}\"]],6]", "y".repeat(2 * HEAD));
  fs::write(&at, format!("{text}{long}")).unwrap();
  let read = |to: u64, kind: &str| {
    let (mut starts, mut parsed) = (Vec::new(), Vec::new());
    let mut wanted = |line: &[u8]| {
      starts.push(String::from_utf8_lossy(line).into_owned());
      line.starts_with(format!("[[\"{kind}\"").as_bytes())
    };
    heads(&at, to, &mut wanted, &mut |one| parsed.push(one)).unwrap();
    (starts, shown(&parsed))
  };
  let (starts, parsed) = read(start, "tell");
  let lines: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
  assert_eq!(starts, lines[..4], "a blank line is none, and a line that does not parse still is");
  assert_eq!(parsed, ["[['tell', 'rung2', 'chain1', ['[[\"checkpoint\", in a text']]]"]);
  let (starts, parsed) = read(u64::MAX, "tell");
  assert_eq!(starts.len(), 8);
  assert_eq!(
    starts[7],
    long[..HEAD],
    "a long line gives its start alone, and a last line needs no end"
  );
  assert_eq!(parsed.len(), 2, "and a wanted line is parsed whole");
  assert_eq!(read(u64::MAX, "done").1, [""; 0], "a wanted line that does not parse gives no entry");
  let mut none = 0;
  heads(&place("gone"), u64::MAX, &mut |_| true, &mut |_| none += 1).unwrap();
  assert_eq!(none, 0, "a record that stands not holds none");
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
