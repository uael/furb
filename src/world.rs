//! The World that the crate writes: ears of the outside that serve the machine, one for each
//! concern.
//!
//! The contract makes the World the ears of the outside that serve the machine, one or many. These
//! are the ones every host shares, written once in rust: [`files`] reads and writes a path, the
//! files of the [`Scratch`] directory of the life among them, [`bash`] runs a command, [`kits`]
//! hosts the process of each kit, as the ones of the web and of MCP,
//! [`time`] reads the clock, draws a chance and ends a wait, [`store`]
//! keeps the record on the disk under a lease, as a file of lines or a database of SQLite, [`log`]
//! writes the log of the life beside the record, which [`logged`] reads, [`retold`]
//! reads a record with no life, and the [`Provider`] answers what the chains stand on and asks the
//! models of rig for their turns, which the [`Catalog`] knows, the claude command line among them.
//! The host names the models it offers, and gives the rest, its console, which reads what the
//! operator answers by the rules of [`answered`]. Beside the ears, [`config`] is the one writer of
//! the configs that the provider and the extensions read, for each host and each window.
//! [`spawn`] starts the child of a host in a group that ends with it.
//!
//! Each is an ear like any other, so an ear that comes before one of them in the order of boot
//! takes a question in its place, and an ear that takes a question and asks it again wraps it.

mod bash;
mod child;
pub mod config;
pub(crate) mod disk;
pub(crate) mod files;
pub(crate) mod kits;
mod log;
mod operator;
mod provider;
mod reader;
mod scratch;
pub mod skills;
mod store;
mod time;

use std::{
  env, io,
  path::{Component, Path, PathBuf},
  process::{Command, Stdio},
};

pub use bash::{bash, program, shell};
pub use child::{Group, Unborn, spawn};
pub use files::{CONFIG, files};
pub(crate) use furb_wire::interruption;
pub use log::{log, logged};
pub(crate) use log::{logging, logging_on};
pub use operator::{Asked, CONSOLE, SHAPES, Shape, answered, answers, named};
pub use provider::{
  Advertised, BACKOFF, Backoff, Class, Hosted, Members, Model, Provider, Retries, Retry, Roster,
  Search, Shared, Thinking, Told, Writes,
  catalog::{self, Catalog},
  claude, codex, images,
  search::Searches,
};
pub(crate) use provider::{Mode, Systems};
pub use reader::{Retold, retold, retold_live, retold_with};
pub use scratch::{SCRATCH, Scratch};
pub(crate) use store::{
  After, Format, Last, STORE, appended, goes_after, last, stored, stored_after,
};
pub use store::{
  CHECKPOINT_SCHEMA, Lease, Lines, MADE, RECORD_SCHEMA, Schema, SchemaError, Sqlite, Store, Tip,
  crossings, engine, keeping, kept, made, readable, release, running, stood, store, texts,
  unopened,
};
pub use time::time;

use crate::{
  ear::{Co, say},
  fact::Fact,
  value::{Fault, Object, entry},
};
#[cfg(unix)]
use rustix::fs;

/// The file that a record keeps beside it, as `<record>.log`: the name of the record with the
/// suffix after it, byte for byte, so a name that is no text reaches its file.
pub(crate) fn beside(record: &Path, suffix: &str) -> PathBuf {
  let mut name = record.as_os_str().to_os_string();
  name.push(suffix);
  PathBuf::from(name)
}

/// The bytes that a process may still write on the disk that holds a folder, none when the disk
/// does not say.
#[cfg(unix)]
pub fn free(folder: &Path) -> Option<u64> {
  let stat = fs::statvfs(folder).ok()?;
  stat.f_bavail.checked_mul(stat.f_frsize)
}

#[cfg(not(unix))]
pub fn free(_: &Path) -> Option<u64> {
  None
}

/// The directory a chain stands in, which a path of it resolves against: where the chain went last,
/// resolved against the directory the life stands on, each read as a path of the machine where it
/// names the home of the user with `~`, as the World writes it from `home`.
pub(crate) async fn here<W: 'static>(
  co: &mut Co<W>,
  on: &str,
  home: Option<&Path>,
) -> Result<PathBuf, Fault> {
  let on = vec![("on", Object::string(on))];
  let cwd = co.call("cwd", vec![], on).await?;
  let standing = co.call("standing", vec![], vec![]).await?;
  let directory = entry(&standing.as_ref(), 1).and_then(|one| one.as_str().map(str::to_owned));
  let cwd = cwd.as_ref().as_str().map(str::to_owned).unwrap_or_default();
  let directory = files::placed(Path::new(""), &directory.unwrap_or_default(), home)?;
  files::placed(&directory, &cwd, home)
}

/// An act that an earlier life took and did not end, taken and closed with a refusal that names it
/// and says it was interrupted, as a carry ends an act that it cuts, since its work may have
/// run: the model reads that end, and asks again if it wants.
pub(crate) async fn interrupted<W: 'static>(co: &mut Co<W>, a: &Fact) -> Result<(), Fault> {
  say(co, Fact::says("started", a.about(), []));
  let id = vec![("id", Object::string(a.about()))];
  co.call("close", vec![Fault::refused(interruption(a.about())).object()], id).await.map(drop)
}

/// A path resolved against a directory, as a path of the machine: the directory with the path after
/// it, or the path itself when it is absolute, with every `.` and `..` read.
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

/// The cache of furb on this machine: under `XDG_CACHE_HOME`, `LOCALAPPDATA` on Windows,
/// `Library/Caches` of the home on macOS, and `.cache` of the home elsewhere. `machine` of
/// `crates/xtask/src/caches.rs` holds the same path, since xtask does not build this crate: a
/// change of one changes the other.
pub fn cache() -> Option<PathBuf> {
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

/// An address opened in the default browser of this machine, by the program that `BROWSER` names
/// when it names one, as the tools of a terminal agree, else by the opener of its system. The
/// opener runs apart: the call ends once it started.
pub fn browse(address: &str) -> io::Result<()> {
  let named = env::var_os("BROWSER").filter(|one| !one.is_empty());
  let mut command = if let Some(program) = named {
    Command::new(program)
  } else if cfg!(target_os = "macos") {
    Command::new("open")
  } else if cfg!(windows) {
    let mut command = Command::new("rundll32");
    command.arg("url.dll,FileProtocolHandler");
    command
  } else {
    Command::new("xdg-open")
  };
  command.arg(address).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
  command.spawn().map(drop)
}
