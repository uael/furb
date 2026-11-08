//! The suite that each store of a record passes, run on each store of the crate. A store of a host passes the same
//! laws: the macro `conformance` holds them, on the function that opens the store and on the name of its record.

use std::{fs, path::Path};

use super::{Lines, Sqlite, Store, digest, keeping, kept, marks, store, texts, unheld};
use crate::testing::Yard;
use crate::value;
use crate::{
  value::{Fault, Object},
  wire::parsed,
};
use std::process;

/// A record of its own for one test, which does not stand yet.
fn place(name: &str, file: &str) -> Yard {
  Yard::new(format!("furb-store-{name}-{}", process::id())).within(file)
}

/// The entries that the suite keeps: facts that hold text on many lines and past ASCII, numbers, nothing, and lists
/// in lists.
fn facts() -> Vec<Object> {
  [
    r#"[["stand","stand1","operator","chain1"]]"#,
    r#"[["done","clock1","time",1.5]]"#,
    r#"[["tell","rung2","chain1",["line one\nline two \u2665",-7,null,[true,[]]]]]"#,
  ]
  .into_iter()
  .map(|one| parsed(one).unwrap())
  .collect()
}

/// The entries as python shows them, which two records match by.
fn shown(entries: &[Object]) -> Vec<String> {
  entries.iter().map(|one| one.as_ref().py_repr()).collect()
}

/// A record whose store kept each fact of the suite and let it go.
fn filled(open: impl Fn(&Path) -> Result<Box<dyn Store>, Fault>, path: &Path) {
  let mut one = open(path).unwrap();
  assert!(one.entries().unwrap().is_empty());
  for entry in facts() {
    one.keep(entry.as_ref()).unwrap();
  }
}

/// The laws of the suite, each a test, on a store that `$open` opens at a path whose name ends in `$file`.
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
        assert!(kept(&path).unwrap().is_empty());
        assert!(open(&path).unwrap().entries().unwrap().is_empty());
        assert!(kept(&path).unwrap().is_empty());
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
        assert!(kept(&path).unwrap().is_empty());
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
        assert!(marks(&path).unwrap().standing.is_none());
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
          let held = marks(&path).unwrap().standing.map(|got| got.as_ref().py_repr());
          assert_eq!(held.as_deref(), Some(want), "{entry}");
        }
      }

      #[test]
      fn the_store_of_a_path_whose_name_ends_so_is_this_store() {
        let path = at("named");
        let (record, ear) = store(&path).unwrap();
        assert!(record.is_empty());
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
  let wake = waker();
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

use std::{cell::Cell, rc::Rc, task::Context};

use crate::{
  ear::{Ear, Heard, Step},
  fact::Fact,
  testing::waker,
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
  let store = Disk { lines: Lines::open(&path).unwrap(), space: Rc::clone(&space) };
  let (_, mut ear) = keeping(store).unwrap();
  let wake = waker();
  assert!(matches!(ear.resume(Heard::Born, &mut Context::from_waker(&wake)), Step::Wait(_)));
  (path, ear, space)
}

/// One pass of the ear on the keep of an entry: the message that it tells the operator, which
/// goes, or nothing.
fn pass(ear: &mut Box<dyn Ear>, entry: &Object) -> Option<String> {
  let wake = waker();
  let mut cx = Context::from_waker(&wake);
  let keep = Fact::says("keep", "", [Object::string("journal"), entry.clone()]);
  match ear.resume(Heard::Fact(keep), &mut cx) {
    Step::Wait(_) => None,
    Step::Call(call) => {
      assert_eq!(call.verb, "prompt");
      let message = call.args[1].as_ref().as_str().unwrap().to_owned();
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
  assert!(kept(&path).unwrap().is_empty());
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

/// A fact made of the engine that a digest names, with the texts given.
fn made_of(engine: &str, held: &[&str]) -> Object {
  let mut fact = serde_json::json!(["made", "chain1", "store", engine, 9, "1.0", "0.4.2"]);
  fact.as_array_mut().unwrap().extend(held.iter().map(|one| serde_json::json!(one)));
  parsed(&serde_json::json!([fact]).to_string()).unwrap()
}

#[test]
fn the_texts_of_a_segment_are_those_of_its_last_fact_made_and_must_be_the_engine_that_it_names() {
  let (source, kernel) = ("x = 1\n", "y = 2\n");
  let (named, other) = (digest(source, kernel), "0".repeat(64));
  let ours = (crate::ENGINE.to_owned(), crate::KERNEL.to_owned());
  assert_eq!(
    texts(&[made_of(&other, &[])]).unwrap(),
    ours,
    "a fact made with no texts runs this build"
  );
  let segment = [made_of(&other, &[]), made_of(&named, &[source, kernel])];
  assert_eq!(
    texts(&segment).unwrap(),
    (source.to_owned(), kernel.to_owned()),
    "the last one counts"
  );
  assert!(
    texts(&[made_of(&other, &[source, kernel])]).is_err(),
    "texts of another engine refuse it"
  );
}

#[test]
fn a_segment_names_the_engine_of_its_last_fact_made_as_unheld_when_it_holds_no_texts_of_another_engine()
 {
  let (source, kernel) = ("x = 1\n", "y = 2\n");
  let (named, other) = (digest(source, kernel), "0".repeat(64));
  assert_eq!(unheld(&[made_of(&other, &[])]), Some(other.clone()), "no texts of another engine");
  assert_eq!(unheld(&[made_of(&named, &[source, kernel])]), None, "the texts are held");
  assert_eq!(
    unheld(&[made_of(&digest(crate::ENGINE, crate::KERNEL), &[])]),
    None,
    "the engine of this build"
  );
  assert_eq!(
    unheld(&[made_of(&other, &[]), made_of(&named, &[source, kernel])]),
    None,
    "the last one counts"
  );
  assert_eq!(unheld(&[]), None, "a record of a build before the fact made");
}
