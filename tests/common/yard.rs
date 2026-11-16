//! The yard of a test: a folder of its own in the temporary directory, which goes when the test
//! ends. The tests of each package of the workspace share it: the crate as a module of its tests,
//! and each suite of `tests/` by a path, as `common/pipes.rs`.

#![allow(dead_code, reason = "a suite takes the part of the yard that it needs")]

use std::{
  env, fs,
  ops::Deref,
  path::{Path, PathBuf},
  sync::atomic::{AtomicUsize, Ordering},
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
  pub(crate) fn new(name: impl AsRef<Path>) -> Self {
    let yard = Self::unmade(name);
    if let Err(no) = fs::create_dir_all(&yard.base) {
      panic!("the yard {} was not made: {no}", yard.base.display());
    }
    yard
  }

  /// The folder of this name in the temporary directory, which the test or the code that it tests
  /// makes. The name takes a count of the yards of the process after it: the tests of a program run
  /// on threads of one process, and two of them that name the same yard hold a folder each.
  pub(crate) fn unmade(name: impl AsRef<Path>) -> Self {
    static MADE: AtomicUsize = AtomicUsize::new(0);
    let mut name = name.as_ref().as_os_str().to_owned();
    name.push(format!("-{}", MADE.fetch_add(1, Ordering::Relaxed)));
    Self::exact(name)
  }

  /// The folder of exactly this name, made empty, for a test whose later life finds it again by its
  /// name alone. No other test of the process may name it.
  pub(crate) fn named(name: impl AsRef<Path>) -> Self {
    let yard = Self::exact(name);
    if let Err(no) = fs::create_dir_all(&yard.base) {
      panic!("the yard {} was not made: {no}", yard.base.display());
    }
    yard
  }

  fn exact(name: impl AsRef<Path>) -> Self {
    let base = temp().join(name);
    drop(fs::remove_dir_all(&base));
    Self { at: base.clone(), base }
  }

  /// The same yard, read as this path beneath its folder.
  pub(crate) fn within(mut self, path: impl AsRef<Path>) -> Self {
    self.at = self.at.join(path);
    self
  }
}

/// The temporary directory with no link in it, as a process that starts there reads its folder and
/// as the daemon names a record: on macOS it stands under `/var`, a link to `/private/var`, and
/// on a runner of Windows under a short name of the user, as `RUNNER~1`.
fn temp() -> PathBuf {
  let temp = env::temp_dir();
  let Ok(real) = fs::canonicalize(&temp) else { return temp };
  // Windows gives a resolved path the prefix `\\?\`, which no folder of a process has.
  match real.to_str().and_then(|one| one.strip_prefix(r"\\?\")) {
    Some(plain) => PathBuf::from(plain),
    None => real,
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
  fn from(yard: &Yard) -> Self {
    yard.at.clone()
  }
}

impl Drop for Yard {
  fn drop(&mut self) {
    if env::var_os(KEEP).is_some() {
      eprintln!("{KEEP} keeps {}", self.base.display());
    } else {
      drop(fs::remove_dir_all(&self.base));
    }
  }
}
