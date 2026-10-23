//! The store of builds: the program of the command line of each version, in the cache of furb, so a record of any
//! version opens on a build of that version.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process,
};

use super::{VERSION, cache};
use crate::value::{Fault, Object};

/// The name of the program in the store of builds, on this system.
const PROGRAM: &str = if cfg!(windows) { "furb.exe" } else { "furb" };

/// Where the store of builds of this machine holds the program of a version: `<cache of furb>/builds/<version>/furb`.
/// It stands there once a build of that version ran on this machine, or once `script/builds.sh` made it.
pub fn build(version: i64) -> Option<PathBuf> {
  cache().map(|at| placed(&at.join("builds"), version))
}

/// The program of a version that the store of builds of this machine holds, and nothing when it holds none.
pub fn stored(version: i64) -> Option<PathBuf> {
  build(version).filter(|at| at.is_file())
}

/// Put a program of this build in the store of builds of this machine as the program of its version, unless the
/// store holds one already, and give where it stands.
pub fn shelve(program: &Path) -> Result<PathBuf, Fault> {
  let store = cache().ok_or_else(|| failed("this machine names no cache of furb".to_owned()))?;
  shelved(&store.join("builds"), program)
}

/// Where a store of builds holds the program of a version.
fn placed(store: &Path, version: i64) -> PathBuf {
  store.join(version.to_string()).join(PROGRAM)
}

/// The program put in a store of builds as the program of this version, unless the store holds one already: as a
/// second link to its file when the store is on the same disk, and else as a copy. Either moves into place whole, so
/// a build of the version that runs at the same moment never finds a part of it.
pub(crate) fn shelved(store: &Path, program: &Path) -> Result<PathBuf, Fault> {
  let at = placed(store, VERSION);
  if at.is_file() {
    return Ok(at);
  }
  let part = at.with_file_name(format!("{PROGRAM}.{}.part", process::id()));
  let made = (|| -> io::Result<()> {
    fs::create_dir_all(store.join(VERSION.to_string()))?;
    let _ = fs::remove_file(&part);
    // A link keeps the file of this build when a later build of the same path writes its own file there, as cargo
    // and pip do. A tool that writes again into the same file would change the build of the store too.
    if fs::hard_link(program, &part).is_err() {
      fs::copy(program, &part)?;
    }
    fs::rename(&part, &at)
  })();
  made.map_err(|no| {
    let _ = fs::remove_file(&part);
    failed(format!("the build of version {VERSION} cannot stand at {}: {no}", at.display()))
  })?;
  Ok(at)
}

/// A failure of the disk.
fn failed(why: String) -> Fault {
  Fault::new("OSError", vec![Object::string(why)])
}

#[cfg(test)]
#[path = "builds.test.rs"]
mod test;
