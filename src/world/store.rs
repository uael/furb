//! The store: the record of a life on the disk, one fact per line, which one process at a time owns by a lease.

use std::{
  fs::{File, OpenOptions, TryLockError, create_dir_all},
  io::{ErrorKind, Write},
  path::{Path, PathBuf},
};

use same_file::Handle;
use serde_json::value::RawValue;

use crate::{
  ear::{Ear, ear, hear},
  value::{Fault, Object},
  wire::{decoded, inward, record},
};

/// The record at a path, read under its lease, and the ear that keeps on it what the journal says to keep.
///
/// The lease is a lock on the file beside the record, `<record>.lock`, which the store holds until its ear is gone
/// or the process ends; the kernel ends the lease of a process that ends. A final line that a crash tore is removed,
/// and a final line that is whole but not ended is ended, before the store keeps anything after it.
pub fn store(path: impl AsRef<Path>) -> Result<(Vec<Object>, Box<dyn Ear>), Fault> {
  let path = path.as_ref().to_path_buf();
  let lease = leased(&path)?;
  let entries = read(&path, true)?;
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", path.display()));
  let mut file = OpenOptions::new().create(true).append(true).open(&path).map_err(failed)?;
  let kept = ear(move |co, _| async move {
    let _lease = lease;
    loop {
      let a = hear(&co).await;
      if a.kind() != "keep" {
        continue;
      }
      let Some(entry) = a.word(0) else { continue };
      let line = format!("{}\n", record(entry));
      file
        .write_all(line.as_bytes())
        .and_then(|()| file.sync_data())
        .map_err(|no| Fault::refused(no.to_string()))?;
    }
  });
  Ok((entries, kept))
}

/// What the store kept at a path, read with no lease and changed in nothing, as a later life would open on it.
pub fn kept(path: impl AsRef<Path>) -> Result<Vec<Object>, Fault> {
  read(path.as_ref(), false)
}

/// The entries of a record, each the fact it kept in a list of one, as boot takes them.
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
    let parsed = serde_json::from_str::<&RawValue>(text);
    let Ok(raw) = parsed else {
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

/// The lease of the record at a path, or the refusal when another process holds it.
///
/// The lock is the lease only while its file is the one at the path. A holder that moved or removed the file after
/// this process opened it leaves a lock that no later process meets, so this process opens the path again. On
/// Windows the identity of a file holds only while a handle keeps the file open, as both handles do here.
fn leased(path: &Path) -> Result<Handle, Fault> {
  let lock = PathBuf::from(format!("{}.lock", path.display()));
  let failed = |no: std::io::Error| Fault::refused(format!("{no}: {}", lock.display()));
  if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
    create_dir_all(parent).map_err(failed)?;
  }
  loop {
    let file: File = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .open(&lock)
      .map_err(failed)?;
    match file.try_lock() {
      Ok(()) => {}
      Err(TryLockError::WouldBlock) => {
        return Err(Fault::refused(format!("Another process owns {}.", path.display())));
      }
      Err(TryLockError::Error(no)) => return Err(failed(no)),
    }
    let held = Handle::from_file(file).map_err(failed)?;
    match Handle::from_path(&lock) {
      Ok(now) if now == held => return Ok(held),
      Ok(_) => {}
      Err(no) if no.kind() == ErrorKind::NotFound => {}
      Err(no) => return Err(failed(no)),
    }
  }
}
