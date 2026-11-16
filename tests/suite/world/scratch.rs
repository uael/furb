//! The scratch directory of a record: its place, the paths of its scheme as a life reads and writes
//! them with the files of the World, the files that a record makes again in it, and the reap of the
//! directories that no life holds.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{
  env, ffi, fs, io,
  path::{Path, PathBuf},
  process,
  sync::Mutex,
  time::{Duration, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

use super::provider::closed;
use crate::{drive::park, yard::Yard};
use furb::{
  Ear, Engine, Object, Text,
  world::{self, SCRATCH, Scratch},
};

/// The file that marks a directory as whole, which the tasks of the developer read too.
const MARK: &str = ".furb-scratch";

/// A directory of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::unmade(format!("furb-scratch-{name}-{}", process::id()))
}

/// A log of the host that keeps nothing, for a record whose files the machine takes.
fn quiet(_: &str) {}

/// One entry of a record: the done of an act with what it came to, as the World said it.
fn done(about: &str, value: Object) -> Object {
  let fact = [Object::string("done"), Object::string(about), Object::string("files"), value];
  Object::list(vec![Object::tuple(fact)])
}

/// The done of a write of a path of the scheme, with its text.
fn wrote(about: &str, path: &str, content: &str) -> Object {
  done(about, Text::new(format!("{SCRATCH}{path}"), content).object())
}

/// A life on the files of the World alone, whose scratch directory is this one.
fn life(scratch: Scratch) -> Engine {
  let config = env::temp_dir().join(format!("furb-scratch-user-{}", process::id()));
  let ears: [(&str, Box<dyn Ear>); 1] = [("files", world::files(scratch, config, None))];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// The text that a word of the root of a life closes with, or why it raised.
fn text(engine: &mut Engine, word: &str) -> Result<Text, String> {
  closed(engine, &format!("close({word})")).map(|got| Text::of(got.as_ref()).expect("a text"))
}

#[test]
fn a_record_has_one_scratch_directory_in_the_cache_of_furb_under_a_short_name_of_its_canonical_path()
 {
  let at = place("canonical");
  fs::create_dir_all(at.join("real")).unwrap();
  #[cfg(unix)]
  unix_fs::symlink(at.join("real"), at.join("link")).unwrap();
  let of = |path: PathBuf| {
    Scratch::of(&path).unwrap().expect("the machine of the test has a cache of furb")
  };
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
    world::cache().map(|cache| Scratch::at(cache.join("scratch")).unwrap().path().to_path_buf());
  assert_eq!(one.path().parent(), cache.as_deref());
  assert_eq!(one.path().file_name().map(ffi::OsStr::len), Some(16));
  assert_eq!(Scratch::at("a/../b").unwrap().path(), env::current_dir().unwrap().join("b"));
}

#[test]
fn a_path_of_the_scheme_is_a_file_of_the_directory_and_a_folder_gives_its_entries_all_named_under_the_scheme()
 {
  let at = place("served");
  let mut engine = life(Scratch::at(&at).unwrap());
  assert_eq!(text(&mut engine, "read('scratch://')").unwrap(), Text::new(SCRATCH, ""));
  assert!(!at.exists(), "a read makes no directory");
  let wrote = text(&mut engine, "write(Text('scratch://notes/../notes/a.md', 'one\\n'))");
  assert_eq!(wrote.unwrap(), Text::new("scratch://notes/a.md", "one\n"));
  text(&mut engine, "write(Text('scratch://plan.md', 'two'))").unwrap();
  assert_eq!(fs::read_to_string(at.join("notes").join("a.md")).unwrap(), "one\n");
  let read = |engine: &mut Engine, path: &str| text(engine, &format!("read('{path}')"));
  assert_eq!(
    read(&mut engine, "scratch://plan.md").unwrap(),
    Text::new("scratch://plan.md", "two")
  );
  assert_eq!(read(&mut engine, "scratch://").unwrap(), Text::new(SCRATCH, "notes/\nplan.md"));
  assert_eq!(read(&mut engine, "scratch://notes").unwrap(), Text::new("scratch://notes", "a.md"));
  let none = read(&mut engine, "scratch://none.md").unwrap_err();
  assert!(none.contains("There is no file at scratch://none.md."), "{none}");
  let out = text(&mut engine, "write(Text('scratch://../out.md', 'x'))").unwrap_err();
  assert!(out.contains("scratch://../out.md is outside the scratch directory."), "{out}");
  assert!(!at.parent().unwrap().join("out.md").exists());
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_out_of_the_directory_is_refused_and_a_listing_reads_no_link_as_a_folder() {
  let (at, away) = (place("link"), place("away"));
  fs::create_dir_all(&away).unwrap();
  fs::write(away.join("secret.md"), "secret").unwrap();
  let mut engine = life(Scratch::at(&at).unwrap());
  text(&mut engine, "write(Text('scratch://kept.md', 'x'))").unwrap();
  unix_fs::symlink(&away, at.join("away")).unwrap();
  let read = text(&mut engine, "read('scratch://away/secret.md')").unwrap_err();
  assert!(read.contains("scratch://away/secret.md is outside the scratch directory."), "{read}");
  let write = text(&mut engine, "write(Text('scratch://away/new.md', 'x'))").unwrap_err();
  assert!(write.contains("scratch://away/new.md is outside the scratch directory."), "{write}");
  assert!(!away.join("new.md").exists());
  assert_eq!(text(&mut engine, "read('scratch://')").unwrap().content, "away\nkept.md");
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
  Scratch::at(&at).unwrap().remade(&record, &quiet);
  let mut engine = life(Scratch::at(&at).unwrap());
  assert_eq!(text(&mut engine, "read('scratch://')").unwrap().content, "a/\nplan.md");
  assert_eq!(fs::read_to_string(at.join("a").join("b.md")).unwrap(), "in a folder");
  assert_eq!(fs::read_to_string(at.join("plan.md")).unwrap(), "new");
  let folded = place("folded");
  Scratch::at(&folded)
    .unwrap()
    .remade(&[wrote("write1", "Plan.md", "first"), wrote("write2", "plan.md", "last")], &quiet);
  assert_eq!(
    fs::read_to_string(folded.join("plan.md")).unwrap(),
    "last",
    "the last write wins where case folds"
  );
}

#[test]
fn a_file_that_the_machine_refuses_is_told_to_the_log_of_the_host() {
  let at = place("refused");
  let told = Mutex::new(Vec::new());
  Scratch::at(&at).unwrap().remade(
    &[wrote("write1", "../out.md", "out"), wrote("write2", "plan.md", "in")],
    &|line| {
      told.lock().unwrap().push(line.to_owned());
    },
  );
  assert_eq!(
    *told.lock().unwrap(),
    [format!(
      "the scratch directory of the record leaves out {SCRATCH}../out.md: it is outside the scratch directory"
    )]
  );
  assert_eq!(fs::read_to_string(at.join("plan.md")).unwrap(), "in");
}

#[test]
fn a_whole_directory_stands_a_directory_that_is_not_whole_is_made_again_and_a_new_record_starts_empty()
 {
  let at = place("whole");
  let record = [wrote("write1", "plan.md", "kept")];
  Scratch::at(&at).unwrap().remade(&record, &quiet);
  fs::write(at.join("plan.md"), "since").unwrap();
  Scratch::at(&at).unwrap().remade(&record, &quiet);
  assert_eq!(
    fs::read_to_string(at.join("plan.md")).unwrap(),
    "since",
    "a whole directory stands as it is"
  );
  fs::remove_file(at.join(MARK)).unwrap();
  Scratch::at(&at).unwrap().remade(&record, &quiet);
  assert_eq!(
    fs::read_to_string(at.join("plan.md")).unwrap(),
    "kept",
    "a directory that is not whole is made again"
  );
  assert!(!at.with_file_name(format!("{}.new", at.file_name().unwrap().display())).exists());
  Scratch::at(&at).unwrap().remade(&[], &quiet);
  assert!(!at.exists(), "a new record starts with no directory");
  Scratch::at(&at).unwrap().remade(&[done("bash1", Object::string("out"))], &quiet);
  assert!(!at.exists(), "a record that no write put a file in makes no directory");
}

#[test]
fn a_scratch_directory_of_its_own_has_a_short_name_and_goes_with_the_last_ear_that_holds_it() {
  let own = Scratch::own();
  let held = own.clone();
  own.remade(&[wrote("write1", "notes.md", "made")], &quiet);
  let at = own.path().to_path_buf();
  assert!(at.join("notes.md").is_file(), "the directory stands");
  assert!(at.starts_with(env::temp_dir()));
  let socket = at.join("pymp-abcdefgh").join("listener-abcdefgh");
  assert!(socket.as_os_str().len() < 104, "a socket of python fits under {}", at.display());
  drop(own);
  assert!(at.is_dir());
  drop(held);
  assert!(!at.exists());
}

/// A file or a folder opened so that its time can change: a folder of Windows opens only with
/// backup semantics, and its time changes with the right to write its attributes.
fn timed(path: &Path) -> io::Result<fs::File> {
  #[cfg(windows)]
  {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new().access_mode(0x100).custom_flags(0x0200_0000).open(path)
  }
  #[cfg(not(windows))]
  fs::File::open(path)
}

/// A directory and each entry under it, at any depth, last changed this long ago, but for the
/// entries that `kept` names.
fn aged(at: &Path, ago: Duration, kept: &[PathBuf]) {
  let when = SystemTime::now() - ago;
  for entry in fs::read_dir(at).expect("the directory reads").flatten() {
    let path = entry.path();
    if entry.file_type().expect("its kind").is_dir() {
      aged(&path, ago, kept);
    } else if !kept.contains(&path) {
      timed(&path).and_then(|file| file.set_modified(when)).expect("its time changes");
    }
  }
  timed(at).and_then(|file| file.set_modified(when)).expect("its time changes");
}

/// Whether a condition holds within ten seconds, as a reap of the background makes it hold.
fn eventually(holds: impl Fn() -> bool) -> bool {
  (0..1000).any(|_| {
    holds() || {
      park(Duration::from_millis(10));
      false
    }
  })
}

/// Eight days, past the seven of a config that names none.
const PAST: Duration = Duration::from_hours(24 * 8);

#[test]
fn a_reap_removes_the_scratch_of_a_record_past_its_days_that_no_life_holds_and_a_reopen_makes_it_again()
 {
  let at = place("reap");
  let root = at.join("scratch");
  let named = |one: char| root.join(one.to_string().repeat(16));
  let (old, held, recent, reopened) = (named('a'), named('b'), named('c'), named('d'));
  let record = [wrote("write1", "notes.md", "kept")];
  Scratch::at(&old).unwrap().remade(&record, &quiet);
  Scratch::at(&reopened).unwrap().remade(&record, &quiet);
  // A life that opens again on a whole directory holds it too.
  let again = Scratch::at(&reopened).unwrap();
  again.remade(&record, &quiet);
  Scratch::at(&recent).unwrap().remade(&record, &quiet);
  let life = Scratch::at(&held).unwrap();
  life.remade(&record, &quiet);
  fs::write(old.join("made"), "a file that a command made").unwrap();
  for one in [&old, &held, &reopened] {
    aged(one, PAST, &[]);
  }
  life.reap(&at.join("user"), &at.join("project"));
  assert!(eventually(|| !old.exists()), "the directory that no life holds and no use kept goes");
  assert!(held.join("notes.md").is_file() && recent.join("notes.md").is_file());
  assert!(reopened.join("notes.md").is_file(), "a life that reopened a whole directory holds it");
  Scratch::at(&old).unwrap().remade(&record, &quiet);
  assert_eq!(fs::read_to_string(old.join("notes.md")).unwrap(), "kept", "a reopen writes it again");
  assert!(!old.join("made").exists(), "a file that a command made is lost");
  drop((life, again));
}

#[test]
fn a_reap_keeps_a_directory_whose_file_changed_within_the_days_and_goes_on_past_one_that_will_not_go()
 {
  let at = place("reap-kept");
  let root = at.join("scratch");
  let named = |one: char| root.join(one.to_string().repeat(16));
  let (written, stuck, free) = (named('a'), named('b'), named('c'));
  let record = [wrote("write1", "notes.md", "kept")];
  for one in [&written, &stuck, &free] {
    Scratch::at(one).unwrap().remade(&record, &quiet);
  }
  fs::create_dir_all(written.join("deep/down")).unwrap();
  fs::write(written.join("deep/down/log"), "").unwrap();
  #[cfg(unix)]
  fs::create_dir_all(stuck.join("locked")).unwrap();
  #[cfg(unix)]
  fs::write(stuck.join("locked/file"), "").unwrap();
  aged(&written, PAST, &[written.join("deep/down/log")]);
  aged(&stuck, PAST, &[]);
  aged(&free, PAST, &[]);
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(stuck.join("locked"), fs::Permissions::from_mode(0o555)).unwrap();
  }
  Scratch::at(&free).unwrap().reap(&at.join("user"), &at.join("project"));
  assert!(eventually(|| !free.exists()), "a directory past its days goes");
  assert!(written.join("deep/down/log").is_file(), "a change at any depth keeps a directory");
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    assert!(stuck.join("locked/file").is_file(), "a directory that will not go stays");
    fs::set_permissions(stuck.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
  }
}
