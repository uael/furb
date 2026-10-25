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
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{
  fact::{Fact, named},
  value::{Fault, IS, Object, ObjectRef, entry, field},
};

use super::{kept, logged, made, version};

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
  /// The facts, in the order that the life said them.
  pub facts: Vec<Object>,
  /// The count of the entries of the record.
  pub entries: usize,
  /// The sha256 of the last entry of the record, in the plain form that the record keeps, and nothing for a record
  /// with no entry.
  pub last: Option<String>,
}

/// What the record at a path tells, read with no lease and changed in nothing.
pub fn retold(path: impl AsRef<Path>) -> Result<Retold, Fault> {
  let path = path.as_ref();
  let record = kept(path)?;
  let (made, version) = (made(&record), version(&record));
  let (entries, last) = (record.len(), record.last().map(digest));
  if let Some(facts) = logged(path)? {
    return Ok(Retold { made, version, logged: true, facts, entries, last });
  }
  let facts = record
    .iter()
    .filter_map(|entry| Some(entry.as_ref().items()?.first()?.to_owned()))
    .collect::<Vec<_>>();
  Ok(Retold { made, version, logged: false, facts: told(&facts), entries, last })
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
pub fn told(kept: &[Object]) -> Vec<Object> {
  let facts: Vec<Fact> = kept.iter().filter_map(|one| Fact::of(one.as_ref())).collect();
  let mut makers: HashMap<String, String> = HashMap::new();
  let chains: HashSet<&str> =
    facts.iter().filter(|one| one.question() && one.kind() == "chain").map(Fact::about).collect();
  for one in
    facts.iter().filter(|one| one.question() && one.kind() == "prompt" && named(one.by(), "rung"))
  {
    if !chains.contains(one.on()) {
      makers.entry(one.on().to_owned()).or_insert_with(|| one.by().to_owned());
    }
  }
  let mut homes: HashMap<String, String> = HashMap::new();
  let mut labels: HashMap<String, String> = HashMap::new();
  for one in facts.iter().filter(|one| one.question() && named(one.on(), "chain")) {
    if named(one.by(), "rung") && one.kind() != "prompt" {
      homes.entry(one.by().to_owned()).or_insert_with(|| one.on().to_owned());
    }
    if one.kind() == "prompt" {
      labels.entry(one.on().to_owned()).or_insert_with(|| titled(&text(one, 2)));
    }
  }
  let mut life = Reader { makers, homes, labels, ..Reader::default() };
  for fact in &facts {
    life.hear(fact);
  }
  life.out
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

/// What the reader knows of the facts that it heard.
#[derive(Default)]
struct Reader {
  out: Vec<Object>,
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

impl Reader {
  fn hear(&mut self, fact: &Fact) {
    let (kind, about, by) = (fact.kind(), fact.about(), fact.by());
    if fact.question() {
      self.asked(fact);
    }
    self.out.push(fact.0.clone());
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
  }

  /// A question: the chain and the rung that it names when the reader holds neither, then its own place.
  fn asked(&mut self, fact: &Fact) {
    let (kind, about, by, on) = (fact.kind(), fact.about(), fact.by(), fact.on());
    if kind != "chain" && named(on, "chain") && !self.scopes.contains_key(on) {
      let maker = self.makers.get(on).cloned().unwrap_or_else(|| "operator".to_owned());
      let under =
        self.scopes.get(&maker).or_else(|| self.homes.get(&maker)).cloned().unwrap_or_default();
      let label = self.labels.get(on).cloned().unwrap_or_else(|| on.to_owned());
      self.out.push(question("chain", on, &maker, [under.as_str(), label.as_str(), ""]));
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
    self.out.push(question("rung", id, &maker, [chain, "", "", ""]));
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
    self.out.push(question("run", &id, chain, [chain.as_str(), rung, word.as_str(), ""]));
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
    self.out.push(said(
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
      self.out.push(said("done", &run, [Object::none()]));
    }
    self.out.push(said("done", id, [value]));
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
