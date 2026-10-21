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

use monty_types::unstable::{MontyGraph, MontyNode};

use crate::{
  ear::{Ear, hear, hearing},
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

/// The record that a store holds, and the ear that keeps in the store what the journal says to keep.
pub fn keeping(mut store: impl Store + 'static) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let entries = store.entries()?;
  let kept = hearing(&["keep"], move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() != "keep" {
        continue;
      }
      let Some(entry) = a.word(0) else { continue };
      store.keep(entry)?;
    }
  });
  Ok((entries, kept))
}

/// Whether the record at a path is a database of SQLite, which its name says.
fn database(path: &Path) -> bool {
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
