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

use serde::{Deserialize, Serialize};
use std::{
  borrow::Cow,
  cell::RefCell,
  collections::HashSet,
  fmt::Display,
  fs::{File, OpenOptions},
  io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
  ops::RangeInclusive,
  path::{Path, PathBuf},
  rc::Rc,
};

use serde_json::Value;

use crate::checkpoint;
use crate::{
  ear::{Co, Ear, Hooks, With, ear, hear},
  fact::{Fact, named},
  value::{Fault, Object},
  wire::{outward, parsed},
  world::{
    beside, kept,
    store::{asks, heads, spoken},
  },
};
use serde::de;
use std::fs;
use std::mem;

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
  logging(record, held, 0)
}

/// The ear of [`log`] for the last entries of a record, which start at a byte of its file of lines.
/// The log reads the start of each line before that byte and parses none, but the lines of the
/// acts that a new log owes.
pub(crate) fn logging(record: impl AsRef<Path>, last: &[Object], from: u64) -> Box<dyn Ear> {
  heard(Log::open(record.as_ref(), last, from), 0)
}

/// The ear of [`log`] for a life that opens from its image, which reads no entry before it: the
/// log goes on after the last line that the ear of the life of the image wrote, with what that
/// ear knew of it. The record holds `tail` entries after the image, which the life says again.
pub(crate) fn logging_on(record: impl AsRef<Path>, tail: usize) -> Box<dyn Ear> {
  heard(Log::tipped(record.as_ref()), tail)
}

/// The ear that writes a log, on a record that holds `tail` entries after those that its lines
/// count.
fn heard(mut log: Log, tail: usize) -> Box<dyn Ear> {
  let sink = Rc::clone(&log.sink);
  let ear = ear(move |mut co| async move {
    let first = log.tip.take();
    log.tip = co.keeps(first)?;
    // A log that never took its place holds no line of the life before, so no line goes after.
    // A life that went on from the image and wrote no later one left lines after those that the
    // image counts, and this life says their facts again, so the log is cut back to those lines.
    if log.fresh.is_none() {
      let (placed, bytes) = (log.tip.borrow().placed, log.tip.borrow().bytes);
      let mut sink = log.sink.borrow_mut();
      let cut = sink.out.as_mut().is_some_and(|out| {
        let file = out.get_mut();
        let short = file.metadata().is_ok_and(|held| held.len() <= bytes);
        (short || file.set_len(bytes).is_ok()) && file.seek(SeekFrom::End(0)).is_ok()
      });
      if !(placed && cut) {
        sink.out = None;
      }
    }
    // The record keeps each entry that the life says again, so no fact of keep comes for it: the
    // log counts them before their facts, as a log that a life writes again from its record does.
    for _ in 0..tail {
      log.kept();
    }
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
  /// What the ear knows of the lines of the log, which the image of the life holds.
  tip: Rc<RefCell<Tip>>,
  /// The entries of the record about each act that an ear of the outside made, by the name of that act, in the order
  /// of the record, which the log writes when the life keeps its first entry and no line shows that act.
  owed: Vec<(String, Value)>,
}

/// What the ear of a log knows of its lines: how many entries the record holds, each name that a
/// line of the log shows as an act, or that names no act, whether the log stands at its path,
/// which it does once the life kept an entry, and the bytes of the lines that the ear wrote.
#[derive(Default, Serialize, Deserialize)]
struct Tip {
  entries: usize,
  seen: HashSet<String>,
  placed: bool,
  bytes: u64,
}

/// The lines of the log at a path that come before a checkpoint, with its head, and each name that
/// they show: nothing when the log does not agree with the entries that a record holds before that
/// checkpoint, of which it is given the count and the questions, as [`agreed`] reads it. A log of a
/// life that went on from an earlier checkpoint holds a line of keep for this checkpoint, where
/// they end.
fn earlier(
  log: &Path,
  before: usize,
  mut asked: HashSet<String>,
) -> Option<(String, HashSet<String>)> {
  let text = fs::read_to_string(log).ok()?;
  let mut lines = text.split_inclusive('\n');
  let head = lines.next()?;
  let mut entries =
    usize::try_from(serde_json::from_str::<Value>(head.trim_end()).ok()?.get("entries")?.as_u64()?)
      .ok()?;
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
        if at > before {
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
  (entries == before && asked.is_empty()).then_some((kept, seen))
}

/// Whether the speaker of a question is an ear of the outside, and not an act, the operator or the
/// journal.
fn outside(by: &str) -> bool {
  !named_act(by) && !matches!(by, "" | "operator" | "journal")
}

/// What the entries of a record before a byte of its file of lines tell from the start of each
/// line, with no parse: how many they are, and the name of each question among them.
fn before(record: &Path, to: u64) -> Result<(usize, HashSet<String>), Fault> {
  let (mut entries, mut asked) = (0, HashSet::new());
  let mut heard = |line: &[u8]| {
    entries += 1;
    asked.extend(asks(line).map(str::to_owned));
    false
  };
  heads(record, to, &mut heard, &mut |_| {})?;
  Ok((entries, asked))
}

/// The entries of a record before a byte of its file of lines about each act that an ear of the
/// outside made, in order, which are the only lines of them that the read parses.
fn theirs(record: &Path, to: u64) -> Result<Vec<Object>, Fault> {
  let (mut made, mut about) = (HashSet::new(), Vec::new());
  let mut wanted = |line: &[u8]| {
    let Some([_, name, by]) = spoken(line) else { return false };
    if asks(line).is_some() && outside(by) {
      made.insert(name.to_owned());
    }
    made.contains(name)
  };
  heads(record, to, &mut wanted, &mut |entry| about.push(entry))?;
  Ok(about)
}

/// Whether a name is the name of an act, as `bash3`, and not the name of an ear.
fn named_act(name: &str) -> bool {
  let kind = name.trim_end_matches(|one: char| one.is_ascii_digit());
  !kind.is_empty() && kind.len() < name.len()
}

/// The entries of a record about each act that an ear of the outside made, which a later life does not say again
/// unless that ear makes the act again.
fn owed<'a>(held: impl Iterator<Item = &'a Object>) -> Vec<(String, Value)> {
  let mut theirs = HashSet::new();
  let mut owed = Vec::new();
  for entry in held {
    let Some(fact) = entry.as_ref().items().and_then(|one| Fact::of(*one.first()?)) else {
      continue;
    };
    if fact.question() && outside(fact.by()) {
      theirs.insert(fact.about().to_owned());
    }
    if theirs.contains(fact.about()) {
      owed.push((fact.about().to_owned(), outward(fact.0.as_ref())));
    }
  }
  owed
}

impl Log {
  /// The log of the record at a path, whose last entries are these and start at a byte of its file
  /// of lines, written again from its first line, or from the lines of the log before it that come
  /// before the checkpoint that the life boots on.
  fn open(record: &Path, last: &[Object], from: u64) -> Self {
    let path = beside(record, ".log");
    let fresh = beside(record, ".log.new");
    let past = before(record, from);
    let reads = past.is_ok();
    let (held, mut asked) = past.unwrap_or_default();
    let earlier = checkpoint::last(last).and_then(|at| {
      let facts = last[..at].iter().filter_map(|entry| Fact::of(*entry.as_ref().items()?.first()?));
      asked.extend(facts.filter(Fact::question).map(|one| one.about().to_owned()));
      earlier(&path, held + at, asked).map(|(text, seen)| (held + at, text, seen))
    });
    // A log that goes on from an earlier one shows each act of the entries before the last ones.
    let made = match &earlier {
      None => theirs(record, from),
      Some(_) => Ok(Vec::new()),
    };
    let reads = reads && made.is_ok();
    let made = made.unwrap_or_default();
    // A log that cannot be opened, or whose record does not read, is not written: the record holds
    // each fact, and a later life writes the log again from it, so the life goes on with no log
    // and loses nothing.
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(&fresh).ok();
    let entries = held + last.len();
    let mut log = Self {
      path,
      sink: Self::sink(file.filter(|_| reads)),
      fresh: Some(fresh),
      tip: Rc::new(RefCell::new(Tip { entries, ..Tip::default() })),
      owed: owed(made.iter().chain(last)),
    };
    match earlier {
      Some((at, mut text, seen)) => {
        log.tip.borrow_mut().seen = seen;
        for n in at + 1..=entries {
          text.push_str(&keep(n));
          text.push('\n');
        }
        log.put(&text);
        log.sink.borrow_mut().flush();
      }
      None => log.line(&serde_json::json!({ "entries": entries })),
    }
    log
  }

  /// The log of a record that goes on after its last line, for a life that opens from its image,
  /// which gives the ear what it knows of the lines.
  fn tipped(record: &Path) -> Self {
    let path = beside(record, ".log");
    // Not an append handle, which cannot cut the file on Windows: the ear cuts the log back to the
    // lines of the image, then writes after them.
    let file = OpenOptions::new().write(true).open(&path).ok();
    Self { path, sink: Self::sink(file), fresh: None, tip: Rc::default(), owed: Vec::new() }
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
      if self.tip.borrow_mut().seen.insert(about.to_owned()) {
        self.line(&outward(a.0.as_ref()));
      }
      return;
    }
    if !about.is_empty() && self.tip.borrow_mut().seen.insert(about.to_owned()) {
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
      .filter(|one| !self.tip.borrow().seen.contains(one))
      .collect();
    for (about, line) in mem::take(&mut self.owed) {
      if missed.contains(&about) {
        self.tip.borrow_mut().seen.insert(about);
        self.line(&line);
      }
    }
    if let Some(fresh) = self.fresh.take()
      && self.live()
      && fs::rename(&fresh, &self.path).is_err()
    {
      self.sink.borrow_mut().out = None;
      // The next life truncates the file, so one that stays does no harm.
      fs::remove_file(&fresh).unwrap_or_default();
    }
    let entries = {
      let mut tip = self.tip.borrow_mut();
      (tip.entries, tip.placed) = (tip.entries + 1, self.live());
      tip.entries
    };
    self.line(&keep(entries));
  }

  /// One line, written after each line that waits, or waiting with them when the file cannot take
  /// it.
  fn line(&mut self, value: &impl Display) {
    self.put(&format!("{value}\n"));
  }

  /// Lines, each with its end, written after each line that waits, and counted.
  fn put(&mut self, lines: &str) {
    if self.live() {
      self.tip.borrow_mut().bytes += lines.len() as u64;
      self.sink.borrow_mut().put(lines);
    }
  }
}

impl Drop for Log {
  /// The file that the life wrote the log again in goes when it never took the place of the log.
  fn drop(&mut self) {
    if let Some(fresh) = self.fresh.take() {
      self.sink.borrow_mut().out = None;
      // The next life truncates the file, so one that stays does no harm.
      fs::remove_file(fresh).unwrap_or_default();
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
  let facts = record.iter().map(|entry| Fact::of(*entry.as_ref().items()?.first()?));
  let last = facts.clone().fold(0, |before, fact| sealed(before, fact.as_ref()));
  let asked: HashSet<String> =
    facts.flatten().filter(Fact::question).map(|one| one.about().to_owned()).collect();
  if !agrees(path, record.len(), &asked, last) {
    return Ok(None);
  }
  let Ok(text) = fs::read_to_string(beside(path, ".log")) else {
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

/// Whether the log of the record at a path agrees with a record of that many entries and of those
/// questions, as [`logged`] reads it, which the caller read once. The record ends with `sealed`
/// checkpoints: a checkpoint is no fact, and one that no life wrote, as the one of
/// `furb checkpoint`, has no line of keep, so a log that lacks the lines of keep of those entries
/// alone still holds each fact of the record.
pub(crate) fn agrees(path: &Path, entries: usize, asked: &HashSet<String>, sealed: usize) -> bool {
  agreed(path, entries.saturating_sub(sealed)..=entries, asked, false)
}

/// How many checkpoints end a record, given each kept fact of it in order: the count before a
/// fact, and after it.
pub(crate) fn sealed(before: usize, fact: Option<&Fact>) -> usize {
  if fact.is_some_and(|one| one.kind() == checkpoint::CHECKPOINT) { before + 1 } else { 0 }
}

/// Whether the lines of the log of the record at a path, as far as the line that counts an entry
/// past `entries`, agree with the first `entries` entries of the record and their questions. A
/// record takes an entry before its log takes the line of it, so the log of a life that runs stands
/// behind its record, and agrees with it only so far.
pub(crate) fn agrees_so_far(path: &Path, entries: usize, asked: &HashSet<String>) -> bool {
  agreed(path, entries..=entries, asked, true)
}

/// How many entries of a record the lines of the log at a path count: those of its head and one
/// for each line of keep, and none for a log that does not read.
pub(crate) fn counted(log: &Path) -> usize {
  let Ok(file) = File::open(log) else { return 0 };
  let mut lines = BufReader::with_capacity(1 << 20, file).lines().map_while(Result::ok);
  let first = lines.next().and_then(|line| serde_json::from_str::<Value>(&line).ok());
  let held = first.and_then(|one| usize::try_from(one.get("entries")?.as_u64()?).ok());
  held.map_or(0, |held| held + lines.filter(|line| matches!(Line::of(line), Line::Keep(_))).count())
}

/// Whether a log agrees with a record of that many entries and of those questions: its head counts the entries that
/// the record held, its lines of keep count the others in order, each line is JSON but a last one that a crash tore,
/// and it shows each question of the record. Each line is read from its head alone, one at a time, and no fact is
/// made. When `past` says so, the log may count entries past those of the record, and its lines
/// before the first of them are the ones read. The record holds the last of `counts` entries, and
/// the log may count as few as the first of them, when the entries that it lacks are no facts.
fn agreed(path: &Path, counts: RangeInclusive<usize>, asked: &HashSet<String>, past: bool) -> bool {
  let record = *counts.end();
  let Ok(file) = File::open(beside(path, ".log")) else { return false };
  let (mut lines, mut asked) = (BufReader::with_capacity(1 << 20, file), asked.clone());
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
    if serde_json::from_str::<de::IgnoredAny>(text).is_err() {
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
        if entries == record {
          return past && asked.is_empty();
        }
        entries += 1;
      }
      Line::Question(name) => {
        asked.remove(name.as_ref());
      }
      Line::Head | Line::Other => {}
    }
  }
  asked.is_empty() && counts.contains(&entries)
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
