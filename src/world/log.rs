//! The log of a life, `<record>.log`: every fact that the life says, in the order of the log, as an ear of a host
//! hears it, so a reader shows the life whole with no life. It holds what the record leaves out, as the rungs of each
//! ladder, each run, and each tell of a chain. It is a cache, which each life on the record writes again from its
//! first line, since a later life says every fact of its record again, before it says its own. A life writes it in
//! `<record>.log.new`, which takes the place of the log when the life keeps its first entry, after its replay, and
//! which goes when the life ends before that, so a life whose replay drifts leaves the log of the life before it as it
//! was, and nothing more.
//!
//! A life that goes on from a checkpoint replays only what the checkpoint carries, so its log starts with the lines of
//! the log before it that come before the checkpoint, while that log agrees with the record up to there, then a line
//! of keep for the checkpoint and for each entry after it, and then what the life says. So a reader of the log still
//! finds each fact of the lives before the checkpoint, and the log agrees with the record.
//!
//! Its first line is a map that holds under `entries` how many entries the record held when the life opened. Each
//! other line is one fact, as JSON reads a value of the sandbox, but for the line `["keep", "", n]`, which says that
//! the record keeps its entry `n`, counted from 1, there. An act that no fact showed before, since an ear of the
//! engine took it, stands on the line before the first fact about it. The fact of a module stands with no words,
//! since a module is no plain value and differs from one life to the next. The ear writes no
//! fsync of the log. A line waits in a buffer, which the file takes at the end of each run of
//! the sandbox, so a reader sees a fact of the log once its run ends. A line that the file
//! cannot take, as on a full disk, waits in memory with each line after it, and the ear writes
//! them in order at its next line, so the log lacks no fact once the disk has room.
//!
//! A later life makes again no act of its record that an ear of the outside made, unless that ear makes it again, and
//! says no fact about one that it does not make. So when the life keeps its first entry, the log writes each entry of
//! the record about such an act that no line shows, in the order of the record, and the log still holds every act of
//! its record.
//!
//! [`Line`] reads a line of a log from its head, and [`keep`] writes the line of keep, so the
//! format of the log has this one home, which the reader of a record with no life shares.

use std::{
  borrow::Cow,
  cell::RefCell,
  collections::HashSet,
  fmt::Display,
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, BufWriter, Write},
  path::{Path, PathBuf},
  rc::Rc,
};

use serde_json::Value;

use crate::{
  ear::{Co, Ear, Hooks, With, ear, hear},
  fact::{Fact, named},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::{beside, kept},
};

/// The kind of the line that says the record kept an entry there, in the log and in what a
/// record tells.
pub(super) const KEEP: &str = "keep";

/// How many bytes of lines the buffer holds before it hands them to the file.
const BUFFER: usize = 1 << 20;

/// The line that says the record kept its entry `n`, counted from 1, with no end of line.
pub(super) fn keep(n: usize) -> String {
  format!(r#"["{KEEP}","",{n}]"#)
}

/// A line of a log, read from its head alone, so no fact is made.
pub(super) enum Line<'a> {
  /// The map of the first line.
  Head,
  /// The line that says the record kept its entry `n`, with `n` when the line reads one.
  Keep(Option<usize>),
  /// A question, with the name of the act that it makes.
  Question(Cow<'a, str>),
  /// Any other fact.
  Other,
}

impl<'a> Line<'a> {
  /// The line from its first two words, as `["bash","bash3",`. A line whose head holds an escape
  /// is parsed whole.
  pub(super) fn of(text: &'a str) -> Self {
    let Some(rest) = text.strip_prefix("[\"") else {
      return if text.starts_with('{') { Line::Head } else { Line::Other };
    };
    let Some((kind, rest)) = rest.split_once('"') else { return Line::Other };
    let about = rest.strip_prefix(",\"").and_then(|one| one.split_once('"'));
    if kind == KEEP
      && let Some(("", number)) = about
    {
      let at = number.strip_prefix(',').and_then(|one| one.strip_suffix(']'));
      return Line::Keep(at.and_then(|one| one.parse().ok()));
    }
    let name = about.map(|one| one.0);
    if kind.contains('\\') || name.is_some_and(|one| one.contains('\\')) {
      let fact = parsed(text).ok();
      let fact = fact.as_ref().and_then(|one| Fact::of(one.as_ref())).filter(Fact::question);
      return fact.map_or(Line::Other, |one| Line::Question(Cow::Owned(one.about().to_owned())));
    }
    name
      .filter(|one| named(one, kind))
      .map_or(Line::Other, |one| Line::Question(Cow::Borrowed(one)))
  }
}

/// The ear of the log of the record at a path, which held a number of entries when the life opened: it hears every
/// fact, and writes the log again from its first line.
pub fn log(record: impl AsRef<Path>, held: &[Object]) -> Box<dyn Ear> {
  heard(Log::open(record.as_ref(), held))
}

/// The ear that writes a log.
fn heard(mut log: Log) -> Box<dyn Ear> {
  let sink = Rc::clone(&log.sink);
  let ear = ear(move |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      match a.kind() {
        "keep" => log.kept(),
        _ => log.heard(&mut co, &a).await,
      }
    }
  });
  With::ear(ear, sink)
}

/// The ear of a log hands the lines to the file at the end of each run, and says when lines wait.
impl Hooks for Rc<RefCell<Sink>> {
  /// A fault of the file ends no run: the lines wait and the ear writes them again.
  fn ran(&mut self, _: &mut dyn Ear) -> Result<(), Fault> {
    self.borrow_mut().flush();
    Ok(())
  }
}

/// The file of the log, and the lines that wait for it.
struct Sink {
  /// The file, in a buffer that fills to [`BUFFER`] bytes, or nothing when the log cannot be
  /// written.
  out: Option<BufWriter<File>>,
  /// The lines that the buffer did not take, as on a full disk, with each line after them, in
  /// order.
  waiting: String,
}

impl Sink {
  /// Lines, each with its end, written after each line that waits.
  fn put(&mut self, lines: &str) {
    if self.out.is_some() {
      self.waiting.push_str(lines);
      self.drain();
    }
  }

  /// Each line that waits, handed to the buffer in order.
  fn drain(&mut self) {
    if let Some(out) = self.out.as_mut()
      && out.write_all(self.waiting.as_bytes()).is_ok()
    {
      self.waiting.clear();
    }
  }

  /// Each line, handed to the file.
  fn flush(&mut self) {
    self.drain();
    if let Some(out) = self.out.as_mut() {
      drop(out.flush());
    }
  }
}

/// The log file, and what the ear knows of it.
struct Log {
  path: PathBuf,
  sink: Rc<RefCell<Sink>>,
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

/// The lines of the log at a path that come before a checkpoint, the entries before which a record holds, with its
/// head, and each name that they show: nothing when the log does not agree with those entries, as [`agreed`] reads it.
/// A log of a life that went on from an earlier checkpoint holds a line of keep for this checkpoint, where they end.
fn earlier(log: &Path, before: &[Object]) -> Option<(String, HashSet<String>)> {
  let text = std::fs::read_to_string(log).ok()?;
  let mut lines = text.split_inclusive('\n');
  let head = lines.next()?;
  let mut entries =
    usize::try_from(serde_json::from_str::<Value>(head.trim_end()).ok()?.get("entries")?.as_u64()?)
      .ok()?;
  let mut asked: HashSet<String> = before
    .iter()
    .filter_map(|entry| Fact::of(*entry.as_ref().items()?.first()?))
    .filter(Fact::question)
    .map(|one| one.about().to_owned())
    .collect();
  let (mut kept, mut seen) = (head.to_owned(), HashSet::new());
  for line in lines {
    let whole = line.strip_suffix('\n')?;
    let value = serde_json::from_str::<Value>(whole).ok()?;
    match Line::of(whole) {
      Line::Keep(at) => {
        let at = at?;
        if at != entries + 1 {
          return None;
        }
        if at > before.len() {
          break;
        }
        entries = at;
      }
      Line::Question(name) => {
        asked.remove(name.as_ref());
      }
      Line::Head | Line::Other => {}
    }
    if let Some(about) = value.get(1).and_then(Value::as_str).filter(|one| !one.is_empty()) {
      seen.insert(about.to_owned());
    }
    kept.push_str(line);
  }
  (entries == before.len() && asked.is_empty()).then_some((kept, seen))
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
  /// The log of the record at a path, which holds these entries, written again from its first line, or from the
  /// lines of the log before it that come before the checkpoint that the life boots on.
  fn open(record: &Path, held: &[Object]) -> Self {
    let path = beside(record, ".log");
    let fresh = beside(record, ".log.new");
    let earlier = crate::checkpoint::last(held)
      .and_then(|at| earlier(&path, &held[..at]).map(|(text, seen)| (at, text, seen)));
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(&fresh).ok();
    let entries = held.len();
    let mut log = Self {
      path,
      sink: Self::sink(file),
      fresh: Some(fresh),
      entries,
      seen: HashSet::new(),
      owed: owed(held),
    };
    match earlier {
      Some((at, mut text, seen)) => {
        log.seen = seen;
        for n in at + 1..=entries {
          text.push_str(&keep(n));
          text.push('\n');
        }
        let mut sink = log.sink.borrow_mut();
        sink.put(&text);
        sink.flush();
      }
      None => log.line(&serde_json::json!({ "entries": entries })),
    }
    log
  }

  /// The sink of a log file, when it opened.
  fn sink(file: Option<File>) -> Rc<RefCell<Sink>> {
    let out = file.map(|file| BufWriter::with_capacity(BUFFER, file));
    Rc::new(RefCell::new(Sink { out, waiting: String::new() }))
  }

  /// Whether the log can be written.
  fn live(&self) -> bool {
    self.sink.borrow().out.is_some()
  }

  /// A fact that the ear heard, after the act it is about when no line shows that act yet.
  async fn heard(&mut self, co: &mut Co, a: &Fact) {
    if !self.live() {
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
      && self.live()
      && std::fs::rename(&fresh, &self.path).is_err()
    {
      self.sink.borrow_mut().out = None;
      // The next life truncates the file, so one that stays does no harm.
      std::fs::remove_file(&fresh).unwrap_or_default();
    }
    self.entries += 1;
    self.line(&keep(self.entries));
  }

  /// One line, written after each line that waits, or waiting with them when the file cannot take
  /// it.
  fn line(&mut self, value: &impl Display) {
    self.sink.borrow_mut().put(&format!("{value}\n"));
  }
}

impl Drop for Log {
  /// The file that the life wrote the log again in goes when it never took the place of the log.
  fn drop(&mut self) {
    if let Some(fresh) = self.fresh.take() {
      self.sink.borrow_mut().out = None;
      // The next life truncates the file, so one that stays does no harm.
      std::fs::remove_file(fresh).unwrap_or_default();
    }
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
  let Ok(text) = std::fs::read_to_string(beside(path, ".log")) else {
    return Ok(None);
  };
  let facts = text
    .lines()
    .skip(1)
    .filter(|line| !matches!(Line::of(line), Line::Keep(_)))
    .map_while(|line| parsed(line).ok())
    .collect();
  Ok(Some(facts))
}

/// Whether the log of the record at a path agrees with a record of that many entries and of those questions, as
/// [`logged`] reads it, which the caller read once.
pub(crate) fn agrees(path: &Path, entries: usize, asked: &HashSet<String>) -> bool {
  File::open(beside(path, ".log"))
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
  let Some(mut entries) = head.and_then(|one| usize::try_from(one.get("entries")?.as_u64()?).ok())
  else {
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
      line.strip_suffix('\n').map_or(line.as_str(), |one| one.strip_suffix('\r').unwrap_or(one));
    if serde_json::from_str::<serde::de::IgnoredAny>(text).is_err() {
      if !ended {
        break;
      }
      return false;
    }
    match Line::of(text) {
      Line::Keep(at) => {
        if at != entries.checked_add(1) {
          return false;
        }
        entries += 1;
      }
      Line::Question(name) => {
        asked.remove(name.as_ref());
      }
      Line::Head | Line::Other => {}
    }
  }
  asked.is_empty() && entries == record
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
