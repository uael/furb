use std::{fs, path::PathBuf};

use super::{DIGEST, PROGRAM, shelved, sound};
use crate::world::VERSION;

/// A new folder of one test.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-builds-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

#[test]
fn a_build_stands_in_the_store_under_its_version_and_the_first_build_of_a_version_stays() {
  let at = place("shelved");
  let (first, second, store) = (at.join("first"), at.join("second"), at.join("builds"));
  fs::write(&first, "the first build").unwrap();
  fs::write(&second, "a later build").unwrap();
  let stood = shelved(&store, &first).unwrap();
  assert_eq!(stood, store.join(VERSION.to_string()).join(PROGRAM));
  assert_eq!(shelved(&store, &second).unwrap(), stood);
  assert_eq!(fs::read_to_string(&stood).unwrap(), "the first build");
  fs::remove_file(&first).unwrap();
  fs::write(&first, "a build that writes a new file at the same path").unwrap();
  assert_eq!(
    fs::read_to_string(&stood).unwrap(),
    "the first build",
    "the store keeps the file of the build"
  );
  let mut left: Vec<_> = fs::read_dir(store.join(VERSION.to_string()))
    .unwrap()
    .map(|one| one.unwrap().file_name().into_string().unwrap())
    .collect();
  left.sort();
  assert_eq!(left, [PROGRAM, DIGEST], "no part stays");
}

#[test]
fn a_build_in_the_store_that_another_file_wrote_over_is_not_sound_and_the_next_build_puts_it_there_again()
 {
  let at = place("unsound");
  let (program, store) = (at.join("program"), at.join("builds"));
  fs::write(&program, "the build of this version").unwrap();
  let stood = shelved(&store, &program).unwrap();
  assert!(sound(&stood));
  fs::write(&stood, "a build of another version").unwrap();
  assert!(!sound(&stood), "the sha256 beside it tells the change");
  assert_eq!(shelved(&store, &program).unwrap(), stood);
  assert_eq!(fs::read_to_string(&stood).unwrap(), "the build of this version");
  assert!(sound(&stood));
  fs::remove_file(stood.with_file_name(DIGEST)).unwrap();
  assert!(sound(&stood), "a build of an earlier furb has no sha256 and counts as sound");
}

#[test]
fn a_store_that_cannot_hold_the_build_says_where_and_why() {
  let at = place("unshelved");
  let store = at.join("builds");
  fs::write(&store, "a file where the store would stand").unwrap();
  let program = at.join("program");
  fs::write(&program, "a build").unwrap();
  let refused = shelved(&store, &program).unwrap_err().to_string();
  let expected = format!(
    "OSError: the build of version {VERSION} cannot stand at {}",
    store.join(VERSION.to_string()).join(PROGRAM).display()
  );
  assert!(refused.starts_with(&expected), "{refused}");
}
