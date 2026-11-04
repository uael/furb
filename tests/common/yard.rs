//! The yard of a test: a folder of its own in the temporary directory, which goes when the test
//! ends. The tests of each package of the workspace share it: the crate as a module of its tests,
//! and each suite of `tests/` by a path, as `common/pipes.rs`.

#![allow(dead_code, reason = "a suite takes the part of the yard that it needs")]

use std::{
  fs,
  ops::Deref,
  path::{Path, PathBuf},
};

/// The env var that keeps the folder of each yard, so that a failure can still be read there.
const KEEP: &str = "FURB_KEEP_YARDS";

/// A folder of one test in the temporary directory, which goes with the yard when the test ends, on
/// a pass and on a failure, unless the env var `FURB_KEEP_YARDS` is set. The yard reads as a path
/// in it, the folder itself unless [`Yard::within`] names a file or a folder beneath.
pub(crate) struct Yard {
  base: PathBuf,
  at: PathBuf,
}

impl Yard {
  /// The folder of this name in the temporary directory, made empty.
  pub(crate) fn new(name: impl AsRef<Path>) -> Yard {
    let yard = Yard::unmade(name);
    if let Err(no) = fs::create_dir_all(&yard.base) {
      panic!("the yard {} was not made: {no}", yard.base.display());
    }
    yard
  }

  /// The folder of this name in the temporary directory, which the test or the code that it tests
  /// makes.
  pub(crate) fn unmade(name: impl AsRef<Path>) -> Yard {
    let base = std::env::temp_dir().join(name);
    drop(fs::remove_dir_all(&base));
    Yard { at: base.clone(), base }
  }

  /// The same yard, read as this path beneath its folder.
  pub(crate) fn within(mut self, path: impl AsRef<Path>) -> Yard {
    self.at = self.at.join(path);
    self
  }
}

impl Deref for Yard {
  type Target = Path;

  fn deref(&self) -> &Path {
    &self.at
  }
}

impl AsRef<Path> for Yard {
  fn as_ref(&self) -> &Path {
    &self.at
  }
}

impl From<&Yard> for PathBuf {
  fn from(yard: &Yard) -> PathBuf {
    yard.at.clone()
  }
}

impl Drop for Yard {
  fn drop(&mut self) {
    if std::env::var_os(KEEP).is_some() {
      eprintln!("{KEEP} keeps {}", self.base.display());
    } else {
      drop(fs::remove_dir_all(&self.base));
    }
  }
}
