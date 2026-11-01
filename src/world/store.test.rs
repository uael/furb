//! The suite that each store of a record passes, run on each store of the crate. A store of a host passes the same
//! laws: the macro `conformance` holds them, on the function that opens the store and on the name of its record.

use std::{
  fs,
  path::{Path, PathBuf},
};

use super::{Lines, Sqlite, Store, kept, store};
use crate::{
  value::{Fault, Object},
  wire::parsed,
};

/// A record of its own for one test, which does not stand yet.
fn place(name: &str, file: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-store-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at.join(file)
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

      fn at(test: &str) -> PathBuf {
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
