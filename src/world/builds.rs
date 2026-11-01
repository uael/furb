//! The store of builds: the program of the command line of each version, in the cache of furb, so a record of any
//! version opens on a build of that version.

use std::{
  fs, io,
  path::{Path, PathBuf},
  process,
};

use sha2::{Digest, Sha256};

use super::{VERSION, cache};
use crate::value::{Fault, Object};

/// The name of the program in the store of builds, on this system.
const PROGRAM: &str = if cfg!(windows) { "furb.exe" } else { "furb" };

/// The name of the file beside the program in the store that holds the sha256 of the program, in hex.
const DIGEST: &str = "furb.sha256";

/// Where the store of builds of this machine holds the program of a version: `<cache of furb>/builds/<version>/furb`.
/// It stands there once a build of that version ran on this machine, or once `script/builds.sh` made it.
pub fn build(version: i64) -> Option<PathBuf> {
  cache().map(|at| placed(&at.join("builds"), version))
}

/// The program of a version that the store of builds of this machine holds, and nothing when it holds none or when
/// the program is not the file that a build of that version put there.
pub fn stored(version: i64) -> Option<PathBuf> {
  build(version).filter(|at| sound(at))
}

/// Whether a program in a store is the file that a build put there: its sha256 is the one that the build wrote beside
/// it. A program that a build of an earlier furb put there has no digest, and counts as sound.
fn sound(at: &Path) -> bool {
  if !at.is_file() {
    return false;
  }
  match fs::read_to_string(at.with_file_name(DIGEST)) {
    Ok(kept) => digest(at).is_ok_and(|now| now == kept.trim()),
    Err(no) => no.kind() == io::ErrorKind::NotFound,
  }
}

/// The sha256 of a file, in hex.
fn digest(at: &Path) -> io::Result<String> {
  let mut file = fs::File::open(at)?;
  let mut hasher = Sha256::new();
  io::copy(&mut file, &mut hasher)?;
  Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
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

/// The program put in a store of builds as the program of this version, as a copy with its sha256 beside it, unless
/// the store holds a sound one already: a program whose sha256 changed is put there again. It moves into place whole,
/// so a build of the version that runs at the same moment never finds a part of it.
pub(crate) fn shelved(store: &Path, program: &Path) -> Result<PathBuf, Fault> {
  let at = placed(store, VERSION);
  if sound(&at) {
    return Ok(at);
  }
  let part = at.with_file_name(format!("{PROGRAM}.{}.part", process::id()));
  let sum = at.with_file_name(format!("{DIGEST}.{}.part", process::id()));
  let made = (|| -> io::Result<()> {
    fs::create_dir_all(store.join(VERSION.to_string()))?;
    let _ = fs::remove_file(&part);
    // A copy and never a second link: a tool that writes again into the file of the build, as a linker or cp onto
    // an existing file does, would change the program of the store too, and a record would open on another version.
    // On APFS, Btrfs and XFS the copy is a clone, which costs no space.
    fs::copy(program, &part)?;
    fs::write(&sum, digest(&part)?)?;
    fs::rename(&sum, at.with_file_name(DIGEST))?;
    fs::rename(&part, &at)
  })();
  made.map_err(|no| {
    let _ = fs::remove_file(&part);
    let _ = fs::remove_file(&sum);
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
