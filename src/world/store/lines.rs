//! The store of a record as a file of lines: one fact per line, in JSON.

use std::{
  fs::{File, OpenOptions},
  io::{self, BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
};

use memchr::{memchr, memrchr_iter};
use serde_json::{Value, value::RawValue};

use crate::{
  value::{Fault, Object, ObjectRef},
  wire::record,
};

use super::{
  Last, Lease, Store, Taken, entry_of,
  error::{At, StoreError, Unread},
  leased, now,
};
use std::str::{self, Utf8Error};

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
    let path = path.as_ref();
    Self::under(path, leased(path)?)
  }

  /// The file of lines at a path, under a lease of it that the caller holds.
  pub(crate) fn under(path: impl AsRef<Path>, lease: Lease) -> Result<Self, Fault> {
    let path = path.as_ref().to_path_buf();
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
  /// The last entries of the record, as [`last`] gives them. A record only grows, and a store
  /// cuts a torn line only as it writes after it, so a file that still ends where a read with no
  /// lease ended holds what that read gave: the store takes those entries and reads no line.
  pub(super) fn last(
    &mut self,
    read: Option<Last>,
    reached: &mut dyn FnMut(u64),
  ) -> Result<Last, Fault> {
    let size = self.file.metadata().map_err(StoreError::io(&self.path))?.len();
    let held = match read {
      Some(read) if read.end == size => read,
      _ => last(&self.path, reached)?,
    };
    self.ends(held.tail);
    Ok(held)
  }

  /// How the last line of the record ends, which the first keep mends.
  pub(super) fn ends(&mut self, tail: Tail) {
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
    let tail = parsing(&self.path, 0, &mut |one, _| entries.push(one))?;
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
  parsing(path, 0, &mut |one, _| take(one)).map(drop)
}

/// Each entry of the file of lines at a path, as [`each`] gives them, but the fact of each line
/// that starts with a head, which is given as the text of its JSON, with no parse into a value.
/// Each entry comes with the bytes of the file that the read passed.
pub(super) fn each_but(
  path: &Path,
  head: &[u8],
  take: &mut dyn FnMut(Taken, u64),
) -> Result<(), StoreError> {
  read(path, 0, Some(head), take).map(drop)
}

/// Each entry of the file of lines at a path, with the byte where the last of them ends and how the
/// last line ends. The bytes of the file that the read passed are told after each line.
pub(super) fn last(path: &Path, reached: &mut dyn FnMut(u64)) -> Result<Last, StoreError> {
  let mut last = Last::default();
  let mut take = |one, at| {
    last.take(one);
    last.end = at;
    reached(at);
  };
  let tail = parsing(path, 0, &mut take)?;
  Ok(Last { tail, ..last })
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
    let Some(read) = block.get_mut(..size) else { break };
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
/// so, and an error when that line is no entry.
pub(super) fn before(path: &Path, head: &[u8], to: u64) -> Result<Option<Object>, StoreError> {
  let Some(at) = start(path, head, to)? else { return Ok(None) };
  let byte = At::Byte(usize::try_from(at).unwrap_or(usize::MAX));
  let invalid = |why| StoreError::Invalid { path: path.to_path_buf(), at: byte, why };
  parsed(&line(path, at)?).map(Some).map_err(invalid)
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
/// most [`HEAD`] bytes of each, and the entry of each line that `wanted` says to parse. No other
/// line is parsed or held, and a blank line is none. A wanted line that is no entry is an error,
/// but the last line when it does not end, which a write cut, as [`read`] leaves it.
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
  // was given, its bytes when it is wanted whole, and the byte where it starts and the byte that
  // the read stands at.
  let (mut head, mut blank, mut given, mut whole) = (Vec::new(), true, false, None::<Vec<u8>>);
  let (mut start, mut at) = (0, 0);
  loop {
    let read = lines.fill_buf().map_err(StoreError::io(path))?;
    let (end, last) = (memchr(b'\n', read), read.is_empty());
    let part = end.and_then(|end| read.get(..end)).unwrap_or(read);
    blank = blank && part.iter().all(u8::is_ascii_whitespace);
    head.extend(part.iter().take(HEAD.saturating_sub(head.len())));
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
    at += passed;
    if over {
      match whole.take().map(|line| parsed(&line)) {
        Some(Ok(entry)) => take(entry),
        Some(Err(why)) if end.is_some() => {
          return Err(StoreError::Invalid { path: path.to_path_buf(), at: At::Byte(start), why });
        }
        Some(Err(_)) | None => {}
      }
      (blank, given, start) = (true, false, at);
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
  let whole = |cut: Utf8Error| str::from_utf8(line.get(..cut.valid_up_to()).unwrap_or_default());
  let text = str::from_utf8(line).or_else(whole).ok()?;
  let mut rest = text.strip_prefix("[[\"")?;
  let mut words = [""; 3];
  for (at, word) in words.iter_mut().enumerate() {
    let (one, after) = rest.split_once('"')?;
    (*word, rest) = (one, if at < 2 { after.strip_prefix(",\"")? } else { after });
  }
  words.iter().all(|one| !one.contains('\\')).then_some(words)
}

/// The entry of a line of a record, or why it is none.
fn parsed(line: &[u8]) -> Result<Object, Unread> {
  let held = serde_json::from_slice::<Vec<&RawValue>>(line).map_err(|_no| Unread::List)?;
  entry_of(untimed(&held)?)
}

/// The entry of a line: its fact, its place, whether the life rested and each word after them,
/// less the time that ends the line, or the line itself when it holds its fact alone, as an older
/// store kept it. A line of two holds a fact and its time, as a record of major 2 holds each, and
/// gives the fact alone. A line of three is no entry.
fn untimed<'a>(held: &'a [&'a RawValue]) -> Result<&'a [&'a RawValue], Unread> {
  match held {
    [_] => Ok(held),
    [entry @ .., time] if held.len() != 3 => {
      serde_json::from_str::<u64>(time.get()).map(|_| entry).map_err(|_no| Unread::Word)
    }
    _ => Err(Unread::Words(held.len())),
  }
}

/// The line of an entry: its fact, its place and whether the life rested, and after them the time
/// at which the store kept it, in milliseconds since the start of 1970, so the fact stays as the
/// life said it.
fn stamped(mut entry: Value, at: u64) -> String {
  if let Value::Array(held) = &mut entry {
    held.push(Value::from(at));
  }
  format!("{entry}\n")
}

/// How the last line of a record ends, which a store mends at its first keep.
#[derive(Clone, Copy, Default)]
pub(super) enum Tail {
  /// Each line ends, or the record holds none.
  #[default]
  Ended,
  /// The last line was torn as it was written, after the length that the record held whole: nothing after it was
  /// kept, so it goes.
  Torn(u64),
  /// The last line is whole and not ended.
  Unended,
}

/// Each entry of a record, in order, each the fact it kept, its place and whether the life rested,
/// in a list, as boot takes them, read one line at a time and changed in nothing, and how its last
/// line ends. The time that ends a line stays out of the entry, as [`untimed`] says. Each entry
/// comes with the bytes of the file that the read passed. The read starts at a byte, where a line
/// starts. A line that starts with the head of the unparsed, when one is given, gives the text of
/// its fact.
fn read(
  path: &Path,
  from: u64,
  unparsed: Option<&[u8]>,
  take: &mut dyn FnMut(Taken, u64),
) -> Result<Tail, StoreError> {
  let file = match File::open(path) {
    Ok(file) => file,
    Err(no) if no.kind() == ErrorKind::NotFound => return Ok(Tail::Ended),
    Err(no) => return Err(StoreError::io(path)(no)),
  };
  let invalid =
    |at: usize, why| StoreError::Invalid { path: path.to_path_buf(), at: At::Byte(at), why };
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
    let body = line.get(..read.saturating_sub(usize::from(ended))).unwrap_or_default();
    let Ok(text) = str::from_utf8(body) else {
      return Err(invalid(start, Unread::List));
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
      return Err(invalid(start, Unread::List));
    };
    let entry = untimed(&held).map_err(|why| invalid(start, why))?;
    if let Some(fact) =
      entry.first().filter(|_| unparsed.is_some_and(|head| text.as_bytes().starts_with(head)))
    {
      take(Taken::Fact(fact.get().to_owned()), next as u64);
    } else {
      take(Taken::Entry(entry_of(entry).map_err(|why| invalid(start, why))?), next as u64);
    }
    if !ended {
      return Ok(Tail::Unended);
    }
    start = next;
  }
  Ok(Tail::Ended)
}

/// Each entry of a record that starts at a byte of its file or after it, where a line starts, as
/// [`read`] gives them, and how the last line ends.
pub(super) fn after(path: &Path, from: u64) -> Result<(Vec<Object>, Tail), StoreError> {
  let mut entries = Vec::new();
  let tail = parsing(path, from, &mut |one, _| entries.push(one))?;
  Ok((entries, tail))
}

/// Each entry of a record from a byte, as [`read`] gives them with no head of the unparsed, so
/// each line comes parsed.
fn parsing(path: &Path, from: u64, take: &mut dyn FnMut(Object, u64)) -> Result<Tail, StoreError> {
  read(path, from, None, &mut |one, at| {
    if let Taken::Entry(one) = one {
      take(one, at);
    }
  })
}

#[cfg(test)]
#[path = "lines.test.rs"]
mod tests;
