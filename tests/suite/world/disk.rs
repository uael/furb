//! The one write of a file, through a life that writes with the files of the World. A write that
//! fails or waits midway stands beside the module, in `src/world/disk.test.rs`, since no door holds
//! a write open.

#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{env, fs, io::Write, path::Path, process};

use crate::{drive::block_on, yard::Yard};
use furb::{Ear, Engine, Object, verbs, world};

/// An empty folder of the test, named with the id of the process.
fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-disk-{name}-{}", process::id()))
}

/// The names in a folder, in order.
fn names(folder: &Path) -> Vec<String> {
  let mut found: Vec<_> = fs::read_dir(folder)
    .expect("the folder reads")
    .map(|one| one.expect("an entry").file_name().to_string_lossy().into_owned())
    .collect();
  found.sort();
  found
}

/// A life on the files of the World alone.
fn life() -> Engine {
  let config = env::temp_dir().join(format!("furb-disk-user-{}", process::id()));
  let ears: [(&str, Box<dyn Ear>); 1] =
    [("files", world::files(world::Scratch::own(), config, None))];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// What a life comes to when its root writes this text at this path, or why the write raised.
fn written(engine: &mut Engine, at: &Path, text: &str) -> Result<(), String> {
  let path = serde_json::to_string(&at.display().to_string()).expect("a path as a string");
  let text = serde_json::to_string(text).expect("a text as a string");
  let word = format!("write(Text({path}, {text}))\nclose(0)");
  let root = engine.root().to_owned();
  let rung = verbs::Rung { word: Some(word), on: Some(root), ..Default::default() };
  let got = block_on(engine.rung(rung).expect("a rung is made"));
  got.map(drop).map_err(|no| no.to_string())
}

#[test]
fn a_write_replaces_the_file_whole_and_leaves_no_part_file() {
  let (at, mut engine) = (yard("replace"), life());
  let file = at.join("state.json");
  written(&mut engine, &file, "old").unwrap();
  written(&mut engine, &file, "new").unwrap();
  assert_eq!(fs::read_to_string(&file).unwrap(), "new");
  assert_eq!(names(&at), ["state.json"]);
}

#[test]
fn a_write_makes_the_folders_above_the_file() {
  let (at, mut engine) = (yard("folders"), life());
  let file = at.join("a").join("b").join("state.json");
  written(&mut engine, &file, "x").unwrap();
  assert_eq!(fs::read_to_string(&file).unwrap(), "x");
}

#[test]
fn a_write_that_fails_names_the_file_and_leaves_the_folder_as_it_was() {
  let (at, mut engine) = (yard("fails"), life());
  fs::write(at.join("held"), "a file").unwrap();
  let no = written(&mut engine, &at.join("held").join("state.json"), "x").unwrap_err();
  assert!(no.contains(&Path::new("held").join("state.json").display().to_string()), "{no}");
  assert_eq!(names(&at), ["held"]);
}

#[test]
fn a_part_file_passes_each_name_that_a_crash_left() {
  let (at, mut engine) = (yard("stale"), life());
  let parts: Vec<String> =
    (0..3).map(|count| format!(".a.txt.{}.{count}.part", process::id())).collect();
  let held: Vec<fs::File> = parts.iter().map(|part| live(&at.join(part))).collect();
  written(&mut engine, &at.join("a.txt"), "new").unwrap();
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "new");
  for part in &parts {
    assert_eq!(
      fs::read_to_string(at.join(part)).unwrap(),
      "stale",
      "a part of a live writer stays"
    );
  }
  drop(held);
}

/// A part file that a writer that runs holds, as a write of the World holds its own: on Windows
/// open with no share of delete, and elsewhere named by the pid of this process, which runs.
fn live(path: &Path) -> fs::File {
  let mut options = fs::OpenOptions::new();
  options.write(true).create_new(true);
  #[cfg(windows)]
  {
    use std::os::windows::fs::OpenOptionsExt;
    // FILE_SHARE_READ and FILE_SHARE_WRITE, with no FILE_SHARE_DELETE.
    options.share_mode(1 | 2);
  }
  let mut file = options.open(path).expect("the part file opens");
  file.write_all(b"stale").expect("the part file is written");
  file
}

/// A pid that no process holds: past the largest pid of each system.
const GONE: u32 = 2_147_483_646;

#[test]
fn the_first_write_of_a_process_into_a_folder_removes_each_part_file_that_a_writer_that_runs_no_more_left()
 {
  let (at, mut engine) = (yard("left"), life());
  let left = [format!(".a.txt.{GONE}.0.part"), format!(".b.txt.{GONE}.3.part")];
  let others = [".a.txt.x.part", ".part", "a.txt.part"];
  for name in left.iter().map(String::as_str).chain(others) {
    fs::write(at.join(name), "left by a kill").unwrap();
  }
  written(&mut engine, &at.join("a.txt"), "new").unwrap();
  let mut stays = vec!["a.txt"];
  stays.extend(others);
  stays.sort_unstable();
  assert_eq!(names(&at), stays, "each part that its writer left goes, and no other file");
  fs::write(at.join(&left[0]), "left by a later kill").unwrap();
  written(&mut engine, &at.join("a.txt"), "again").unwrap();
  assert!(names(&at).contains(&left[0]), "a second write into the folder reads no listing of it");
}

#[cfg(unix)]
#[test]
fn a_reader_that_holds_a_file_open_during_a_write_reads_the_old_bytes_whole() {
  use std::io::Read;
  let (at, mut engine) = (yard("reader"), life());
  fs::write(at.join("a.txt"), "old text\n").unwrap();
  let mut held = fs::File::open(at.join("a.txt")).unwrap();
  written(&mut engine, &at.join("a.txt"), "new\n").unwrap();
  let mut seen = String::new();
  held.read_to_string(&mut seen).unwrap();
  assert_eq!(seen, "old text\n", "the write never touched the bytes that the reader holds");
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "new\n");
}

#[cfg(unix)]
#[test]
fn a_write_keeps_the_mode_of_the_old_file() {
  use std::os::unix::fs::PermissionsExt;
  let (at, mut engine) = (yard("mode"), life());
  fs::write(at.join("run.sh"), "old\n").unwrap();
  fs::set_permissions(at.join("run.sh"), fs::Permissions::from_mode(0o751)).unwrap();
  written(&mut engine, &at.join("run.sh"), "new\n").unwrap();
  let mode = fs::metadata(at.join("run.sh")).unwrap().permissions().mode() & 0o7777;
  assert_eq!(mode, 0o751);
}

#[cfg(unix)]
#[test]
fn a_write_to_a_link_keeps_the_link_and_changes_its_target() {
  use std::os::unix::fs::symlink;
  let (at, mut engine) = (yard("link"), life());
  fs::create_dir_all(at.join("deep")).unwrap();
  fs::write(at.join("deep/target.txt"), "old\n").unwrap();
  symlink("deep/target.txt", at.join("mid")).unwrap();
  symlink("mid", at.join("link")).unwrap();
  written(&mut engine, &at.join("link"), "new\n").unwrap();
  assert!(fs::symlink_metadata(at.join("link")).unwrap().file_type().is_symlink());
  assert!(fs::symlink_metadata(at.join("mid")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("deep/target.txt")).unwrap(), "new\n");
  fs::create_dir_all(at.join("gone")).unwrap();
  symlink("gone/made.txt", at.join("dangling")).unwrap();
  written(&mut engine, &at.join("dangling"), "made\n").unwrap();
  assert!(fs::symlink_metadata(at.join("dangling")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("gone/made.txt")).unwrap(), "made\n");
}

#[cfg(unix)]
#[test]
fn a_write_to_a_file_that_the_user_cannot_write_is_refused_as_a_plain_write_refuses_it() {
  use std::os::unix::fs::PermissionsExt;
  let (at, mut engine) = (yard("readonly"), life());
  fs::write(at.join("a.txt"), "old\n").unwrap();
  fs::set_permissions(at.join("a.txt"), fs::Permissions::from_mode(0o444)).unwrap();
  let root = fs::OpenOptions::new().write(true).open(at.join("a.txt")).is_ok();
  if !root {
    assert!(written(&mut engine, &at.join("a.txt"), "new\n").is_err());
    assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
  }
}
