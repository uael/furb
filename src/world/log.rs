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
//! since a module is no plain value and differs from one life to the next. The ear writes no fsync of the log, and a
//! log that it cannot write it removes, so no reader takes a log that lacks a fact.
//!
//! A later life makes again no act of its record that an ear of the outside made, unless that ear makes it again, and
//! says no fact about one that it does not make. So when the life keeps its first entry, the log writes each entry of
//! the record about such an act that no line shows, in the order of the record, and the log still holds every act of
//! its record.

use std::{
  collections::HashSet,
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, Write},
  path::{Path, PathBuf},
};

use serde_json::{Value, value::RawValue};

use crate::{
  ear::{Co, Ear, ear, hear},
  fact::{Fact, named},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::store::each,
};

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
  ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      match a.kind() {
        "keep" => log.kept(),
        _ => log.heard(&mut co, &a).await,
      }
    }
  })
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
    let mut log =
      Log { path, file, fresh: Some(fresh), entries, seen: HashSet::new(), owed: owed(held) };
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
    Log { path, file, fresh: None, entries, seen, owed: Vec::new() }
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

  /// One line, or the end of the log when the ear cannot write it.
  fn line(&mut self, value: &Value) {
    let Some(file) = self.file.as_mut() else { return };
    if file.write_all(format!("{value}\n").as_bytes()).is_err() {
      self.file = None;
      let _ = std::fs::remove_file(self.fresh.as_ref().unwrap_or(&self.path));
    }
  }
}

/// The facts of the log of the record at a path, read with no lease, when the log agrees with the record: each entry
/// that the record holds is one that the log counts. Nothing when the record has no log, or a log that does not agree,
/// as one that an older life left, one that a life that wrote no log made old, or one that a life whose replay drifted
/// left short of an act that the record holds. A last line that a crash tore is left out.
pub fn logged(path: impl AsRef<Path>) -> Result<Option<Vec<Object>>, Fault> {
  let path = path.as_ref();
  let (entries, asked) = asked(path)?;
  let mut facts = Vec::new();
  Ok(walked(path, entries, asked, Some(&mut |one| facts.push(one)))?.then_some(facts))
}

/// How many entries the record at a path holds, and the name of each question among them, which a log that agrees
/// with the record shows each as a question.
pub(crate) fn asked(path: &Path) -> Result<(usize, HashSet<String>), Fault> {
  let (mut entries, mut asked) = (0, HashSet::new());
  each(path, &mut |entry| {
    entries += 1;
    let fact = entry.as_ref().items().and_then(|held| Fact::of(*held.first()?));
    if let Some(one) = fact.filter(Fact::question) {
      asked.insert(one.about().to_owned());
    }
  })?;
  Ok((entries, asked))
}

/// Each fact of the log of the record at a path, in order, as [`logged`] gives them, when the log agrees with a record
/// of that many entries and of those questions, which [`asked`] gives. It reads the log once to check it, then once
/// more for the facts, one line at a time, so a reader holds no more of a large log than it keeps of each fact, and
/// it gives no fact of a log that does not agree.
pub(crate) fn each_logged(
  path: &Path,
  entries: usize,
  asked: &HashSet<String>,
  take: &mut dyn FnMut(Object),
) -> Result<bool, Fault> {
  if !walked(path, entries, asked.clone(), None)? {
    return Ok(false);
  }
  match walked(path, entries, asked.clone(), Some(take))? {
    true => Ok(true),
    false => Err(Fault::refused(format!("The log of {} changed as it was read.", path.display()))),
  }
}

/// Whether the log of the record at a path agrees with a record of that many entries and of those questions, read one
/// line at a time, and each of its facts in order, to take when it is given one.
fn walked(
  path: &Path,
  record: usize,
  mut asked: HashSet<String>,
  mut take: Option<&mut dyn FnMut(Object)>,
) -> Result<bool, Fault> {
  let Ok(file) = File::open(format!("{}.log", path.display())) else {
    return Ok(false);
  };
  let mut lines = BufReader::with_capacity(1 << 20, file);
  let mut line = String::new();
  let head = match lines.read_line(&mut line) {
    Ok(_) => serde_json::from_str::<Value>(line.trim_end_matches('\n')).ok(),
    Err(_) => return Ok(false),
  };
  let Some(mut entries) = head.and_then(|one| one.get("entries")?.as_u64()) else {
    return Ok(false);
  };
  loop {
    line.clear();
    match lines.read_line(&mut line) {
      Ok(0) => break,
      Ok(_) => {}
      Err(_) => return Ok(false),
    }
    let (ended, text) = (line.ends_with('\n'), line.trim_end_matches('\n'));
    let fact = match take {
      Some(_) => parsed(text).ok(),
      None => None,
    };
    let Ok(held) = serde_json::from_str::<Vec<&RawValue>>(text) else {
      if !ended {
        break;
      }
      return Ok(false);
    };
    let word = |at: usize| {
      held
        .get(at)
        .and_then(|one| serde_json::from_str::<String>(one.get()).ok())
        .unwrap_or_default()
    };
    let (kind, about) = (word(0), word(1));
    if held.len() == 3 && kind == "keep" && about.is_empty() {
      if serde_json::from_str::<u64>(held[2].get()).ok() != Some(entries + 1) {
        return Ok(false);
      }
      entries += 1;
      continue;
    }
    if named(&about, &kind) {
      asked.remove(&about);
    }
    if let Some(take) = take.as_mut() {
      let Some(fact) = fact else { return Ok(false) };
      take(fact);
    }
  }
  Ok(asked.is_empty() && usize::try_from(entries).ok() == Some(record))
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
