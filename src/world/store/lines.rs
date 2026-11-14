//! The store of a record as a file of lines: one fact per line, in JSON.

use std::{
  fs::{File, OpenOptions},
  io::{self, BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
};

use memchr::{memchr, memrchr_iter};
use serde_json::{Value, value::RawValue};

use crate::{
  checkpoint::CHECKPOINT,
  value::{Fault, Object, ObjectRef},
  wire::record,
};

use super::{
  Lease, Store, entry_of,
  error::{At, StoreError},
  leased, now,
};
use std::str;

/// The record at a path as a file of lines, under its lease.
///
/// A final line that a crash tore is removed, and a final line that is whole but not ended is ended, at the first
/// keep, so a store that keeps nothing leaves the record as it was. Each line holds the entry and, after the fact, the time at which the store kept
/// it, which the life never reads. The store writes each line at once, and syncs the data of the file only at a sync
/// that follows a line. A line that the store fails to write whole is cut from the file before the next line.
pub struct Lines {
  path: PathBuf,
  file: File,
  /// Whether the store wrote a line since its last sync.
  unsynced: bool,
  /// The length of the file after the last line that the store wrote whole, once it wrote one or failed to.
  whole: Option<u64>,
  /// Whether a write failed after that length, so the file can hold a part of a line past it.
  torn: bool,
  /// Whether the last line of the file is whole but not ended, so the next line starts with an end of line.
  unended: bool,
  _lease: Lease,
}

impl Lines {
  /// The file of lines at a path, under its lease, or the refusal when another process holds it.
  pub fn open(path: impl AsRef<Path>) -> Result<Self, Fault> {
    let path = path.as_ref().to_path_buf();
    let lease = leased(&path)?;
    // Not an append handle: on Windows it cannot cut a torn line, and the lease gives one writer.
    let mut file = OpenOptions::new()
      .create(true)
      .truncate(false)
      .write(true)
      .open(&path)
      .map_err(StoreError::io(&path))?;
    file.seek(SeekFrom::End(0)).map_err(StoreError::io(&path))?;
    Ok(Self {
      path,
      file,
      unsynced: false,
      whole: None,
      torn: false,
      unended: false,
      _lease: lease,
    })
  }
}

impl Lines {
  /// The last entries of the record and the byte where they start, as [`last`] gives them.
  pub(super) fn last(&mut self, reached: &mut dyn FnMut(u64)) -> Result<(u64, Vec<Object>), Fault> {
    let (from, entries, tail) = last(&self.path, reached)?;
    self.ends(tail);
    Ok((from, entries))
  }

  /// How the last line of the record ends, which the first keep mends.
  fn ends(&mut self, tail: Tail) {
    match tail {
      Tail::Torn(whole) => (self.whole, self.torn) = (Some(whole), true),
      Tail::Unended => self.unended = true,
      Tail::Ended => {}
    }
  }
}

impl Store for Lines {
  fn entries(&mut self) -> Result<Vec<Object>, Fault> {
    let mut entries = Vec::new();
    let tail = read(&self.path, 0, &mut |one| entries.push(one), &mut |_| {})?;
    self.ends(tail);
    Ok(entries)
  }

  fn keep(&mut self, entry: ObjectRef<'_>) -> Result<(), Fault> {
    let line = stamped(record(entry), now());
    let line = if self.unended { format!("\n{line}") } else { line };
    let whole = match self.whole {
      Some(whole) => whole,
      None => self.file.metadata().map_err(StoreError::io(&self.path))?.len(),
    };
    self.whole = Some(whole);
    // A write that failed can leave a part of its line, which goes before any line after it.
    if self.torn {
      self.file.set_len(whole).map_err(StoreError::io(&self.path))?;
      self.file.seek(SeekFrom::Start(whole)).map_err(StoreError::io(&self.path))?;
      self.torn = false;
    }
    self.unsynced = true;
    if let Err(no) = self.file.write_all(line.as_bytes()) {
      self.torn = true;
      return Err(StoreError::io(&self.path)(no).into());
    }
    self.whole = Some(whole + line.len() as u64);
    self.unended = false;
    Ok(())
  }

  fn sync(&mut self) -> Result<(), Fault> {
    if self.unsynced {
      self.file.sync_data().map_err(StoreError::io(&self.path))?;
      self.unsynced = false;
    }
    Ok(())
  }

  fn room(&self) -> Option<(&Path, u64)> {
    super::room(&self.path)
  }

  fn cut(&self) -> Option<u64> {
    let whole = self.whole.filter(|_| self.torn)?;
    Some(self.file.metadata().ok()?.len().saturating_sub(whole))
  }
}

/// Each entry of the file of lines at a path, in order, read with no lease and changed in nothing, one line at a time.
pub(super) fn each(path: &Path, take: &mut dyn FnMut(Object)) -> Result<(), StoreError> {
  read(path, 0, take, &mut |_| {}).map(drop)
}

/// The entries of the file of lines at a path that a life opens on: those from its last checkpoint
/// on, or each entry of a record that holds none, with the byte where they start and how the last
/// line ends. The read finds that checkpoint from the end of the file, so it passes and parses no
/// line before it. The bytes of the file that the read passed are told after each line.
pub(super) fn last(
  path: &Path,
  reached: &mut dyn FnMut(u64),
) -> Result<(u64, Vec<Object>, Tail), StoreError> {
  let (head, mut before) = (format!("[[\"{CHECKPOINT}\","), u64::MAX);
  loop {
    let from = start(path, head.as_bytes(), before)?.unwrap_or(0);
    let mut entries = Vec::new();
    let tail = read(path, from, &mut |one| entries.push(one), reached)?;
    // A last line that a stop tore as the store wrote a checkpoint is no entry, so the record
    // ends at the checkpoint before it.
    if from == 0 || !entries.is_empty() {
      return Ok((from, entries, tail));
    }
    before = from;
  }
}

/// Where the last line that starts with a head starts, of the lines that start before a byte. The
/// read goes from the end of the file to that line, a block at a time, and no further.
fn start(path: &Path, head: &[u8], before: u64) -> Result<Option<u64>, StoreError> {
  let mut file = match File::open(path) {
    Ok(file) => file,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(None),
    Err(no) => return Err(StoreError::io(path)(no)),
  };
  let mut end = file.metadata().map_err(StoreError::io(path))?.len().min(before);
  let mut block = vec![0; 1 << 20];
  while end > 0 {
    let size = block.len().min(usize::try_from(end).unwrap_or(usize::MAX));
    let from = end - size as u64;
    let read = &mut block[..size];
    file.seek(SeekFrom::Start(from)).map_err(StoreError::io(path))?;
    file.read_exact(read).map_err(StoreError::io(path))?;
    // A line starts after each end of line, and the line at byte 0 after no end of line.
    let ends = memrchr_iter(b'\n', read).map(|at| at + 1).chain((from == 0).then_some(0));
    for at in ends {
      let line = from + at as u64;
      let starts = match read.get(at..at + head.len()) {
        Some(start) => start == head,
        None => starts(&mut file, line, head).map_err(StoreError::io(path))?,
      };
      if starts && line < before {
        return Ok(Some(line));
      }
    }
    end = from;
  }
  Ok(None)
}

/// The entry of the last line that starts with a head, of the lines that start before a byte,
/// which [`start`] finds from that byte toward the start of the file: nothing when no line starts
/// so, or when that line does not parse.
pub(super) fn before(path: &Path, head: &[u8], to: u64) -> Result<Option<Object>, StoreError> {
  let Some(at) = start(path, head, to)? else { return Ok(None) };
  Ok(parsed(&line(path, at)?))
}

/// Whether the last line of the file of lines at a path starts with a head and is no entry: a stop
/// tore it as the store wrote it, so it has no end and does not parse.
pub(super) fn torn(path: &Path, head: &[u8]) -> Result<bool, StoreError> {
  let Some(at) = start(path, head, u64::MAX)? else { return Ok(false) };
  let line = line(path, at)?;
  Ok(!line.ends_with(b"\n") && parsed(&line).is_none())
}

/// The line of a file that starts at a byte, with its end when it has one.
fn line(path: &Path, at: u64) -> Result<Vec<u8>, StoreError> {
  let mut lines = BufReader::new(File::open(path).map_err(StoreError::io(path))?);
  lines.seek(SeekFrom::Start(at)).map_err(StoreError::io(path))?;
  let mut line = Vec::new();
  lines.read_until(b'\n', &mut line).map_err(StoreError::io(path))?;
  Ok(line)
}

/// Whether the bytes of a file at a place are those of a head.
fn starts(file: &mut File, at: u64, head: &[u8]) -> io::Result<bool> {
  let mut read = Vec::with_capacity(head.len());
  file.seek(SeekFrom::Start(at))?;
  file.take(head.len() as u64).read_to_end(&mut read)?;
  Ok(read == head)
}

/// How many bytes of the start of a line [`heads`] gives.
const HEAD: usize = 256;

/// The start of each line of the file of lines at a path that starts before a byte, in order, at
/// most [`HEAD`] bytes of each, and the entry of each line that `wanted` says to parse, when it
/// parses. No other line is parsed or held, and a blank line is none.
pub(super) fn heads(
  path: &Path,
  to: u64,
  wanted: &mut dyn FnMut(&[u8]) -> bool,
  take: &mut dyn FnMut(Object),
) -> Result<(), StoreError> {
  let file = match File::open(path) {
    Ok(file) => file,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(()),
    Err(no) => return Err(StoreError::io(path)(no)),
  };
  let mut lines = BufReader::with_capacity(1 << 20, file.take(to));
  // The line that the read stands in: its start, whether it is blank so far, whether its start
  // was given, and its bytes when it is wanted whole.
  let (mut head, mut blank, mut given, mut whole) = (Vec::new(), true, false, None::<Vec<u8>>);
  loop {
    let read = lines.fill_buf().map_err(StoreError::io(path))?;
    let (end, last) = (memchr(b'\n', read), read.is_empty());
    let part = &read[..end.unwrap_or(read.len())];
    blank = blank && part.iter().all(u8::is_ascii_whitespace);
    head.extend_from_slice(&part[..part.len().min(HEAD - head.len())]);
    let over = end.is_some() || last;
    if !given && !blank && (over || head.len() == HEAD) {
      given = true;
      whole = wanted(&head).then(Vec::new);
    }
    if let Some(line) = &mut whole {
      line.extend_from_slice(part);
    }
    let passed = end.map_or(read.len(), |at| at + 1);
    lines.consume(passed);
    if over {
      if let Some(entry) = whole.take().and_then(|line| parsed(&line)) {
        take(entry);
      }
      (blank, given) = (true, false);
      head.clear();
    }
    if last {
      return Ok(());
    }
  }
}

/// The kind, the name and the speaker of the fact of a line of a record, read from the start of the
/// line alone: nothing when the start ends before them, or when one of them holds an escape, as no
/// name of an act does.
pub(crate) fn spoken(line: &[u8]) -> Option<[&str; 3]> {
  let text = str::from_utf8(line).or_else(|cut| str::from_utf8(&line[..cut.valid_up_to()])).ok()?;
  let mut rest = text.strip_prefix("[[\"")?;
  let mut words = [""; 3];
  for (at, word) in words.iter_mut().enumerate() {
    let (one, after) = rest.split_once('"')?;
    (*word, rest) = (one, if at < 2 { after.strip_prefix(",\"")? } else { after });
  }
  words.iter().all(|one| !one.contains('\\')).then_some(words)
}

/// The entry of a line of a record, and nothing for a line that does not parse.
fn parsed(line: &[u8]) -> Option<Object> {
  serde_json::from_slice::<Vec<&RawValue>>(line).ok()?.first().and_then(|one| entry_of(one))
}

/// The line of an entry: its fact, and after it the time at which the store kept it, in milliseconds since the start
/// of 1970, so the fact stays as the journal said it.
fn stamped(mut entry: Value, at: u64) -> String {
  if let Value::Array(held) = &mut entry {
    held.push(Value::from(at));
  }
  format!("{entry}\n")
}

/// How the last line of a record ends, which a store mends at its first keep.
#[derive(Clone, Copy)]
pub(super) enum Tail {
  /// Each line ends, or the record holds none.
  Ended,
  /// The last line was torn as it was written, after the length that the record held whole: nothing after it was
  /// kept, so it goes.
  Torn(u64),
  /// The last line is whole and not ended.
  Unended,
}

/// Each entry of a record, in order, each the fact it kept in a list of one, as boot takes them, read one line at a
/// time and changed in nothing, and how its last line ends. The time that a line holds after its fact stays out of
/// the entry, and a line with no time, as an older store kept it, reads the same. The bytes of the
/// file that the read passed are told after each line. The read starts at a byte, where a line
/// starts.
fn read(
  path: &Path,
  from: u64,
  take: &mut dyn FnMut(Object),
  reached: &mut dyn FnMut(u64),
) -> Result<Tail, StoreError> {
  let file = match File::open(path) {
    Ok(file) => file,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(Tail::Ended),
    Err(no) => return Err(StoreError::io(path)(no)),
  };
  let invalid = |at: usize| StoreError::Invalid { path: path.to_path_buf(), at: At::Byte(at) };
  let mut lines = BufReader::with_capacity(1 << 20, file);
  lines.seek(SeekFrom::Start(from)).map_err(StoreError::io(path))?;
  let (mut start, mut line) = (usize::try_from(from).unwrap_or(usize::MAX), Vec::new());
  loop {
    line.clear();
    let read = lines.read_until(b'\n', &mut line).map_err(StoreError::io(path))?;
    if read == 0 {
      break;
    }
    let ended = line.last() == Some(&b'\n');
    let next = start + read;
    let Ok(text) = str::from_utf8(&line[..read - usize::from(ended)]) else {
      return Err(invalid(start));
    };
    if text.trim().is_empty() {
      start = next;
      continue;
    }
    let parsed = serde_json::from_str::<Vec<&RawValue>>(text);
    let Ok(held) = parsed else {
      if !ended {
        return Ok(Tail::Torn(start as u64));
      }
      return Err(invalid(start));
    };
    let timed = |one: &RawValue| serde_json::from_str::<u64>(one.get()).is_ok();
    if held.is_empty() || held.len() > 2 || !held[1..].iter().all(|one| timed(one)) {
      return Err(invalid(start));
    }
    take(entry_of(held[0]).ok_or_else(|| invalid(start))?);
    reached(next as u64);
    if !ended {
      return Ok(Tail::Unended);
    }
    start = next;
  }
  Ok(Tail::Ended)
}

#[cfg(test)]
#[path = "lines.test.rs"]
mod tests;
