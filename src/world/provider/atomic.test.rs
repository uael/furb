use std::{fs, path::PathBuf};

use super::{Access, Error, write};

/// An empty folder of the test, named with the id of the process, as every yard of the tests of
/// the crate is.
fn yard(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-atomic-{name}-{}", std::process::id()));
  drop(fs::remove_dir_all(&at));
  fs::create_dir_all(&at).expect("a yard of the test");
  at
}

fn names(folder: &std::path::Path) -> Vec<String> {
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
  assert!(matches!(&no, Error::Write { path, .. } if path.ends_with("held/state.json")));
  assert!(no.to_string().contains("held/state.json was not written"));
  assert_eq!(names(&at), ["held"]);
}

#[test]
fn two_targets_that_differ_in_their_extension_keep_each_their_own_part_file() {
  let at = yard("extensions");
  write(&at.join("prompt.md"), b"md", Access::Shared).expect("the first write");
  write(&at.join("prompt.json"), b"json", Access::Shared).expect("the second write");
  assert_eq!(names(&at), ["prompt.json", "prompt.md"]);
}

#[cfg(unix)]
#[test]
fn a_write_for_the_owner_leaves_a_file_that_no_one_else_reads() {
  use std::os::unix::fs::PermissionsExt;
  let at = yard("owner");
  let file = at.join("token.json");
  write(&file, b"secret", Access::Owner).expect("the write");
  let mode = fs::metadata(&file).expect("the file stats").permissions().mode();
  assert_eq!(mode & 0o777, 0o600);
}
