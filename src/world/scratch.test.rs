//! The scratch directory, proved where no life reaches it: its place, the paths of its scheme, and the files that a
//! record makes again in it. The tests of the World drive it in a life.

use std::{fs, path::PathBuf};

use super::{SCRATCH, Scratch, WHOLE};
use crate::value::{Object, Text};

/// A directory of its own for one test, which does not stand yet.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-scratch-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  at
}

/// One entry of a record: the done of an act with what it came to, as the World said it.
fn done(about: &str, value: Object) -> Object {
  let fact = [Object::string("done"), Object::string(about), Object::string("files"), value];
  Object::list(vec![Object::tuple(fact)])
}

/// The done of a write of a path of the scheme, with its text.
fn wrote(about: &str, path: &str, content: &str) -> Object {
  done(about, Text::new(format!("{SCRATCH}{path}"), content).object())
}

#[test]
fn a_record_has_one_scratch_directory_in_the_cache_of_furb_under_a_short_name_of_its_canonical_path()
 {
  let at = place("canonical");
  fs::create_dir_all(at.join("real")).unwrap();
  #[cfg(unix)]
  std::os::unix::fs::symlink(at.join("real"), at.join("link")).unwrap();
  let of = |path: PathBuf| Scratch::of(&path).expect("the machine of the test has a cache of furb");
  let one = of(at.join("real").join("a.jsonl"));
  assert_eq!(of(at.join("real").join(".").join("a.jsonl")).path(), one.path());
  #[cfg(unix)]
  assert_eq!(
    of(at.join("link").join("a.jsonl")).path(),
    one.path(),
    "a link to the folder names the same record"
  );
  assert_ne!(of(at.join("real").join("b.jsonl")).path(), one.path());
  let cache =
    crate::world::cache().map(|cache| Scratch::at(cache.join("scratch")).path().to_path_buf());
  assert_eq!(one.path().parent(), cache.as_deref());
  assert_eq!(one.path().file_name().map(|name| name.len()), Some(16));
  assert_eq!(Scratch::at("a/../b").path(), std::env::current_dir().unwrap().join("b"));
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn a_path_of_the_scheme_is_a_file_of_the_directory_and_a_folder_gives_its_entries_all_named_under_the_scheme()
 {
  let at = place("served");
  let scratch = Scratch::at(&at);
  assert_eq!(scratch.served("", None).unwrap(), Text::new(SCRATCH, ""));
  assert!(!at.exists(), "a read makes no directory");
  let wrote = scratch.served("notes/../notes/a.md", Some("one\n".to_owned())).unwrap();
  assert_eq!(wrote, Text::new("scratch://notes/a.md", "one\n"));
  scratch.served("plan.md", Some("two".to_owned())).unwrap();
  assert!(at.join(WHOLE).is_file(), "a write makes the directory whole");
  assert_eq!(fs::read_to_string(at.join("notes").join("a.md")).unwrap(), "one\n");
  assert_eq!(scratch.served("plan.md", None).unwrap(), Text::new("scratch://plan.md", "two"));
  assert_eq!(scratch.served("", None).unwrap(), Text::new(SCRATCH, "notes/\nplan.md"));
  assert_eq!(scratch.served("notes", None).unwrap(), Text::new("scratch://notes", "a.md"));
  let none = scratch.served("none.md", None).unwrap_err();
  assert_eq!(none.message(), "There is no file at scratch://none.md.");
  let out = scratch.served("../out.md", Some("x".to_owned())).unwrap_err();
  assert_eq!(out.message(), "scratch://../out.md is outside the scratch directory.");
  assert!(!at.parent().unwrap().join("out.md").exists());
  fs::remove_dir_all(&at).unwrap();
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_out_of_the_directory_is_refused_and_a_listing_reads_no_link_as_a_folder() {
  let at = place("link");
  let away = place("away");
  fs::create_dir_all(&away).unwrap();
  fs::write(away.join("secret.md"), "secret").unwrap();
  let scratch = Scratch::at(&at);
  scratch.made().unwrap();
  std::os::unix::fs::symlink(&away, at.join("away")).unwrap();
  let refused = |path: &str, content: Option<&str>| {
    scratch.served(path, content.map(str::to_owned)).unwrap_err().message()
  };
  assert_eq!(
    refused("away/secret.md", None),
    "scratch://away/secret.md is outside the scratch directory."
  );
  assert_eq!(
    refused("away/new.md", Some("x")),
    "scratch://away/new.md is outside the scratch directory."
  );
  assert!(!away.join("new.md").exists());
  assert_eq!(scratch.served("", None).unwrap().content, "away");
  fs::remove_dir_all(&at).unwrap();
  fs::remove_dir_all(&away).unwrap();
}

#[test]
fn a_missing_directory_takes_the_last_write_of_each_path_in_the_order_of_those_writes() {
  let at = place("remade");
  let record = [
    wrote("write1", "plan.md", "old"),
    wrote("write2", "a", "a file"),
    done("write3", Text::new("/w/a.txt", "elsewhere").object()),
    done("read1", Text::new("scratch://seen.md", "read").object()),
    wrote("write4", "a/b.md", "in a folder"),
    wrote("write5", "plan.md", "new"),
  ];
  Scratch::at(&at).remade(&record);
  assert!(at.join(WHOLE).is_file());
  assert_eq!(Scratch::at(&at).served("", None).unwrap().content, "a/\nplan.md");
  assert_eq!(fs::read_to_string(at.join("a").join("b.md")).unwrap(), "in a folder");
  assert_eq!(fs::read_to_string(at.join("plan.md")).unwrap(), "new");
  let folded = place("folded");
  Scratch::at(&folded)
    .remade(&[wrote("write1", "Plan.md", "first"), wrote("write2", "plan.md", "last")]);
  assert_eq!(
    fs::read_to_string(folded.join("plan.md")).unwrap(),
    "last",
    "the last write wins where case folds"
  );
  fs::remove_dir_all(&folded).unwrap();
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn a_whole_directory_stands_a_directory_that_is_not_whole_is_made_again_and_a_new_record_starts_empty()
 {
  let at = place("whole");
  let record = [wrote("write1", "plan.md", "kept")];
  Scratch::at(&at).remade(&record);
  fs::write(at.join("plan.md"), "since").unwrap();
  Scratch::at(&at).remade(&record);
  assert_eq!(
    fs::read_to_string(at.join("plan.md")).unwrap(),
    "since",
    "a whole directory stands as it is"
  );
  fs::remove_file(at.join(WHOLE)).unwrap();
  Scratch::at(&at).remade(&record);
  assert_eq!(
    fs::read_to_string(at.join("plan.md")).unwrap(),
    "kept",
    "a directory that is not whole is made again"
  );
  assert!(!at.with_file_name(format!("{}.new", at.file_name().unwrap().display())).exists());
  Scratch::at(&at).remade(&[]);
  assert!(!at.exists(), "a new record starts with an empty directory");
}

#[test]
fn a_scratch_directory_of_its_own_has_a_short_name_and_goes_with_the_last_ear_that_holds_it() {
  let own = Scratch::own();
  let held = own.clone();
  let at = own.made().unwrap().to_path_buf();
  assert!(at.starts_with(std::env::temp_dir()));
  let socket = at.join("pymp-abcdefgh").join("listener-abcdefgh");
  assert!(socket.as_os_str().len() < 104, "a socket of python fits under {}", at.display());
  drop(own);
  assert!(at.is_dir());
  drop(held);
  assert!(!at.exists());
}
