//! The file of lines, proved where no life reaches it: the line it keeps of an entry, and the record it reads again.

use std::{fs, io::Write};

use serde_json::json;

use super::{HEAD, Lines, heads, last, stamped};
use crate::testing::Yard;
use crate::value::Object;
use crate::world::{checkpoints, kept, store::Store};
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-lines-{name}-{}", process::id())).within("record.jsonl")
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

/// The lines of a record with entries before its last checkpoint, one of which does not parse,
/// and the byte where that checkpoint starts.
fn checkpointed() -> (String, u64) {
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
fn the_last_entries_of_a_record_start_at_its_last_checkpoint_and_no_line_before_it_is_parsed() {
  let at = place("last");
  let (text, start) = checkpointed();
  fs::write(&at, &text).unwrap();
  let mut passed = Vec::new();
  let (from, entries, _) = last(&at, &mut |byte| passed.push(byte)).unwrap();
  assert_eq!(from, start);
  assert_eq!(
    shown(&entries),
    [
      "[['checkpoint', 'chain1', 'store', {'schema': '2.0'}]]",
      "[['made', 'chain1', 'store', 'abd', 1]]",
      "[['stand', 'stand4', 'operator', 'chain1']]",
    ]
  );
  // The read tells the byte after each line that it parsed: the three lines from the checkpoint on.
  assert_eq!(passed.len(), 3);
  assert!(passed.iter().all(|byte| *byte > start), "{passed:?} against {start}");
  assert_eq!(passed.last(), Some(&(text.len() as u64)));
}

#[test]
fn a_record_with_no_checkpoint_or_with_one_as_its_first_line_is_read_from_its_start() {
  let none = place("none");
  fs::write(
    &none,
    "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],1]\n[[\"done\",\"clock1\",\"time\",1.5]]",
  )
  .unwrap();
  let (from, entries, _) = last(&none, &mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (0, 2));
  let first = place("first");
  fs::write(
    &first,
    "[[\"checkpoint\",\"chain1\",\"store\",{}]]\n[[\"done\",\"clock1\",\"time\",1.5]]\n",
  )
  .unwrap();
  let (from, entries, _) = last(&first, &mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (0, 2));
  let (from, entries, _) = last(&place("gone"), &mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (0, 0), "a record that stands not holds none");
}

#[test]
fn a_checkpoint_that_a_stop_tore_as_it_was_written_is_no_checkpoint_and_goes_at_the_first_keep() {
  let at = place("torn-checkpoint");
  let (text, start) = checkpointed();
  fs::write(&at, format!("{text}[[\"checkpoint\",\"chain1\",\"store\",{{\"sche")).unwrap();
  let mut store = Lines::open(&at).unwrap();
  let (from, entries) = store.last(&mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (start, 3));
  store.keep(entries[2].as_ref()).unwrap();
  assert_eq!(
    fs::read_to_string(&at).unwrap().len(),
    text.len() + "[[\"stand\",\"stand4\",\"operator\",\"chain1\"],1760000000000]\n".len()
  );
  assert_eq!(checkpoints(&at).unwrap(), [2, 4], "the torn line is gone");
  fs::write(&at, format!("{text}[[\"checkpoint\",\"chain1\",\"store\",{{\"sche")).unwrap();
  assert_eq!(checkpoints(&at).unwrap(), [2, 4], "and it was no checkpoint before");
  // A whole checkpoint that no end of line follows is the last one.
  let whole = place("unended-checkpoint");
  fs::write(&whole, format!("{text}[[\"checkpoint\",\"chain1\",\"store\",{{}}]]")).unwrap();
  let (from, entries, _) = last(&whole, &mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (text.len() as u64, 1));
  assert_eq!(checkpoints(&whole).unwrap(), [2, 4, 7]);
}

#[test]
fn the_last_checkpoint_is_found_when_its_head_stands_across_two_blocks_of_the_read() {
  let at = place("blocks");
  let before = "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],1]\n";
  let checkpoint = "[[\"checkpoint\",\"chain1\",\"store\",{}],2]\n";
  // The read takes blocks of 1 MiB from the end of the file, so the checkpoint starts 5 bytes
  // before the last of them.
  let (open, close) = ("[[\"tell\",\"rung2\",\"chain1\",[\"", "\"]],3]\n");
  let filler = "x".repeat((1 << 20) + 5 - checkpoint.len() - open.len() - close.len());
  fs::write(&at, format!("{before}{checkpoint}{open}{filler}{close}")).unwrap();
  let (from, entries, _) = last(&at, &mut |_| {}).unwrap();
  assert_eq!((from, entries.len()), (before.len() as u64, 2));
}

#[test]
fn the_heads_of_a_record_are_the_start_of_each_line_before_a_byte_and_only_a_wanted_line_is_parsed()
{
  let at = place("heads");
  let (text, start) = checkpointed();
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
