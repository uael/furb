use std::{fs, path::PathBuf};

use super::{PROGRAM, shelved};
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
  let left: Vec<_> = fs::read_dir(store.join(VERSION.to_string()))
    .unwrap()
    .map(|one| one.unwrap().file_name())
    .collect();
  assert_eq!(left, [PROGRAM], "no part stays");
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
