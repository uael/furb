//! The World that the crate writes: ears of the outside that serve the machine, one for each concern.
//!
//! The contract makes the World the ears of the outside that serve the machine, one or many. These are the ones
//! every host shares, written once in rust: [`files`] reads and writes a path, the files of the [`Scratch`] directory
//! of the life among them, [`bash`] runs a command, [`time`] reads the clock, draws a chance and ends a wait,
//! [`store`] keeps the record on the disk under a lease, as a file of lines or a database of SQLite, the store of
//! builds holds the program of each version that ran on this machine, which [`stored`] finds, [`log`] writes
//! the log of the life beside the record, which [`logged`] reads, [`retold`] reads a record with no life, and the
//! [`Provider`] answers what the chains stand on and asks the models of rig for their turns, which the [`Catalog`]
//! knows, the claude command line among them. The host names the models it offers, and gives the rest, its
//! console, which reads what the operator answers by the rules of [`answered`].
//!
//! Each is an ear like any other, so an ear that comes before one of them in the order of boot takes a question in
//! its place, and an ear that takes a question and asks it again wraps it.

mod bash;
mod builds;
pub(crate) mod files;
mod log;
mod operator;
mod provider;
mod reader;
mod scratch;
mod store;
mod time;

use std::{
  env,
  path::{Component, Path, PathBuf},
};

pub use bash::{bash, program, shell};
pub use builds::{build, shelve, stored};
pub use files::files;
pub(crate) use log::continued;
pub use log::{log, logged};
pub use operator::{SHAPES, answered, answers};
pub(crate) use provider::search::{posted, spawned};
pub use provider::{
  BACKOFF, Backoff, Hosted, Model, Provider, Retries, Retry, Search, Told, Writes,
  catalog::{self, Catalog},
  claude, images,
  search::Searches,
};
pub use reader::{READER, Retold, retell, retold};
pub use scratch::{SCRATCH, Scratch};
pub(crate) use store::database;
pub use store::{Lines, MADE, Sqlite, Store, VERSION, engine, keeping, kept, made, store, version};
pub use time::time;

use crate::{
  ear::Co,
  value::{Fault, Object},
};

/// The directory a chain stands in, which a path of it resolves against: where the chain went last, resolved against
/// the directory the life stands on.
pub(crate) async fn here<W: 'static>(co: &mut Co<W>, on: &str) -> Result<PathBuf, Fault> {
  let on = vec![("on", Object::string(on))];
  let cwd = co.call("cwd", vec![], on).await?;
  let standing = co.call("standing", vec![], vec![]).await?;
  let directory = standing
    .as_ref()
    .items()
    .and_then(|held| held.get(1).and_then(|one| one.as_str().map(str::to_owned)));
  let cwd = cwd.as_ref().as_str().map(str::to_owned).unwrap_or_default();
  Ok(resolved(Path::new(&directory.unwrap_or_default()), &cwd))
}

/// A path resolved against a directory, as a path of the machine: the directory with the path after it, or the path
/// itself when it is absolute, with every `.` and `..` read.
pub(crate) fn resolved(directory: &Path, path: &str) -> PathBuf {
  let mut out = PathBuf::new();
  for part in directory.join(path).components() {
    match part {
      Component::CurDir => {}
      Component::ParentDir => {
        out.pop();
      }
      other => out.push(other),
    }
  }
  out
}

/// The cache of furb on this machine: under `XDG_CACHE_HOME`, `LOCALAPPDATA` on Windows, `Library/Caches` of the
/// home on macOS, and `.cache` of the home elsewhere.
pub(crate) fn cache() -> Option<PathBuf> {
  let named = |name: &str| env::var_os(name).filter(|one| !one.is_empty()).map(PathBuf::from);
  let base = named("XDG_CACHE_HOME").or_else(|| {
    if cfg!(windows) {
      named("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
      env::home_dir().map(|home| home.join("Library/Caches"))
    } else {
      env::home_dir().map(|home| home.join(".cache"))
    }
  });
  base.map(|base| base.join("furb"))
}

#[cfg(test)]
#[path = "world.test.rs"]
mod test;
