//! The one write of a file, proved on a folder of each test.

use std::{fs, io::Write};

use super::{Access, beside, filled, write};
use crate::testing::Yard;
use std::io;
use std::path;
use std::process;
use std::sync::atomic;

/// An empty folder of the test, named with the id of the process, as every yard of the tests of
/// the crate is.
fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-disk-{name}-{}", process::id()))
}

fn names(folder: &path::Path) -> Vec<String> {
  let mut found: Vec<_> = fs::read_dir(folder)
    .expect("the folder reads")
    .map(|one| one.expect("an entry").file_name().to_string_lossy().into_owned())
    .collect();
  found.sort();
  found
}

#[test]
fn a_write_replaces_the_file_whole_and_leaves_no_part_file() {
  let at = yard("replace");
  let file = at.join("state.json");
  write(&file, b"old", Access::Shared).expect("the first write");
  write(&file, b"new", Access::Shared).expect("the second write");
  assert_eq!(fs::read_to_string(&file).expect("the file reads"), "new");
  assert_eq!(names(&at), ["state.json"]);
}

#[test]
fn a_write_makes_the_folders_above_the_file() {
  let at = yard("folders");
  let file = at.join("a").join("b").join("state.json");
  write(&file, b"x", Access::Shared).expect("the write");
  assert_eq!(fs::read_to_string(&file).expect("the file reads"), "x");
}

#[test]
fn a_write_that_fails_names_the_file_and_leaves_the_folder_as_it_was() {
  let at = yard("fails");
  fs::write(at.join("held"), "a file").expect("a file where a folder is asked");
  let no = write(&at.join("held").join("state.json"), b"x", Access::Shared).expect_err("no folder");
  assert!(no.path.ends_with("held/state.json"));
  let shown = path::Path::new("held").join("state.json");
  assert!(no.to_string().contains(&format!("{} was not written", shown.display())));
  assert_eq!(names(&at), ["held"]);
}

#[test]
fn a_write_that_fails_midway_as_on_a_full_disk_leaves_the_old_file_whole_and_no_part_of_a_new_one()
{
  for access in [Access::Shared, Access::Owner] {
    let at = yard("midway");
    fs::write(at.join("a.txt"), "old\n").unwrap();
    let torn = filled(&at.join("a.txt"), b"new content\n", access, |file, bytes| {
      file.write_all(&bytes[..4])?;
      Err(io::Error::other("the disk is full"))
    });
    assert_eq!(torn.unwrap_err().source.to_string(), "the disk is full");
    let full = |_: &mut fs::File, _: &[u8]| Err(io::Error::other("the disk is full"));
    assert!(filled(&at.join("b.png"), b"new\n", access, full).is_err(), "a new file fails too");
    assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
    assert_eq!(names(&at), ["a.txt"], "no part file stays, and no part of a new file");
  }
}

#[test]
fn a_part_file_passes_each_name_that_a_crash_left() {
  let at = yard("stale");
  let part = |count: u32| at.join(format!(".a.txt.{}.{count}.part", process::id()));
  for count in 0..3 {
    fs::write(part(count), "stale").unwrap();
  }
  let counts = atomic::AtomicU32::new(0);
  let (made, _) = beside(&at.join("a.txt"), Access::Shared, &counts).expect("a part file opens");
  assert_eq!(made, part(3));
}

#[cfg(unix)]
#[test]
fn a_reader_that_holds_a_file_open_during_a_write_reads_the_old_bytes_whole() {
  use std::io::Read;
  let at = yard("reader");
  fs::write(at.join("a.txt"), "old text\n").unwrap();
  let mut held = fs::File::open(at.join("a.txt")).unwrap();
  write(&at.join("a.txt"), b"new\n", Access::Shared).unwrap();
  let mut seen = String::new();
  held.read_to_string(&mut seen).unwrap();
  assert_eq!(seen, "old text\n", "the write never touched the bytes that the reader holds");
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "new\n");
}

#[cfg(unix)]
#[test]
fn a_shared_write_keeps_the_mode_of_the_old_file_and_one_of_the_owner_gives_it_to_the_user_alone() {
  use std::os::unix::fs::PermissionsExt;
  let at = yard("mode");
  let mode = |name: &str| fs::metadata(at.join(name)).unwrap().permissions().mode() & 0o7777;
  for name in ["run.sh", "token.json"] {
    fs::write(at.join(name), "old\n").unwrap();
    fs::set_permissions(at.join(name), fs::Permissions::from_mode(0o751)).unwrap();
  }
  write(&at.join("run.sh"), b"new\n", Access::Shared).unwrap();
  write(&at.join("token.json"), b"new\n", Access::Owner).unwrap();
  write(&at.join("new.json"), b"new\n", Access::Owner).unwrap();
  assert_eq!(mode("run.sh"), 0o751);
  assert_eq!(mode("token.json"), 0o600);
  assert_eq!(mode("new.json"), 0o600);
}

#[cfg(unix)]
#[test]
fn a_write_to_a_link_keeps_the_link_and_changes_its_target() {
  use std::os::unix::fs::symlink;
  let at = yard("link");
  fs::create_dir_all(at.join("deep")).unwrap();
  fs::write(at.join("deep/target.txt"), "old\n").unwrap();
  symlink("deep/target.txt", at.join("mid")).unwrap();
  symlink("mid", at.join("link")).unwrap();
  write(&at.join("link"), b"new\n", Access::Shared).unwrap();
  assert!(fs::symlink_metadata(at.join("link")).unwrap().file_type().is_symlink());
  assert!(fs::symlink_metadata(at.join("mid")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("deep/target.txt")).unwrap(), "new\n");
  fs::create_dir_all(at.join("gone")).unwrap();
  symlink("gone/made.txt", at.join("dangling")).unwrap();
  write(&at.join("dangling"), b"made\n", Access::Shared).unwrap();
  assert!(fs::symlink_metadata(at.join("dangling")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("gone/made.txt")).unwrap(), "made\n");
}

#[cfg(unix)]
#[test]
fn a_write_to_a_file_that_the_user_cannot_write_is_refused_as_a_plain_write_refuses_it() {
  use std::os::unix::fs::PermissionsExt;
  let at = yard("readonly");
  fs::write(at.join("a.txt"), "old\n").unwrap();
  fs::set_permissions(at.join("a.txt"), fs::Permissions::from_mode(0o444)).unwrap();
  let root = fs::OpenOptions::new().write(true).open(at.join("a.txt")).is_ok();
  if !root {
    assert!(write(&at.join("a.txt"), b"new\n", Access::Shared).is_err());
    assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
  }
}
