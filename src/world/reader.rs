//! The reader of a record: what a record tells, read with no life, so no word runs again, no model is asked, no
//! command runs, and a record of any engine opens. A host that reads a record hears these facts in order, as an ear
//! hears the facts of a life.
//!
//! While the log beside the record agrees with it, the reader gives every fact of the log, as [`logged`] reads it.
//! Else it gives the facts that the record kept. The record keeps only what the outside said: the World, the models
//! and the operator. So the reader says, after the facts that name them, the facts that the engine said and the record
//! does not keep, as far as the kept facts show them:
//!
//! - each chain that a kept act stands on, made by the rung that makes the first prompt on it, or by the operator,
//!   named by the first line of that prompt, or by its own name when the record keeps no prompt on it, and standing
//!   under the chain that its maker ran on;
//! - each rung that a kept act names as its maker, on the open prompt of its chain that is no question to the
//!   operator, or on its chain, which ends the rung before it of the same maker;
//! - the run of the word of each rung, which the rung holds or the reply that it asked for gives, once the gate lets it
//!   run, and
//!   the paragraph that tells the findings of the gate on a rung whose word it refused;
//! - the done of each act that the operator closed or cancelled.
//!
//! A question that the reader says names its maker, as the engine names it, and each other fact that it says is said
//! by [`READER`].

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{
  fact::{Fact, named},
  value::{Fault, IS, Object, ObjectRef, entry, field},
  wire::{outward, parsed},
};

use super::{log::agrees, made, store::each, version};

/// Who says each fact that the reader adds to the facts that a record kept, but a question.
pub const READER: &str = "reader";

/// What a record tells, read with no life.
#[derive(Debug, Clone)]
pub struct Retold {
  /// The engine that made the record, as the record names it, and nothing when it names none.
  pub made: Option<String>,
  /// The version of the build that made the record, as the record names it, and nothing when it names none.
  pub version: Option<i64>,
  /// Whether the facts are those of the log, which holds every fact that the life said.
  pub logged: bool,
  /// The file that holds the facts, in the order that the life said them, each on a line of its own as JSON reads a
  /// value of the sandbox: the log beside the record while it agrees with the record, and else the file of what the
  /// record tells, `<record>.told`, beside it. A line that does not start with `[`, and a line `["keep", "", n]` of
  /// the log, holds no fact.
  pub file: PathBuf,
  /// The count of the entries of the record.
  pub entries: usize,
  /// The sha256 of the last entry of the record, in the plain form that the record keeps, and nothing for a record
  /// with no entry.
  pub last: Option<String>,
}

impl Retold {
  /// Each fact of the file, in the order that the life said them, read one line at a time, so a reader holds no more
  /// of a large record than it keeps of each fact.
  pub fn each(&self, take: &mut dyn FnMut(Object)) -> Result<(), Fault> {
    let failed =
      |no: std::io::Error| Fault::refused(format!("{} cannot be read: {no}", self.file.display()));
    let file = std::fs::File::open(&self.file).map_err(failed)?;
    let mut lines = std::io::BufReader::with_capacity(1 << 20, file);
    let mut line = String::new();
    loop {
      line.clear();
      if lines.read_line(&mut line).map_err(failed)? == 0 {
        return Ok(());
      }
      let text = line.trim_end_matches('\n');
      if text.starts_with('[') && !text.starts_with(r#"["keep","","#) {
        take(parsed(text)?);
      }
    }
  }

  /// The facts of the file, in the order that the life said them.
  pub fn facts(&self) -> Result<Vec<Object>, Fault> {
    let mut facts = Vec::new();
    self.each(&mut |one| facts.push(one))?;
    Ok(facts)
  }
}

/// What the record at a path tells, read with no lease and changed in nothing.
///
/// The file of what the record tells is a cache: its first line names the size and the time of change of the record
/// and of its log, and the build that wrote it. While all agree, a later read takes it as it stands and reads no
/// record. Else the reader writes it again from the record.
pub fn retold(path: impl AsRef<Path>) -> Result<Retold, Fault> {
  let path = path.as_ref();
  let log = PathBuf::from(format!("{}.log", path.display()));
  let cache = PathBuf::from(format!("{}.told", path.display()));
  let stamp = serde_json::json!([stamped(path), stamped(&log), BUILD]);
  if let Some(found) = cached(&cache, &stamp, &log) {
    return Ok(found);
  }
  let (mut maker, mut built, mut entries, mut last) = (None, None, 0, None);
  let (mut asked, mut makers) = (HashSet::new(), Makers::default());
  each(path, &mut |entry| {
    let one = std::slice::from_ref(&entry);
    if maker.is_none() {
      maker = made(one);
    }
    built = built.or_else(|| version(one));
    if let Some(fact) = kept(&entry) {
      makers.hear(&fact);
      if fact.question() {
        asked.insert(fact.about().to_owned());
      }
    }
    entries += 1;
    last = Some(entry);
  })?;
  let logged = agrees(path, entries, &asked);
  let last = last.as_ref().map(digest);
  let head = serde_json::json!({ "stamp": stamp, "made": maker, "version": built, "logged": logged, "entries": entries, "last": last });
  let fresh = PathBuf::from(format!("{}.told.new", path.display()));
  let failed =
    |no: std::io::Error| Fault::refused(format!("{} cannot be written: {no}", fresh.display()));
  let mut file = std::io::BufWriter::new(std::fs::File::create(&fresh).map_err(failed)?);
  writeln!(file, "{head}").map_err(failed)?;
  if !logged {
    let mut broken = None;
    {
      let mut out = |fact: Object| {
        if broken.is_none()
          && let Err(no) = writeln!(file, "{}", outward(fact.as_ref()))
        {
          broken = Some(no);
        }
      };
      let mut life = Reader::new(makers.ended(), &mut out);
      each(path, &mut |entry| {
        if let Some(fact) = kept(&entry) {
          life.hear(fact);
        }
      })?;
    }
    if let Some(no) = broken {
      return Err(failed(no));
    }
  }
  file.into_inner().map_err(|no| failed(no.into_error()))?.sync_data().map_err(failed)?;
  std::fs::rename(&fresh, &cache).map_err(failed)?;
  Ok(Retold {
    made: maker,
    version: built,
    logged,
    file: if logged { log } else { cache },
    entries,
    last,
  })
}

/// What the build that writes a file of what a record tells names in it: a reader of another build writes it again.
const BUILD: &str = concat!(env!("CARGO_PKG_VERSION"), "/", env!("CARGO_PKG_NAME"), "/1");

/// The size and the time of change of a file, in nanoseconds, or nothing when no file stands there.
fn stamped(path: &Path) -> Option<(u64, u128)> {
  let meta = std::fs::metadata(path).ok()?;
  let at = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
  Some((meta.len(), at))
}

/// What the file of what a record tells holds, while its first line names the stamp that the record, its log and the
/// build give now.
fn cached(told: &Path, stamp: &serde_json::Value, log: &Path) -> Option<Retold> {
  let file = std::fs::File::open(told).ok()?;
  let mut head = String::new();
  std::io::BufRead::read_line(&mut std::io::BufReader::new(file), &mut head).ok()?;
  let head: serde_json::Value = serde_json::from_str(&head).ok()?;
  if head.get("stamp")? != stamp {
    return None;
  }
  let logged = head.get("logged")?.as_bool()?;
  Some(Retold {
    made: head.get("made")?.as_str().map(str::to_owned),
    version: head.get("version")?.as_i64(),
    logged,
    file: if logged { log.to_path_buf() } else { told.to_path_buf() },
    entries: usize::try_from(head.get("entries")?.as_u64()?).ok()?,
    last: head.get("last")?.as_str().map(str::to_owned),
  })
}

/// The fact of an entry of a record.
fn kept(entry: &Object) -> Option<Fact> {
  Fact::owned(entry.as_ref().items()?.first()?.to_owned())
}

/// The sha256 of an entry of a record, in the plain form that the record keeps.
fn digest(entry: &Object) -> String {
  format!("{:x}", Sha256::digest(crate::wire::record(entry.as_ref()).to_string()))
}

/// A fact that the reader says: its kind, what it is about, and its words.
fn said(kind: &str, about: &str, words: impl IntoIterator<Item = Object>) -> Object {
  let mut held = vec![Object::string(kind), Object::string(about), Object::string(READER)];
  held.extend(words);
  Object::list(held)
}

/// A question that the reader says for its maker: its kind, its name, its maker, and its words, as the engine says it.
fn question<'a>(
  kind: &'a str,
  id: &'a str,
  maker: &'a str,
  words: impl IntoIterator<Item = &'a str>,
) -> Object {
  Object::list([kind, id, maker].into_iter().chain(words).map(Object::string))
}

/// The items of a value that is a list or a tuple, which a record keeps as a map whose `is` is tuple.
fn items(said: ObjectRef<'_>) -> Vec<ObjectRef<'_>> {
  match field(&said, IS).and_then(|one| one.as_str()) {
    Some("tuple") => {
      field(&said, "args").and_then(|one| entry(&one, 0)).and_then(|one| one.items())
    }
    _ => said.items(),
  }
  .unwrap_or_default()
}

/// The text of a word of a fact, and an empty text when it is no text.
fn text(fact: &Fact, at: usize) -> String {
  fact.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// The facts that a record kept, with the facts that the engine said and the record does not keep.
#[cfg(test)]
fn told(kept: Vec<Object>) -> Vec<Object> {
  let facts: Vec<Fact> = kept.into_iter().filter_map(Fact::owned).collect();
  let mut makers = Makers::default();
  facts.iter().for_each(|one| makers.hear(one));
  let mut out = Vec::new();
  let mut take = |one| out.push(one);
  let mut life = Reader::new(makers.ended(), &mut take);
  for fact in facts {
    life.hear(fact);
  }
  out
}

/// The label of a chain from the message of its first prompt: its first line that says something, in at most 80
/// characters.
fn titled(message: &str) -> String {
  let line = message.lines().map(str::trim).find(|one| !one.is_empty()).unwrap_or_default();
  match line.char_indices().nth(80) {
    Some((at, _)) => format!("{}…", line[..at].trim_end()),
    None => line.to_owned(),
  }
}

/// What the reader needs to know of all the facts that a record kept before it tells the first of them: each chain
/// that the record holds, the chain and the maker of each prompt that a rung made, in order, the chain that each rung
/// ran on, and the first prompt on each chain.
#[derive(Default)]
struct Makers {
  chains: HashSet<String>,
  prompts: Vec<(String, String)>,
  homes: HashMap<String, String>,
  labels: HashMap<String, String>,
}

/// What the reader knows of the chains that the record does not hold.
#[derive(Default)]
struct Known {
  /// The maker of each chain: the rung that makes the first prompt on it.
  makers: HashMap<String, String>,
  /// The chain of each rung that a kept act on a chain names as its maker, which is the chain that the rung ran on.
  homes: HashMap<String, String>,
  /// The label of each chain: the first line of the first prompt that the record kept on it.
  labels: HashMap<String, String>,
}

impl Makers {
  fn hear(&mut self, one: &Fact) {
    if one.question() && one.kind() == "chain" {
      self.chains.insert(one.about().to_owned());
    }
    if one.question() && one.kind() == "prompt" && named(one.by(), "rung") {
      self.prompts.push((one.on().to_owned(), one.by().to_owned()));
    }
    if one.question() && named(one.on(), "chain") {
      if named(one.by(), "rung") && one.kind() != "prompt" {
        self.homes.entry(one.by().to_owned()).or_insert_with(|| one.on().to_owned());
      }
      if one.kind() == "prompt" {
        self.labels.entry(one.on().to_owned()).or_insert_with(|| titled(&text(one, 2)));
      }
    }
  }

  /// What the reader knows of the chains that the record does not hold.
  fn ended(self) -> Known {
    let mut makers = HashMap::new();
    for (on, by) in self.prompts.into_iter().filter(|(on, _)| !self.chains.contains(on)) {
      makers.entry(on).or_insert(by);
    }
    Known { makers, homes: self.homes, labels: self.labels }
  }
}

impl<'a> Reader<'a> {
  /// The reader that tells each fact to out, with what it knows of the chains that the record does not hold.
  fn new(known: Known, out: &'a mut dyn FnMut(Object)) -> Reader<'a> {
    Reader {
      out,
      stage: Vec::new(),
      makers: known.makers,
      homes: known.homes,
      labels: known.labels,
      scopes: HashMap::new(),
      done: HashSet::new(),
      prompts: Vec::new(),
      rungs: HashMap::new(),
      words: HashMap::new(),
      replies: HashMap::new(),
      gates: HashMap::new(),
      runs: HashMap::new(),
      gated: HashSet::new(),
    }
  }
}

/// What the reader knows of the facts that it heard, and where it says the facts that it tells.
struct Reader<'a> {
  out: &'a mut dyn FnMut(Object),
  /// The facts that the reader said and did not tell yet.
  stage: Vec<Object>,
  /// The chain of each rung that a kept act on a chain names as its maker, which is the chain that the rung ran on.
  homes: HashMap<String, String>,
  /// The label of each chain that the record does not hold: the first line of the first prompt that it kept on it.
  labels: HashMap<String, String>,
  /// The maker of each chain that the record does not hold, when a rung makes the first prompt on it.
  makers: HashMap<String, String>,
  /// Each act that a fact names, and the chain of each.
  scopes: HashMap<String, String>,
  /// Each act that is done.
  done: HashSet<String>,
  /// Each prompt that is not done, in order, with its chain, its maker, and whether it goes to the operator.
  prompts: Vec<(String, String, String, bool)>,
  /// The rung of each maker that is not done.
  rungs: HashMap<String, String>,
  /// The word of each rung, the rung of each reply, and the rung of each gate.
  words: HashMap<String, String>,
  replies: HashMap<String, String>,
  gates: HashMap<String, String>,
  /// The run of each rung, and each rung whose word the gate read.
  runs: HashMap<String, String>,
  gated: HashSet<String>,
}

impl Reader<'_> {
  fn hear(&mut self, owned: Fact) {
    let fact = &owned;
    let (kind, about, by) = (fact.kind(), fact.about(), fact.by());
    if fact.question() {
      self.asked(fact);
    }
    // The fact stands after the facts that name it, and before those that it makes the reader say.
    self.flush();
    match kind {
      "rung" if fact.question() && !text(fact, 1).is_empty() => {
        self.words.insert(about.to_owned(), text(fact, 1));
        self.rungs.insert(about.to_owned(), about.to_owned());
      }
      "reply" if fact.question() => {
        self.replies.insert(about.to_owned(), by.to_owned());
      }
      "gate" if fact.question() => {
        let chain = fact.on();
        let worded = self.rungs.values().find(|rung| {
          self.scopes.get(*rung).is_some_and(|one| one == chain)
            && self.words.contains_key(*rung)
            && !self.gated.contains(*rung)
        });
        if let Some(rung) = worded.cloned() {
          self.gated.insert(rung.clone());
          self.gates.insert(about.to_owned(), rung);
        }
      }
      "done" => self.answered(fact),
      "close" if !self.done.contains(about) => {
        let value = fact.word(0).map_or_else(Object::none, |one| one.to_owned());
        self.end(about, value);
      }
      "cancel" if !self.done.contains(about) => {
        self.end(about, Fault::new("CancelledError", vec![]).object());
      }
      _ => {}
    }
    (self.out)(owned.0);
    self.flush();
  }

  /// Tell the facts that the reader said, in order.
  fn flush(&mut self) {
    for one in self.stage.drain(..) {
      (self.out)(one);
    }
  }

  /// A question: the chain and the rung that it names when the reader holds neither, then its own place.
  fn asked(&mut self, fact: &Fact) {
    let (kind, about, by, on) = (fact.kind(), fact.about(), fact.by(), fact.on());
    if kind != "chain" && named(on, "chain") && !self.scopes.contains_key(on) {
      let maker = self.makers.get(on).cloned().unwrap_or_else(|| "operator".to_owned());
      let under =
        self.scopes.get(&maker).or_else(|| self.homes.get(&maker)).cloned().unwrap_or_default();
      let label = self.labels.get(on).cloned().unwrap_or_else(|| on.to_owned());
      self.stage.push(question("chain", on, &maker, [under.as_str(), label.as_str(), ""]));
      self.scopes.insert(on.to_owned(), on.to_owned());
    }
    let chain = self.scope(on);
    if kind != "prompt" && named(by, "rung") && !self.scopes.contains_key(by) {
      self.rung(by, &chain);
    }
    if named(by, "rung") && kind != "reply" {
      self.ran(by);
    }
    let scope = if kind == "chain" { about.to_owned() } else { chain.clone() };
    self.scopes.insert(about.to_owned(), scope);
    if kind == "prompt" {
      self.prompts.push((about.to_owned(), chain, by.to_owned(), text(fact, 3) == "operator"));
    }
  }

  /// The chain that a name stands in: itself for a chain, and the chain of the act that it names for a thread.
  fn scope(&self, on: &str) -> String {
    self.scopes.get(on).cloned().unwrap_or_else(|| on.to_owned())
  }

  /// A rung that the record names as the maker of an act and does not hold, on the open prompt of its chain that is
  /// no question to the operator and that it did not make, or on its chain, which ends the rung before it of the same
  /// maker.
  fn rung(&mut self, id: &str, chain: &str) {
    let maker = self
      .prompts
      .iter()
      .rev()
      .find(|(_, on, by, operator)| on == chain && by != id && !operator)
      .map_or_else(|| chain.to_owned(), |(prompt, ..)| prompt.clone());
    if let Some(last) = self.rungs.remove(&maker) {
      self.end(&last, Object::none());
    }
    self.stage.push(question("rung", id, &maker, [chain, "", "", ""]));
    self.scopes.insert(id.to_owned(), chain.to_owned());
    self.rungs.insert(maker, id.to_owned());
  }

  /// The run of the word of a rung, once, unless the gate refused it.
  fn ran(&mut self, rung: &str) {
    if self.runs.contains_key(rung) || self.done.contains(rung) {
      return;
    }
    let (Some(word), Some(chain)) = (self.words.get(rung), self.scopes.get(rung)) else { return };
    let id = format!("run{}", self.runs.len() + 1);
    self.stage.push(question("run", &id, chain, [chain.as_str(), rung, word.as_str(), ""]));
    self.runs.insert(rung.to_owned(), id);
  }

  /// A done that the record kept: the word of a reply, and the findings of a gate.
  fn answered(&mut self, fact: &Fact) {
    let about = fact.about();
    self.done.insert(about.to_owned());
    self.prompts.retain(|(prompt, ..)| prompt != about);
    if let Some(rung) = self.rungs.remove(about) {
      self.end(&rung, Object::none());
    }
    if let Some(rung) = self.replies.remove(about) {
      let turn = fact.word(0).map(items).unwrap_or_default();
      let word = turn.get(1).and_then(|one| one.as_str().map(str::to_owned));
      if let Some(word) = word {
        self.words.insert(rung, word);
      }
      return;
    }
    let Some(rung) = self.gates.remove(about) else { return };
    let found: Vec<String> = fact
      .word(0)
      .map(items)
      .unwrap_or_default()
      .iter()
      .map(|one| one.as_str().map_or_else(|| one.py_repr(), str::to_owned))
      .collect();
    if found.is_empty() {
      self.ran(&rung);
      return;
    }
    let findings = found.join("\n");
    let name = format!("{rung}_findings");
    let bound = if findings.contains('\n') {
      format!("<s:{name}>\n{findings}</s:{name}>")
    } else {
      format!("{name} = {}", Object::string(findings).py_repr())
    };
    self.stage.push(said(
      "tell",
      &rung,
      [Object::list([Object::string(format!("#{rung} refused\n{bound}"))])],
    ));
    self.end(&rung, Fault::new("Refused", vec![]).object());
  }

  /// An act done with a value, with the rung of it that runs and the run of a rung, as the engine ends them.
  fn end(&mut self, id: &str, value: Object) {
    if !self.done.insert(id.to_owned()) {
      return;
    }
    if let Some(run) = self.runs.get(id).cloned() {
      self.done.insert(run.clone());
      self.stage.push(said("done", &run, [Object::none()]));
    }
    self.stage.push(said("done", id, [value]));
    self.prompts.retain(|(prompt, ..)| prompt != id);
    if let Some(rung) = self.rungs.remove(id) {
      self.end(&rung, Object::none());
    }
    self.rungs.retain(|_, rung| rung != id);
  }
}

#[cfg(test)]
#[path = "reader.test.rs"]
mod tests;
