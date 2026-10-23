//! The store: the record of a life, which one process at a time owns by a lease. A record whose name ends in
//! `.sqlite`, `.sqlite3` or `.db` is a database of SQLite, and any other record is a file of lines, one fact per line.

mod lines;
mod sqlite;

use std::{
  fmt::Display,
  fs::{File, OpenOptions, TryLockError, create_dir_all},
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

use monty_types::unstable::{MontyGraph, MontyNode};

use crate::{
  ENGINE, KERNEL,
  ear::{Ear, hear, hearing, say},
  engine::ROOT,
  fact::Fact,
  value::{Fault, Object, ObjectRef},
  wire::{built, whole},
};

pub use lines::Lines;
pub use sqlite::Sqlite;

/// The place that keeps the record of a life, which a host can write for a place of its own and give to `keeping`.
///
/// A store owns its record from its opening to its end, by a lease, so no other store opens on the record while it
/// lives. A life reads the entries once, as it opens, before it keeps anything. Each store passes the suite in
/// `store.test.rs`.
pub trait Store {
  /// Each entry that the record holds, in the order that the store kept them, each the fact it kept in a list of
  /// one, as boot takes them.
  fn entries(&mut self) -> Result<Vec<Object>, Fault>;

  /// Keep one entry after each entry that the store kept before it. When keep returns, the entry outlives a crash of
  /// the process.
  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault>;
}

/// The record at a path, read under its lease, and the ear that keeps on it what the journal says to keep: a
/// database of SQLite when the name of the path ends in `.sqlite`, `.sqlite3` or `.db`, and else a file of lines.
pub fn store(path: impl AsRef<Path>) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let path = path.as_ref();
  match database(path) {
    true => keeping(Sqlite::open(path)?),
    false => keeping(Lines::open(path)?),
  }
}

/// What the store kept at a path, read with no lease and changed in nothing, as a later life would open on it.
pub fn kept(path: impl AsRef<Path>) -> Result<Vec<Object>, Fault> {
  let path = path.as_ref();
  match database(path) {
    true => sqlite::kept(path),
    false => lines::kept(path),
  }
}

/// Each entry that the store kept at a path, in order, read with no lease and changed in nothing, one at a time, so
/// a reader holds no more of a large record than it keeps of each entry.
pub fn each(path: impl AsRef<Path>, take: &mut dyn FnMut(Object)) -> Result<(), Fault> {
  let path = path.as_ref();
  match database(path) {
    true => sqlite::each(path, take),
    false => lines::each(path, take),
  }
}

/// The kind of the fact by which a record names the build that made it: `("made", "chain1", "store", engine,
/// version)`, where engine is what [`engine`] gives and version is [`VERSION`]. A record of a build before the
/// versions holds no version. No contract holds it, so this is the one home of its shape.
pub const MADE: &str = "made";

/// The version of this build, which a record names. A record replays on each build of the version that made it.
///
/// The gate of the corpus holds the versions few: each record of the corpus in `tests/corpus` replays with no drift
/// on each build of its version. A change that makes a record of the corpus drift adds one to the version, and
/// makes the corpus again in the same change, as `tests/corpus/README.md` says. The command line of each version puts
/// its program in the store of builds of the machine, where [`build`](super::build) says, so a build of a later
/// version hands a record of this one to it.
pub const VERSION: i64 = 1;

/// The engine of this build, which a record names: a digest of the engine and of the Kernel, which make each act of
/// a replay. Another engine can make other acts from the same record, so a record replays only on the engine that
/// made it.
pub fn engine() -> String {
  let mut digest = Sha256::new();
  for one in [ENGINE, KERNEL] {
    digest.update(one.as_bytes());
    digest.update([0]);
  }
  format!("{:x}", digest.finalize())
}

/// The engine that a record names, and nothing when it names none, as a record of a build before the fact made.
pub fn made(record: &[Object]) -> Option<String> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(0)?.as_str().map(str::to_owned)).flatten()
  })
}

/// The version of the build that a record names, and nothing when it names none, as a record of a build before
/// the versions.
pub fn version(record: &[Object]) -> Option<i64> {
  record.iter().find_map(|entry| {
    let fact = Fact::of(*entry.as_ref().items()?.first()?)?;
    (fact.kind() == MADE).then(|| fact.word(1)?.as_int()).flatten()
  })
}

/// The record that a store holds, and the ear that keeps in the store what the journal says to keep.
///
/// A new record names the engine and the version of this build: the ear says them as it hears the root at its birth,
/// so a later life says them again at the same place. A record of a build before the fact made names no engine, and
/// stays so, since this engine did not make it.
pub fn keeping(mut store: impl Store + 'static) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let entries = store.entries()?;
  let new = entries.is_empty();
  let kept = hearing(&["keep", "chain"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      match a.kind() {
        "keep" => {
          let Some(entry) = a.word(0) else { continue };
          store.keep(entry)?;
        }
        "chain" if new && a.about() == ROOT => {
          say(&mut co, Fact::says(MADE, ROOT, [Object::string(engine()), Object::int(VERSION)]));
        }
        _ => {}
      }
    }
  });
  Ok((entries, kept))
}

/// Whether the record at a path is a database of SQLite, which its name says.
pub(crate) fn database(path: &Path) -> bool {
  let named = path.extension().and_then(|one| one.to_str()).unwrap_or_default();
  ["sqlite", "sqlite3", "db"].iter().any(|one| named.eq_ignore_ascii_case(one))
}

/// The entry of one fact, from the JSON that the record holds of the fact, or nothing when it is no fact: a fact is
/// a list of three words at least, whose first word is its kind.
fn entry_of(fact: &RawValue) -> Option<Object> {
  let mut graph = MontyGraph::new();
  let fact = built(&mut graph, fact, 1).ok()?;
  let root = graph.push(MontyNode::List(vec![fact]));
  let entry = whole(graph, root).ok()?;
  let fact = entry.as_ref().items().and_then(|held| held.first().and_then(|one| one.items()));
  let holds = fact.is_some_and(|fact| fact.len() >= 3 && fact[0].as_str().is_some());
  holds.then_some(entry)
}

/// The refusal of what a store could not do at a path.
fn failed(path: &Path, no: impl Display) -> Fault {
  Fault::refused(format!("{no}: {}", path.display()))
}

/// The time now, in milliseconds since the start of 1970.
fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

/// The lease of a record: the lock on the file beside it, which ends when the lease goes. A process that this one
/// starts holds a copy of the file until its program runs, so the lease ends the lock itself, and not with the last
/// copy of the file.
struct Lease(File);

impl Drop for Lease {
  fn drop(&mut self) {
    let _ = self.0.unlock();
  }
}

/// The lease of the record at a path, or the refusal when another process holds it: the lock on the file beside
/// the record, `<record>.lock`, which every store of the crate takes, and which the kernel ends when the process ends.
///
/// The lease is the lock on the file at the path, and nothing checks which file that is. A rename of the file would
/// leave a lock that no later process meets, and on macOS a lookup of a renamed path can go on giving the moved file
/// to every process for seconds, so no holder moves or removes the lock file.
fn leased(path: &Path) -> Result<Lease, Fault> {
  let lock = PathBuf::from(format!("{}.lock", path.display()));
  if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
    create_dir_all(parent).map_err(|no| failed(&lock, no))?;
  }
  let file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open(&lock)
    .map_err(|no| failed(&lock, no))?;
  match file.try_lock() {
    Ok(()) => Ok(Lease(file)),
    Err(TryLockError::WouldBlock) => {
      Err(Fault::refused(format!("Another process owns {}.", path.display())))
    }
    Err(TryLockError::Error(no)) => Err(failed(&lock, no)),
  }
}

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
