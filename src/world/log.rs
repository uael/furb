//! The log of a life, `<record>.log`: every fact that the life says, in the order of the log, as an ear of a host
//! hears it, so a reader shows the life whole with no life. It holds what the record leaves out, as the rungs of each
//! ladder, each run, and each tell of a chain. It is a cache, which each life on the record writes again from its
//! first line, since a later life says every fact of its record again, before it says its own. A life writes it in
//! `<record>.log.new`, which takes the place of the log when the life keeps its first entry, after its replay, so a
//! life whose replay drifts leaves the log of the life before it as it was.
//!
//! Its first line is a map that holds under `entries` how many entries the record held when the life opened. Each
//! other line is one fact, as JSON reads a value of the sandbox, but for the line `["keep", "", n]`, which says that
//! the record keeps its entry `n`, counted from 1, there. An act that no fact showed before, since an ear of the
//! engine took it, stands on the line before the first fact about it. The fact of a module stands with no words,
//! since a module is no plain value and differs from one life to the next. The ear writes no fsync of the log. A line
//! that it cannot write, as on a full disk, waits in memory with each line after it, and the ear writes them in order
//! at its next line, after it cut what the failed write left, so the log lacks no fact once the disk has room.
//!
//! A later life makes again no act of its record that an ear of the outside made, unless that ear makes it again, and
//! says no fact about one that it does not make. So when the life keeps its first entry, the log writes each entry of
//! the record about such an act that no line shows, in the order of the record, and the log still holds every act of
//! its record.

use std::{
  cell::Cell,
  collections::HashSet,
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
  rc::Rc,
  task::{Context, Poll},
};

use serde_json::Value;

use crate::{
  ear::{Co, Ear, Heard, Spoken, Step, ear, hear},
  engine::Life,
  fact::{Fact, named},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::kept,
};

/// The start of the line that says the record kept an entry, as the ear writes it.
const KEEP: &str = r#"["keep","","#;

/// The ear of the log of the record at a path, which held a number of entries when the life opened: it hears every
/// fact, and writes the log again from its first line.
pub fn log(record: impl AsRef<Path>, held: &[Object]) -> Box<dyn Ear> {
  heard(Log::open(record.as_ref(), held))
}

/// The ear of the log of the record at a path, for a life that opens from the snapshot of an earlier life on the
/// record: it writes the log on after the lines that it holds, since this life says none of them again.
pub(crate) fn continued(record: impl AsRef<Path>, entries: usize) -> Box<dyn Ear> {
  heard(Log::continued(record.as_ref(), entries))
}

/// The ear that writes a log.
fn heard(mut log: Log) -> Box<dyn Ear> {
  let behind = Rc::clone(&log.behind);
  let ear = ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      match a.kind() {
        "keep" => log.kept(),
        _ => log.heard(&mut co, &a).await,
      }
    }
  });
  Box::new(Logging { ear, behind })
}

/// The ear that writes a log, which says when lines of the log wait.
struct Logging {
  ear: Box<dyn Ear>,
  behind: Rc<Cell<bool>>,
}

impl Ear for Logging {
  fn resume(&mut self, heard: Heard, cx: &mut Context<'_>) -> Step {
    self.ear.resume(heard, cx)
  }

  fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Spoken> {
    self.ear.poll(cx)
  }

  fn answered(&mut self, got: Result<Object, Fault>) {
    self.ear.answered(got);
  }

  fn lent(&mut self, heard: Heard, cx: &mut Context<'_>, life: Life) -> (Step, Life) {
    self.ear.lent(heard, cx, life)
  }

  fn behind(&self) -> bool {
    self.behind.get()
  }
}

/// The log file, and what the ear knows of it.
struct Log {
  path: PathBuf,
  file: Option<File>,
  /// The file that a life writes the log again in, beside the log, until it keeps its first entry.
  fresh: Option<PathBuf>,
  /// How many entries the record holds.
  entries: usize,
  /// Each name that a line of the log shows as an act, or that names no act.
  seen: HashSet<String>,
  /// The entries of the record about each act that an ear of the outside made, by the name of that act, in the order
  /// of the record, which the log writes when the life keeps its first entry and no line shows that act.
  owed: Vec<(String, Value)>,
  /// The lines that the ear could not write yet, in order.
  waiting: String,
  /// The length of the file after the last lines that the ear wrote whole.
  whole: u64,
  /// Whether a write failed after that length, so the file can hold a part of a line past it.
  torn: bool,
  /// Whether lines wait, which the ear tells the life.
  behind: Rc<Cell<bool>>,
}

/// Whether a name is the name of an act, as `bash3`, and not the name of an ear.
fn named_act(name: &str) -> bool {
  let kind = name.trim_end_matches(|one: char| one.is_ascii_digit());
  !kind.is_empty() && kind.len() < name.len()
}

/// The entries of a record about each act that an ear of the outside made, which a later life does not say again
/// unless that ear makes the act again.
fn owed(held: &[Object]) -> Vec<(String, Value)> {
  let mut theirs = HashSet::new();
  let mut owed = Vec::new();
  for entry in held {
    let Some(fact) = entry.as_ref().items().and_then(|one| Fact::of(*one.first()?)) else {
      continue;
    };
    if fact.question() && !named_act(fact.by()) && !matches!(fact.by(), "" | "operator" | "journal")
    {
      theirs.insert(fact.about().to_owned());
    }
    if theirs.contains(fact.about()) {
      owed.push((fact.about().to_owned(), outward(fact.0.as_ref())));
    }
  }
  owed
}

impl Log {
  /// The log of the record at a path, written again from its first line.
  fn open(record: &Path, held: &[Object]) -> Log {
    let path = PathBuf::from(format!("{}.log", record.display()));
    let fresh = PathBuf::from(format!("{}.log.new", record.display()));
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(&fresh).ok();
    let entries = held.len();
    let mut log = Log {
      path,
      file,
      fresh: Some(fresh),
      entries,
      seen: HashSet::new(),
      owed: owed(held),
      waiting: String::new(),
      whole: 0,
      torn: false,
      behind: Rc::default(),
    };
    log.line(&serde_json::json!({ "entries": entries }));
    log
  }

  /// The log of the record at a path, written on after the lines that it holds, each name of which it has seen.
  fn continued(record: &Path, entries: usize) -> Log {
    let path = PathBuf::from(format!("{}.log", record.display()));
    let mut seen = HashSet::new();
    for line in std::fs::read_to_string(&path).unwrap_or_default().lines().skip(1) {
      let about = serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|one| one.get(1)?.as_str().map(str::to_owned));
      seen.extend(about.filter(|one| !one.is_empty()));
    }
    let file = OpenOptions::new().append(true).open(&path).ok();
    let whole = file.as_ref().and_then(|one| one.metadata().ok()).map_or(0, |one| one.len());
    Log {
      path,
      file,
      fresh: None,
      entries,
      seen,
      owed: Vec::new(),
      waiting: String::new(),
      whole,
      torn: false,
      behind: Rc::default(),
    }
  }

  /// A fact that the ear heard, after the act it is about when no line shows that act yet.
  async fn heard(&mut self, co: &mut Co, a: &Fact) {
    if self.file.is_none() {
      return;
    }
    let about = a.about();
    if a.question() {
      if self.seen.insert(about.to_owned()) {
        self.line(&outward(a.0.as_ref()));
      }
      return;
    }
    if !about.is_empty() && self.seen.insert(about.to_owned()) {
      let made = co.call("get", vec![Object::string(about)], vec![]).await;
      if let Ok(made) = made.as_ref().map(Object::as_ref)
        && made.type_name() != "NoneType"
      {
        self.line(&outward(made));
      }
    }
    match a.kind() {
      "module" => self.line(&serde_json::json!(["module", about, a.by()])),
      _ => self.line(&outward(a.0.as_ref())),
    }
  }

  /// The line that says the record kept its next entry.
  fn kept(&mut self) {
    let missed: HashSet<String> = self
      .owed
      .iter()
      .map(|(about, _)| about.clone())
      .filter(|one| !self.seen.contains(one))
      .collect();
    for (about, line) in std::mem::take(&mut self.owed) {
      if missed.contains(&about) {
        self.seen.insert(about);
        self.line(&line);
      }
    }
    if let Some(fresh) = self.fresh.take()
      && self.file.is_some()
      && std::fs::rename(&fresh, &self.path).is_err()
    {
      self.file = None;
      let _ = std::fs::remove_file(&fresh);
    }
    self.entries += 1;
    self.line(&serde_json::json!(["keep", "", self.entries]));
  }

  /// One line, written after each line that waits, or waiting with them when the ear cannot write it.
  fn line(&mut self, value: &Value) {
    if self.file.is_none() {
      return;
    }
    self.waiting.push_str(&format!("{value}\n"));
    let failed = self.flushed().is_err();
    self.behind.set(failed);
  }

  /// Each line that waits, written in order after what a failed write left is cut.
  fn flushed(&mut self) -> std::io::Result<()> {
    let Some(file) = self.file.as_mut() else { return Ok(()) };
    if self.torn {
      file.set_len(self.whole)?;
      file.seek(SeekFrom::Start(self.whole))?;
      self.torn = false;
    }
    if let Err(no) = file.write_all(self.waiting.as_bytes()) {
      self.torn = true;
      return Err(no);
    }
    self.whole += self.waiting.len() as u64;
    self.waiting.clear();
    Ok(())
  }
}

/// The facts of the log of the record at a path, read with no lease, when the log agrees with the record: each entry
/// that the record holds is one that the log counts. Nothing when the record has no log, or a log that does not agree,
/// as one that an older life left, one that a life that wrote no log made old, or one that a life whose replay drifted
/// left short of an act that the record holds. A last line that a crash tore is left out.
pub fn logged(path: impl AsRef<Path>) -> Result<Option<Vec<Object>>, Fault> {
  let path = path.as_ref();
  let record = kept(path)?;
  let asked: HashSet<String> = record
    .iter()
    .filter_map(|entry| Fact::of(*entry.as_ref().items()?.first()?))
    .filter(Fact::question)
    .map(|one| one.about().to_owned())
    .collect();
  if !agrees(path, record.len(), &asked) {
    return Ok(None);
  }
  let Ok(text) = std::fs::read_to_string(format!("{}.log", path.display())) else {
    return Ok(None);
  };
  let facts = text
    .lines()
    .skip(1)
    .filter(|line| !line.starts_with(KEEP))
    .map_while(|line| parsed(line).ok())
    .collect();
  Ok(Some(facts))
}

/// Whether the log of the record at a path agrees with a record of that many entries and of those questions, as
/// [`logged`] reads it, which the caller read once.
pub(crate) fn agrees(path: &Path, entries: usize, asked: &HashSet<String>) -> bool {
  File::open(format!("{}.log", path.display()))
    .is_ok_and(|file| agreed(BufReader::with_capacity(1 << 20, file), entries, asked.clone()))
}

/// Whether a log agrees with a record of that many entries and of those questions: its head counts the entries that
/// the record held, its lines of keep count the others in order, each line is JSON but a last one that a crash tore,
/// and it shows each question of the record. Each line is read from its head alone, one at a time, and no fact is
/// made.
fn agreed(mut lines: impl BufRead, record: usize, mut asked: HashSet<String>) -> bool {
  let mut line = String::new();
  let head = match lines.read_line(&mut line) {
    Ok(_) => serde_json::from_str::<Value>(line.trim_end_matches('\n')).ok(),
    Err(_) => return false,
  };
  let Some(mut entries) = head.and_then(|one| one.get("entries")?.as_u64()) else {
    return false;
  };
  loop {
    line.clear();
    match lines.read_line(&mut line) {
      Ok(0) => break,
      Ok(_) => {}
      Err(_) => return false,
    }
    let ended = line.ends_with('\n');
    let text =
      line.strip_suffix('\n').map(|one| one.strip_suffix('\r').unwrap_or(one)).unwrap_or(&line);
    if serde_json::from_str::<serde::de::IgnoredAny>(text).is_err() {
      if !ended {
        break;
      }
      return false;
    }
    if let Some(at) = text.strip_prefix(KEEP).and_then(|one| one.strip_suffix(']')) {
      if at.parse::<u64>().ok() != entries.checked_add(1) {
        return false;
      }
      entries += 1;
    } else if let Some(name) = question(text) {
      asked.remove(&name);
    }
  }
  asked.is_empty() && usize::try_from(entries).ok() == Some(record)
}

/// The name of the question that a line of a log shows, read from its first two words, as `["bash","bash3",`. A
/// line whose head holds an escape is parsed whole.
fn question(line: &str) -> Option<String> {
  let rest = line.strip_prefix("[\"")?;
  let (kind, rest) = rest.split_once('"')?;
  let about = rest.strip_prefix(",\"").and_then(|one| one.split_once('"')).map(|one| one.0);
  if kind.contains('\\') || about.is_some_and(|one| one.contains('\\')) {
    let fact = parsed(line).ok()?;
    let fact = Fact::of(fact.as_ref())?;
    return fact.question().then(|| fact.about().to_owned());
  }
  about.filter(|one| named(one, kind)).map(str::to_owned)
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
