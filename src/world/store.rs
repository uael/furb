//! The store: the record of a life on the disk, one fact per line, which one process at a time owns by a lease.

use std::{
  fs::{File, OpenOptions, TryLockError, create_dir_all},
  io::{ErrorKind, Write},
  path::{Path, PathBuf},
};

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, value::RawValue};

use crate::{
  ear::{Ear, ear, hear},
  value::{Fault, Object},
  wire::{decoded, inward, record},
};

/// The record at a path, read under its lease, and the ear that keeps on it what the journal says to keep.
///
/// The lease is a lock on the file beside the record, `<record>.lock`, which the store holds until its ear is gone
/// or the process ends; the kernel ends the lease of a process that ends. The lock file never moves and is never
/// removed, so its name always means the one file that every process locks. A final line that a crash tore is
/// removed, and a final line that is whole but not ended is ended, before the store keeps anything after it. Each
/// line holds the entry and, after the fact, the time at which the store kept it, which the life never reads.
pub fn store(path: impl AsRef<Path>) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let path = path.as_ref().to_path_buf();
  let lease = leased(&path)?;
  let entries = read(&path, true)?;
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", path.display()));
  let mut file = OpenOptions::new().create(true).append(true).open(&path).map_err(failed)?;
  let kept = ear(move |mut co| async move {
    let _lease = lease;
    loop {
      let a = hear(&mut co).await;
      if a.kind() != "keep" {
        continue;
      }
      let Some(entry) = a.word(0) else { continue };
      let line = stamped(record(entry), now());
      file
        .write_all(line.as_bytes())
        .and_then(|()| file.sync_data())
        .map_err(|no| Fault::refused(no.to_string()))?;
    }
  });
  Ok((entries, kept))
}

/// The line of an entry: its fact, and after it the time at which the store kept it, in milliseconds since the start
/// of 1970, so the fact stays as the journal said it.
fn stamped(mut entry: Value, at: u64) -> String {
  if let Value::Array(held) = &mut entry {
    held.push(Value::from(at));
  }
  format!("{entry}\n")
}

/// The time now, in milliseconds since the start of 1970.
fn now() -> u64 {
  let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
}

/// What the store kept at a path, read with no lease and changed in nothing, as a later life would open on it.
pub fn kept(path: impl AsRef<Path>) -> Result<Vec<Object>, Fault> {
  read(path.as_ref(), false)
}

/// The entries of a record, each the fact it kept in a list of one, as boot takes them. The time that a line holds
/// after its fact stays out of the entry, and a line with no time, as an older store kept it, reads the same.
fn read(path: &Path, repair: bool) -> Result<Vec<Object>, Fault> {
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", path.display()));
  let data = match std::fs::read(path) {
    Ok(data) => data,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
    Err(no) => return Err(failed(no)),
  };
  let invalid =
    |at: usize| Fault::refused(format!("Invalid record entry at byte {at} in {}.", path.display()));
  let mut entries = Vec::new();
  let mut start = 0;
  while start < data.len() {
    let end = data[start..].iter().position(|one| *one == b'\n').map(|at| start + at);
    let line = &data[start..end.unwrap_or(data.len())];
    let next = end.map_or(data.len(), |at| at + 1);
    let text = std::str::from_utf8(line).map_err(|_| invalid(start))?;
    if text.trim().is_empty() {
      start = next;
      continue;
    }
    let parsed = serde_json::from_str::<Vec<&RawValue>>(text);
    let Ok(held) = parsed else {
      if end.is_none() {
        // The last line was torn as it was written: nothing after it was kept, so it goes.
        if repair {
          let file = OpenOptions::new().write(true).open(path).map_err(failed)?;
          file.set_len(start as u64).map_err(failed)?;
        }
        break;
      }
      return Err(invalid(start));
    };
    let timed = |one: &RawValue| serde_json::from_str::<u64>(one.get()).is_ok();
    if held.is_empty() || held.len() > 2 || !held[1..].iter().all(|one| timed(one)) {
      return Err(invalid(start));
    }
    let whole = format!("[{}]", held[0].get());
    let raw = serde_json::from_str::<&RawValue>(&whole).map_err(|_| invalid(start))?;
    let entry =
      inward(&decoded(raw, 0).map_err(|_| invalid(start))?).map_err(|_| invalid(start))?;
    let fact = entry.as_ref().items().and_then(|held| held.first().and_then(|one| one.items()));
    let holds = entry.as_ref().items().is_some_and(|held| held.len() == 1);
    if !holds || !fact.is_some_and(|fact| fact.len() >= 3 && fact[0].as_str().is_some()) {
      return Err(invalid(start));
    }
    entries.push(entry);
    if end.is_none() && repair {
      let mut file = OpenOptions::new().append(true).open(path).map_err(failed)?;
      file.write_all(b"\n").map_err(failed)?;
    }
    start = next;
  }
  Ok(entries)
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

/// The lease of the record at a path, or the refusal when another process holds it.
///
/// The lease is the lock on the file at the path, and nothing checks which file that is. A rename of the file would
/// leave a lock that no later process meets, and on macOS a lookup of a renamed path can go on giving the moved file
/// to every process for seconds, so no holder moves or removes the lock file.
fn leased(path: &Path) -> Result<Lease, Fault> {
  let lock = PathBuf::from(format!("{}.lock", path.display()));
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", lock.display()));
  if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
    create_dir_all(parent).map_err(failed)?;
  }
  let file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .truncate(false)
    .open(&lock)
    .map_err(failed)?;
  match file.try_lock() {
    Ok(()) => Ok(Lease(file)),
    Err(TryLockError::WouldBlock) => {
      Err(Fault::refused(format!("Another process owns {}.", path.display())))
    }
    Err(TryLockError::Error(no)) => Err(failed(no)),
  }
}

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
