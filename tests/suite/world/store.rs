//! The suite that each store of a record passes, run on each store of the crate. A store of a host
//! passes the same laws: the macro `conformance` holds them, on the function that opens the store
//! and on the name of its record.

use std::{
  env, fs,
  path::Path,
  process::{Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use sha2::{Digest as _, Sha256};

use crate::yard::Yard;
use furb::{
  value,
  value::{Fault, Object},
  wire::parsed,
  world::{Lines, Sqlite, Store, crossings, keeping, kept, stood, store, texts},
};
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str, file: &str) -> Yard {
  Yard::new(format!("furb-store-{name}-{}", process::id())).within(file)
}

/// The entries that the suite keeps: facts that hold text on many lines and past ASCII, numbers,
/// nothing, and lists in lists.
fn facts() -> Vec<Object> {
  [
    r#"[["stand","stand1","operator","chain1"]]"#,
    r#"[["done","clock1","time",1.5]]"#,
    r#"[["tell","rung2","chain1",["line one\nline two \u2665",-7,null,[true,[]]]]]"#,
  ]
  .into_iter()
  .map(|one| parsed(one).unwrap_or_else(|no| panic!("{no}")))
  .collect()
}

/// The entries as python shows them, which two records match by.
fn shown(entries: &[Object]) -> Vec<String> {
  entries.iter().map(|one| one.as_ref().py_repr()).collect()
}

/// A record whose store kept each fact of the suite and let it go.
fn filled(open: impl Fn(&Path) -> Result<Box<dyn Store>, Fault>, path: &Path) {
  let mut one = open(path).unwrap_or_else(|no| panic!("{no}"));
  assert_eq!(one.entries().unwrap_or_else(|no| panic!("{no}")), Vec::<Object>::new());
  for entry in facts() {
    one.keep(entry.as_ref()).unwrap_or_else(|no| panic!("{no}"));
  }
}

/// The laws of the suite, each a test, on a store that `$open` opens at a path whose name ends in
/// `$file`.
macro_rules! conformance {
  ($name:ident, $file:literal, $open:expr) => {
    mod $name {
      use super::*;

      fn open(path: &Path) -> Result<Box<dyn Store>, Fault> {
        $open(path).map(|one| Box::new(one) as Box<dyn Store>)
      }

      fn at(test: &str) -> Yard {
        place(&format!("{}-{test}", stringify!($name)), $file)
      }

      #[test]
      fn a_new_record_holds_no_entry() {
        let path = at("new");
        assert_eq!(kept(&path).unwrap(), Vec::<Object>::new());
        assert!(open(&path).unwrap().entries().unwrap().is_empty());
        assert_eq!(kept(&path).unwrap(), Vec::<Object>::new());
      }

      #[test]
      fn a_later_store_reads_each_entry_in_the_order_it_was_kept_and_unchanged() {
        let path = at("again");
        filled(open, &path);
        assert_eq!(shown(&open(&path).unwrap().entries().unwrap()), shown(&facts()));
        assert_eq!(shown(&kept(&path).unwrap()), shown(&facts()));
      }

      #[test]
      fn one_store_at_a_time_holds_a_record() {
        let path = at("lease");
        let first = open(&path).unwrap();
        let refused = open(&path).err().unwrap();
        assert_eq!(
          refused.to_string(),
          format!("Refused: Another process owns {}.", path.display())
        );
        drop(first);
        assert!(open(&path).is_ok());
      }

      #[test]
      fn a_read_with_no_lease_sees_each_entry_once_it_is_kept_while_a_store_holds_the_record() {
        let path = at("read");
        let mut one = open(&path).unwrap();
        one.entries().unwrap();
        for (n, entry) in facts().into_iter().enumerate() {
          one.keep(entry.as_ref()).unwrap();
          assert_eq!(shown(&kept(&path).unwrap()), shown(&facts()[..=n]));
        }
      }

      #[test]
      fn a_sync_succeeds_with_no_entry_kept_since_the_last_one() {
        let path = at("idle");
        let mut one = open(&path).unwrap();
        one.entries().unwrap();
        one.sync().unwrap();
        one.sync().unwrap();
        assert_eq!(kept(&path).unwrap(), Vec::<Object>::new());
      }

      #[test]
      fn a_later_store_reads_each_entry_kept_before_and_after_a_sync_in_order() {
        let path = at("synced");
        let (all, half) = (facts(), facts().len() / 2);
        let mut one = open(&path).unwrap();
        one.entries().unwrap();
        for entry in &all[..half] {
          one.keep(entry.as_ref()).unwrap();
        }
        one.sync().unwrap();
        assert_eq!(shown(&kept(&path).unwrap()), shown(&all[..half]));
        for entry in &all[half..] {
          one.keep(entry.as_ref()).unwrap();
        }
        one.sync().unwrap();
        drop(one);
        assert_eq!(shown(&open(&path).unwrap().entries().unwrap()), shown(&all));
      }

      #[test]
      fn the_standing_of_a_record_is_the_list_that_its_last_done_of_a_stand_gave() {
        let path = at("standing");
        let stood = |path: &Path| kept(path).unwrap().iter().rev().find_map(stood);
        assert!(stood(&path).is_none());
        let mut one = open(&path).unwrap();
        one.entries().unwrap();
        let first = "[['first'], 1]";
        let last = "[['last'], '\u{2665}', None]";
        for (entry, want) in [
          (r#"[["done","stand1","operator",[["first"],1]]]"#, first),
          (r#"[["tell","rung2","chain1",["a stand is not here"]]]"#, first),
          (r#"[["done","stand7","operator",[["last"],"\u2665",null]]]"#, last),
          (r#"[["done","stand9","operator","refused"]]"#, last),
          (r#"[["done","bash3","operator",[["not","a","stand"]]]]"#, last),
          (r#"[["stand","stand11","operator","chain1"]]"#, last),
        ] {
          one.keep(parsed(entry).unwrap().as_ref()).unwrap();
          let held = stood(&path).map(|got| got.as_ref().py_repr());
          assert_eq!(held.as_deref(), Some(want), "{entry}");
        }
      }

      #[test]
      fn the_places_of_the_crossings_of_a_record_count_each_entry_before_them() {
        let path = at("crossings");
        assert_eq!(crossings(&path).unwrap(), [0usize; 0]);
        let mut one = open(&path).unwrap();
        one.entries().unwrap();
        for entry in [
          r#"[["stand","stand1","operator","chain1"]]"#,
          r#"[["made","chain1","store","one"]]"#,
          r#"[["tell","rung2","chain1",["[[\"made\", in a text"]]]"#,
          r#"[["made","chain1","store","one"]]"#,
          r#"[["made","chain1","store","two"]]"#,
        ] {
          one.keep(parsed(entry).unwrap().as_ref()).unwrap();
        }
        assert_eq!(crossings(&path).unwrap(), [4]);
      }

      #[test]
      fn the_store_of_a_path_whose_name_ends_so_is_this_store() {
        let path = at("named");
        let (record, ear) = store(&path).unwrap();
        assert_eq!(record, Vec::<Object>::new());
        drop(ear);
        filled(open, &path);
        let (record, _ear) = store(&path).unwrap();
        assert_eq!(shown(&record), shown(&facts()));
      }
    }
  };
}

conformance!(lines, "record.jsonl", Lines::open);
conformance!(sqlite, "record.sqlite", Sqlite::open);

#[test]
fn a_line_holds_the_entry_and_after_its_fact_the_time_the_store_kept_it() {
  let at = place("stamped", "record.jsonl");
  let mut store = Lines::open(&at).unwrap();
  let before = jiff::Timestamp::now().as_millisecond();
  store.keep(facts()[1].as_ref()).unwrap();
  store.sync().unwrap();
  let after = jiff::Timestamp::now().as_millisecond();
  let line: serde_json::Value = serde_json::from_str(&fs::read_to_string(&at).unwrap()).unwrap();
  assert_eq!(line[0], serde_json::json!(["done", "clock1", "time", 1.5]));
  let time = line[1].as_i64().unwrap();
  assert!((before..=after).contains(&time), "{time} in {before}..={after}");
  assert_eq!(line.as_array().map(Vec::len), Some(2));
}

#[test]
fn a_record_reads_the_same_with_a_time_after_each_fact_and_with_none() {
  let timed = place("timed", "record.jsonl");
  let plain = place("plain", "record.jsonl");
  fs::write(&timed, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],1760000000000]\n").unwrap();
  fs::write(&plain, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n").unwrap();
  let one = kept(&timed).unwrap();
  assert_eq!(one.len(), 1);
  assert_eq!(shown(&one), shown(&kept(&plain).unwrap()));
}

#[test]
fn a_line_whose_word_after_the_fact_is_no_time_is_invalid() {
  let at = place("invalid", "record.jsonl");
  fs::write(&at, "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],\"noon\"]\n").unwrap();
  assert!(kept(&at).is_err());
}

#[test]
fn a_record_is_read_whole_from_its_start_past_a_blank_line_to_a_last_line_with_no_end() {
  let at = place("whole", "record.jsonl");
  let text =
    "[[\"stand\",\"stand1\",\"operator\",\"chain1\"],1]\n\n[[\"done\",\"clock1\",\"time\",1.5]]";
  fs::write(&at, text).unwrap();
  assert_eq!(kept(&at).unwrap().len(), 2);
  assert_eq!(
    kept(place("gone", "record.jsonl")).unwrap().len(),
    0,
    "a record that stands not holds none"
  );
}

#[test]
fn a_record_whose_last_line_is_torn_or_not_ended_is_mended_at_the_first_keep_and_not_before() {
  let one = "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n";
  for (tail, name, held) in [
    ("[[\"stand\",\"sta", "torn-tail", 2),
    ("[[\"stand\",\"stand2\",\"operator\",\"chain1\"]]", "unended", 3),
  ] {
    let at = place(name, "record.jsonl");
    fs::write(&at, format!("{one}{tail}")).unwrap();
    let before = fs::read(&at).unwrap();
    let mut store = Lines::open(&at).unwrap();
    let entries = store.entries().unwrap();
    store.sync().unwrap();
    assert_eq!(fs::read(&at).unwrap(), before, "a store that keeps nothing changes nothing");
    store.keep(entries[0].as_ref()).unwrap();
    store.sync().unwrap();
    drop(store);
    assert_eq!(kept(&at).unwrap().len(), held, "{name}");
    assert!(fs::read_to_string(&at).unwrap().ends_with('\n'));
  }
}

#[test]
fn the_place_of_a_crossing_counts_the_entries_that_a_read_gives_past_a_made_line_that_a_stop_tore()
{
  let at = place("torn-crossings", "record.jsonl");
  let lines = [
    r#"[["made","chain1","store","one"]]"#,
    r#"[["stand","stand1","operator","chain1"]]"#,
    r#"[["made","chain1","store","two"]]"#,
    r#"[["made","chain1","sto"#,
  ];
  fs::write(&at, lines.join("\n")).unwrap();
  let read = kept(&at).unwrap();
  assert_eq!(crossings(&at).unwrap(), [2], "{:?}", shown(&read));
  assert_eq!(shown(&read[2..]), ["[['made', 'chain1', 'store', 'two']]"]);
  // A failed write cuts its torn line before the next one, so only the last line of a record
  // stands torn, and a torn line before another is no record that a life opens.
  fs::write(&at, [lines[0], lines[3], lines[2]].join("\n")).unwrap();
  assert!(kept(&at).is_err());
}

#[test]
fn a_record_named_as_a_database_is_one_of_sqlite_and_any_other_is_a_file_of_lines() {
  for (file, database) in
    [("r.sqlite", true), ("r.SQLITE3", true), ("r.db", true), ("r.jsonl", false), ("r", false)]
  {
    let path = place(&format!("kind-{file}"), file);
    let (_, ear) = store(&path).unwrap();
    drop(ear);
    let head = fs::read(&path).unwrap();
    assert_eq!(head.starts_with(b"SQLite format 3\0"), database, "{file}");
  }
}

#[test]
fn a_row_of_the_database_that_holds_no_fact_is_refused_with_its_number() {
  let path = place("row", "record.sqlite");
  filled(|at| Sqlite::open(at).map(|one| Box::new(one) as Box<dyn Store>), &path);
  let db = rusqlite::Connection::open(&path).unwrap();
  db.execute("INSERT INTO entries (entry, at) VALUES ('[[\"x\"]]', 0)", []).unwrap();
  drop(db);
  assert_eq!(
    kept(&path).err().unwrap().to_string(),
    format!("Refused: Invalid record entry at row 4 in {}.", path.display())
  );
}

#[test]
fn a_writing_that_a_record_of_an_older_build_holds_as_a_question_is_read_as_the_writing_fact() {
  let older = place("writing-older", "record.jsonl");
  let newer = place("writing-newer", "record.jsonl");
  for (path, entry) in [
    (&older, r#"[["writing","writing1","provider","chain1","rung2","x = 1\n",6]]"#),
    (&newer, r#"[["writing","rung2","provider","x = 1\n",6]]"#),
  ] {
    let mut one = Lines::open(path).unwrap();
    one.keep(parsed(entry).unwrap().as_ref()).unwrap();
  }
  assert_eq!(shown(&kept(&older).unwrap()), shown(&kept(&newer).unwrap()));
  let (record, _ear) = store(&older).unwrap();
  assert_eq!(shown(&record), shown(&kept(&newer).unwrap()));
}

/// A store whose disk is full: it refuses each entry.
struct Refusing;

impl Store for Refusing {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    Ok(Vec::new())
  }

  fn keep(&mut self, _: value::ObjectRef<'_>) -> Result<(), Fault> {
    Err(Fault::refused("No space left on device (os error 28): the record"))
  }

  fn sync(&mut self) -> Result<(), Fault> {
    Err(Fault::refused("No space left on device (os error 28): the record"))
  }
}

#[test]
fn a_stop_message_that_failed_to_reach_the_operator_is_said_again_at_the_next_pass() {
  let (_, mut ear) = keeping(Refusing).unwrap();
  let wake = Waker::noop().clone();
  let mut cx = Context::from_waker(&wake);
  let keep = || Fact::says("keep", "", [Object::string("journal"), facts()[0].clone()]);
  assert!(matches!(ear.resume(Heard::Born, &mut cx), Step::Wait(_)));
  let mut asked = |ear: &mut Box<dyn Ear>, answer: Result<Object, Fault>| {
    let Step::Call(call) = ear.resume(Heard::Fact(keep()), &mut cx) else {
      panic!("a pass that finds the record stopped tells the operator")
    };
    assert_eq!(call.verb, "prompt");
    let message = call.args[1].as_ref().as_str().unwrap().to_owned();
    assert!(message.contains("cannot be written: No space left on device"), "{message}");
    // The ear takes what the verb gave, and waits for the next fact.
    assert!(matches!(ear.resume(Heard::Answer(answer), &mut cx), Step::Wait(_)));
    message
  };
  // The first send fails, so the pass after it says the same message again.
  let first = asked(&mut ear, Err(Fault::refused("the operator is gone")));
  let second = asked(&mut ear, Ok(Object::string("prompt1")));
  assert_eq!(first, second);
  // The second send went, so the next pass says nothing.
  assert!(matches!(ear.resume(Heard::Fact(keep()), &mut cx), Step::Wait(_)));
}

use std::{
  cell::Cell,
  rc::Rc,
  task::{Context, Waker},
};

use furb::{
  ear::{Ear, Heard, Step},
  fact::Fact,
};

/// A file of lines whose disk the test fills and empties: a keep and a sync fail while it is full,
/// and its room is what the test gives.
struct Disk {
  lines: Lines,
  space: Rc<Space>,
}

/// What the test makes of the disk: whether it is full, and the bytes that it still takes.
struct Space {
  full: Cell<bool>,
  free: Cell<u64>,
}

impl Store for Disk {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    self.lines.entries()
  }

  fn keep(&mut self, entry: value::ObjectRef<'_>) -> Result<(), Fault> {
    if self.space.full.get() {
      return Err(Fault::refused("No space left on device (os error 28): the record"));
    }
    self.lines.keep(entry)
  }

  fn sync(&mut self) -> Result<(), Fault> {
    if self.space.full.get() {
      return Err(Fault::refused("No space left on device (os error 28): the record"));
    }
    self.lines.sync()
  }

  fn room(&self) -> Option<(&Path, u64)> {
    Some((Path::new("/the/folder"), self.space.free.get()))
  }
}

/// The record at a path of its own on a disk with room, the ear that keeps it, born, and the disk.
fn disk(name: &str) -> (Yard, Box<dyn Ear>, Rc<Space>) {
  let path = place(name, "record.jsonl");
  let space = Rc::new(Space { full: Cell::new(false), free: Cell::new(u64::MAX) });
  let store = Disk {
    lines: Lines::open(&path).unwrap_or_else(|no| panic!("{no}")),
    space: Rc::clone(&space),
  };
  let (_, mut ear) = keeping(store).unwrap_or_else(|no| panic!("{no}"));
  let wake = Waker::noop().clone();
  assert!(matches!(ear.resume(Heard::Born, &mut Context::from_waker(&wake)), Step::Wait(_)));
  (path, ear, space)
}

/// One pass of the ear on the keep of an entry: the message that it tells the operator, which
/// goes, or nothing.
fn pass(ear: &mut Box<dyn Ear>, entry: &Object) -> Option<String> {
  let wake = Waker::noop().clone();
  let mut cx = Context::from_waker(&wake);
  let keep = Fact::says("keep", "", [Object::string("journal"), entry.clone()]);
  match ear.resume(Heard::Fact(keep), &mut cx) {
    Step::Wait(_) => None,
    Step::Call(call) => {
      assert_eq!(call.verb, "prompt");
      let message = call.args[1].as_ref().as_str().unwrap_or_else(|| panic!("nothing")).to_owned();
      assert!(matches!(
        ear.resume(Heard::Answer(Ok(Object::string("prompt1"))), &mut cx),
        Step::Wait(_)
      ));
      Some(message)
    }
    _ => panic!("a pass of the ear keeps, tells or waits"),
  }
}

#[test]
fn a_record_on_a_full_disk_tells_the_operator_once_and_takes_each_entry_in_order_once_the_disk_has_room()
 {
  let (path, mut ear, space) = disk("full");
  let entries = facts();
  space.full.set(true);
  let first = pass(&mut ear, &entries[0]).expect("the first keep that fails tells the operator");
  assert!(first.contains("cannot be written: No space left on device"), "{first}");
  assert_eq!(pass(&mut ear, &entries[1]), None, "the operator knows already");
  assert_eq!(kept(&path).unwrap(), Vec::<Object>::new());
  space.full.set(false);
  // The end of a run keeps each entry that waits and syncs them, which ends the fault.
  ear.ran().unwrap();
  let again = pass(&mut ear, &entries[2])
    .expect("the record that takes the entries again tells the operator");
  assert!(again.contains("is written again"), "{again}");
  assert_eq!(shown(&kept(&path).unwrap()), shown(&entries), "each entry, in order, once");
}

#[test]
fn a_record_on_a_disk_that_is_almost_full_tells_the_operator_once_with_its_folder_and_the_space_left()
 {
  let (path, mut ear, space) = disk("low");
  let entries = facts();
  space.free.set(5_000_000);
  let low = pass(&mut ear, &entries[0]).expect("a disk under the bound tells the operator");
  assert!(low.contains("almost full: /the/folder has 5 MB left"), "{low}");
  assert_eq!(pass(&mut ear, &entries[1]), None, "the operator knows already");
  space.free.set(3 << 30);
  assert_eq!(pass(&mut ear, &entries[2]), None, "a disk with room again tells nothing");
  space.free.set(5_000_000);
  let again =
    pass(&mut ear, &entries[0]).expect("a disk that falls under the bound again tells again");
  assert_eq!(again, low);
  assert_eq!(
    kept(&path).unwrap().len(),
    4,
    "the disk was never full, so each entry landed at once"
  );
}

#[test]
fn a_record_whose_last_line_a_stop_tore_tells_the_operator_once_the_bytes_that_its_open_cut() {
  let path = place("torn-told", "record.jsonl");
  let entries = facts();
  Lines::open(&path).unwrap().keep(entries[0].as_ref()).unwrap();
  let mut text = fs::read(&path).unwrap();
  text.extend_from_slice(b"[[\"stand\",\"sta");
  fs::write(&path, text).unwrap();
  let (record, mut ear) = keeping(Lines::open(&path).unwrap()).unwrap();
  assert_eq!(record.len(), 1, "the open holds each line written whole");
  let wake = Waker::noop().clone();
  assert!(matches!(ear.resume(Heard::Born, &mut Context::from_waker(&wake)), Step::Wait(_)));
  let told = pass(&mut ear, &entries[1]).expect("the first pass tells the bytes that the open cut");
  assert!(told.contains("it cut 14 bytes"), "{told}");
  assert_eq!(pass(&mut ear, &entries[2]), None, "the operator knows already");
  assert_eq!(kept(&path).unwrap().len(), 3, "the keep cut the torn line and kept each entry");
}

/// A fact made of the engine that a digest names, with the texts given.
fn made_of(engine: &str, held: &[&str]) -> Object {
  let mut fact = serde_json::json!(["made", "chain1", "store", engine, 9, "1.0", "0.4.2"]);
  fact
    .as_array_mut()
    .unwrap_or_else(|| panic!("nothing"))
    .extend(held.iter().map(|one| serde_json::json!(one)));
  parsed(&serde_json::json!([fact]).to_string()).unwrap_or_else(|no| panic!("{no}"))
}

#[test]
fn the_texts_of_a_segment_are_those_of_its_last_fact_made_and_must_be_the_engine_that_it_names() {
  let (source, kernel, checkpointer) = ("x = 1\n", "y = 2\n", "z = 3\n");
  let other = "0".repeat(64);
  let ours = (furb::ENGINE.to_owned(), furb::KERNEL.to_owned(), furb::CHECKPOINTER.to_owned());
  assert_eq!(
    texts(&[made_of(&other, &[])]).unwrap(),
    ours,
    "a fact made with no texts runs this build"
  );
  let all = [source, kernel, checkpointer];
  assert_eq!(
    texts(&[made_of(&other, &[]), made_of(&digest(&all), &all)]).unwrap(),
    (source.to_owned(), kernel.to_owned(), checkpointer.to_owned()),
    "the last one counts, and it names the digest of the three texts that it holds"
  );
  assert!(texts(&[made_of(&other, &all)]).is_err(), "texts of another engine refuse it");
}

/// The digest of the texts of an engine, of its Kernel and of its checkpointer that a fact made
/// names: the sha256 of each text, each followed by a byte zero.
fn digest(texts: &[&str]) -> String {
  let mut hash = Sha256::new();
  for one in texts {
    hash.update(one.as_bytes());
    hash.update([0]);
  }
  format!("{:x}", hash.finalize())
}

#[test]
fn a_store_says_the_file_that_it_could_not_use_before_the_text_of_the_system() {
  let at = Yard::new(format!("furb-world-unusable-{}", process::id()));
  fs::write(at.join("sessions"), "").unwrap();
  let path = at.join("sessions").join("record.jsonl");
  let refused = store(&path).err().unwrap().to_string();
  // The rest is the text of the system, which differs on each system.
  let said = format!("Refused: Could not use {}: ", path.with_extension("jsonl.lock").display());
  assert!(refused.starts_with(&said), "{refused}");
}

#[test]
fn one_store_at_a_time_holds_a_record_and_a_read_with_no_lease_reads_it_all_the_same() {
  let at = Yard::new(format!("furb-world-lease-{}", process::id()));
  let path = at.join("record.jsonl");
  let (record, ear) = store(&path).unwrap();
  assert_eq!(record, Vec::<Object>::new());
  let refused = store(&path).err().unwrap();
  assert_eq!(refused.to_string(), format!("Refused: Another process owns {}.", path.display()));
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]\n").unwrap();
  assert_eq!(kept(&path).unwrap().len(), 1);
  drop(ear);
  let (record, _ear) = store(&path).unwrap();
  assert_eq!(record.len(), 1);
}

#[test]
fn the_store_reads_past_a_torn_last_line_and_a_whole_one_that_no_keep_mends_and_refuses_a_damaged_line()
 {
  let at = Yard::new(format!("furb-world-mend-{}", process::id()));
  let path = at.join("record.jsonl");
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]\n[\"partial\"").unwrap();
  let (record, ear) = store(&path).unwrap();
  drop(ear);
  assert_eq!(record.len(), 1);
  assert_eq!(fs::read_to_string(&path).unwrap(), "[[\"done\",\"x\",\"world\",null]]\n[\"partial\"");
  fs::write(&path, "[[\"done\",\"x\",\"world\",null]]").unwrap();
  let (record, ear) = store(&path).unwrap();
  drop(ear);
  assert_eq!(record.len(), 1);
  assert_eq!(fs::read_to_string(&path).unwrap(), "[[\"done\",\"x\",\"world\",null]]");
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
  assert_eq!(kept(&path).unwrap(), Vec::<Object>::new());
  assert_eq!(fs::read_to_string(&path).unwrap(), "[\"torn\"");
}

#[test]
fn two_holders_never_own_the_record_at_once_while_programs_start_and_the_record_moves_away() {
  let at = Yard::new(format!("furb-world-race-{}", process::id()));
  let path = at.join("record.jsonl");
  // Programs start all the while, from other threads of this process, since that is when macOS can
  // give for a renamed path the file that a rename moved away.
  let over = Arc::new(AtomicBool::new(false));
  let starters: Vec<_> = (0..4)
    .map(|_| {
      let over = Arc::clone(&over);
      thread::spawn(move || {
        let program = env::current_exe().unwrap();
        while !over.load(Ordering::Relaxed) {
          drop(
            Command::new(&program)
              .arg("--help")
              .stdout(Stdio::piped())
              .stderr(Stdio::piped())
              .output(),
          );
        }
      })
    })
    .collect();
  // Each holder makes a file that one alone can make while it owns the record, and two move the
  // record away before they let it go, each to a name of its own, as the delete of a session does,
  // which leaves the lock file.
  let holders: Vec<_> = (0..8)
    .map(|n| {
      let (path, at) = (path.clone(), at.to_path_buf());
      thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut taken = 0;
        while taken < 50 {
          assert!(Instant::now() < deadline, "holder {n} took the lease {taken} times");
          let Ok((_, ear)) = store(&path) else { continue };
          let owner = at.join("owner");
          fs::OpenOptions::new().write(true).create_new(true).open(&owner).expect("one owner");
          taken += 1;
          fs::remove_file(&owner).unwrap();
          if n < 2 {
            fs::rename(&path, at.join(format!("moved.{n}.{taken}"))).unwrap();
          }
          drop(ear);
        }
      })
    })
    .collect();
  let held = holders.into_iter().map(thread::JoinHandle::join).collect::<Vec<_>>();
  over.store(true, Ordering::Relaxed);
  for one in starters {
    one.join().unwrap();
  }
  for one in held {
    one.unwrap();
  }
}
