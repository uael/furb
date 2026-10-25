//! The store of a record as a file of lines: one fact per line, in JSON.

use std::{
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, ErrorKind, Write},
  path::{Path, PathBuf},
};

use serde_json::{Value, value::RawValue};

use crate::{
  value::{Fault, Object, ObjectRef},
  wire::record,
};

use super::{Lease, Store, entry_of, failed, leased, now};

/// The record at a path as a file of lines, under its lease.
///
/// A final line that a crash tore is removed, and a final line that is whole but not ended is ended, before the
/// store keeps anything after it. Each line holds the entry and, after the fact, the time at which the store kept
/// it, which the life never reads. The store syncs the data of the file after each line.
pub struct Lines {
  path: PathBuf,
  file: File,
  _lease: Lease,
}

impl Lines {
  /// The file of lines at a path, under its lease, or the refusal when another process holds it.
  pub fn open(path: impl AsRef<Path>) -> Result<Self, Fault> {
    let path = path.as_ref().to_path_buf();
    let lease = leased(&path)?;
    let file =
      OpenOptions::new().create(true).append(true).open(&path).map_err(|no| failed(&path, no))?;
    Ok(Self { path, file, _lease: lease })
  }
}

impl Store for Lines {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    let mut entries = Vec::new();
    read(&self.path, true, &mut |one| entries.push(one))?;
    Ok(entries)
  }

  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault> {
    let line = stamped(record(entry), now());
    self
      .file
      .write_all(line.as_bytes())
      .and_then(|()| self.file.sync_data())
      .map_err(|no| failed(&self.path, no))
  }
}

/// What the file of lines at a path holds, read with no lease and changed in nothing.
pub(super) fn kept(path: &Path) -> Result<Vec<Object>, Fault> {
  let mut entries = Vec::new();
  read(path, false, &mut |one| entries.push(one))?;
  Ok(entries)
}

/// Each entry of the file of lines at a path, in order, read with no lease and changed in nothing, one line at a time.
pub(super) fn each(path: &Path, take: &mut dyn FnMut(Object)) -> Result<(), Fault> {
  read(path, false, take)
}

/// The entries of the file of lines at a path whose fact is of one of some kinds, as [`kept`] reads them: a line whose
/// head names another kind is not parsed, and a line that does not parse is passed over.
pub(super) fn heads(path: &Path, kinds: &[&str]) -> Result<Vec<Object>, Fault> {
  let data = match std::fs::read(path) {
    Ok(data) => data,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
    Err(no) => return Err(failed(path, no)),
  };
  let starts: Vec<String> = kinds.iter().map(|kind| format!("[[\"{kind}\",")).collect();
  let mut entries = Vec::new();
  let mut start = 0;
  while start < data.len() {
    let end = memchr::memchr(b'\n', &data[start..]).map_or(data.len(), |at| start + at);
    let line = &data[start..end];
    if starts.iter().any(|head| line.starts_with(head.as_bytes()))
      && let Ok(held) = serde_json::from_slice::<Vec<&RawValue>>(line)
      && let Some(entry) = held.first().and_then(|one| entry_of(one))
    {
      entries.push(entry);
    }
    start = end + 1;
  }
  Ok(entries)
}

/// The line of an entry: its fact, and after it the time at which the store kept it, in milliseconds since the start
/// of 1970, so the fact stays as the journal said it.
fn stamped(mut entry: Value, at: u64) -> String {
  if let Value::Array(held) = &mut entry {
    held.push(Value::from(at));
  }
  format!("{entry}\n")
}

/// Each entry of a record, in order, each the fact it kept in a list of one, as boot takes them, read one line at a
/// time. The time that a line holds after its fact stays out of the entry, and a line with no time, as an older store
/// kept it, reads the same.
fn read(path: &Path, repair: bool, take: &mut dyn FnMut(Object)) -> Result<(), Fault> {
  let file = match File::open(path) {
    Ok(file) => file,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(()),
    Err(no) => return Err(failed(path, no)),
  };
  let invalid =
    |at: usize| Fault::refused(format!("Invalid record entry at byte {at} in {}.", path.display()));
  let mut lines = BufReader::with_capacity(1 << 20, file);
  let (mut start, mut line) = (0, Vec::new());
  loop {
    line.clear();
    let read = lines.read_until(b'\n', &mut line).map_err(|no| failed(path, no))?;
    if read == 0 {
      break;
    }
    let ended = line.last() == Some(&b'\n');
    let next = start + read;
    let text =
      std::str::from_utf8(&line[..read - usize::from(ended)]).map_err(|_| invalid(start))?;
    if text.trim().is_empty() {
      start = next;
      continue;
    }
    let parsed = serde_json::from_str::<Vec<&RawValue>>(text);
    let Ok(held) = parsed else {
      if !ended {
        // The last line was torn as it was written: nothing after it was kept, so it goes.
        if repair {
          let file = OpenOptions::new().write(true).open(path).map_err(|no| failed(path, no))?;
          file.set_len(start as u64).map_err(|no| failed(path, no))?;
        }
        break;
      }
      return Err(invalid(start));
    };
    let timed = |one: &RawValue| serde_json::from_str::<u64>(one.get()).is_ok();
    if held.is_empty() || held.len() > 2 || !held[1..].iter().all(|one| timed(one)) {
      return Err(invalid(start));
    }
    take(entry_of(held[0]).ok_or_else(|| invalid(start))?);
    if !ended && repair {
      let mut file = OpenOptions::new().append(true).open(path).map_err(|no| failed(path, no))?;
      file.write_all(b"\n").map_err(|no| failed(path, no))?;
    }
    start = next;
  }
  Ok(())
}

#[cfg(test)]
#[path = "lines.test.rs"]
mod tests;
