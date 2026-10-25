//! The ear of the compaction extension, which answers a carry with what a chain of another record holds, read with
//! no life, so that a chain of this life goes on from it.

use std::{
  collections::{HashMap, HashSet},
  path::Path,
};

use crate::{
  ear::{Ear, hear, hearing, say},
  fact::Fact,
  value::{Fault, Object},
  world::{Retold, here, resolved, retold},
};

/// The name that each fact that a carry gives is about: a reply and a number, as the turns of a chain read a reply,
/// which no act of a life takes, so no carried fact stands for an act of this life.
pub const CARRIED: &str = "reply0";

/// The ear of the compaction: it answers a carry of a record and of a chain, at a path that the chain of the carry
/// resolves, with a dict: `record`, the path; `entries`, the count of its entries; `digest`, the sha256 of its last
/// entry; `logged`, whether its log gave the facts; `turns`, each tell and each reply of the chain with its done, as
/// its model read them after the last prefix said on it; and `prompts`, each open prompt of a model on that chain, as
/// `(name, shape, message, to)`. Each carried fact is about [`CARRIED`] and said by the carry, so a fork keeps the
/// turns whole and a filter of the extension keeps or drops them with the carry. A record of no log gives its replies and no tell, since a record keeps no
/// tell. It refuses a carry of a path that holds no record, of a record that it cannot read, or of a chain that the
/// record does not hold.
pub fn compaction() -> Box<dyn Ear> {
  hearing(&["carry"], |mut co| async move {
    loop {
      let a = hear(&mut co).await;
      if a.kind() != "carry" || !a.question() {
        continue;
      }
      let word =
        |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      let (about, record, chain) = (a.about().to_owned(), word(1), word(2));
      let path = resolved(&here(&mut co, a.on()).await?, &record);
      let none = Fault::refused(format!("No record at {}.", path.display()));
      let read = |path: &Path| {
        retold(path).map_err(|no| Fault::refused(format!("{}: {}", path.display(), no.message())))
      };
      let got = (if path.is_file() { read(&path) } else { Err(none) })
        .and_then(|told| carried(&told, &path, &chain, &about, a.on()));
      say(&mut co, Fact::says("done", &about, [got.unwrap_or_else(|no| no.object())]));
    }
  })
}

/// What a carry under the name about, asked on a chain on, gives of the chain of a record that the reader told.
fn carried(
  told: &Retold,
  path: &Path,
  chain: &str,
  about: &str,
  on: &str,
) -> Result<Object, Fault> {
  let (held, done) = held(&told.facts()?, chain).ok_or_else(|| {
    Fault::refused(format!("the record {} holds no chain {chain}", path.display()))
  })?;
  let replies: HashSet<&str> =
    held.iter().filter(|one| one.question() && one.kind() == "reply").map(Fact::about).collect();
  let text = |one: &str| Object::string(one);
  let word = |fact: &Fact, at: usize| fact.word(at).map_or_else(Object::none, |one| one.to_owned());
  let mut turns = Vec::new();
  for fact in &held {
    match fact.kind() {
      "tell" => {
        turns.push(Object::tuple([text("tell"), text(CARRIED), text(about), word(fact, 0)]))
      }
      "reply" if fact.question() => {
        turns.push(Object::tuple([
          text("reply"),
          text(CARRIED),
          text(about),
          text(on),
          word(fact, 1),
        ]));
      }
      "done" if replies.contains(fact.about()) => {
        turns.push(Object::tuple([text("done"), text(CARRIED), text(about), word(fact, 0)]));
      }
      _ => {}
    }
  }
  let open = |one: &&Fact| {
    one.question()
      && one.kind() == "prompt"
      && !done.contains(one.about())
      && fact_text(one, 3) != "operator"
  };
  let prompts = held
    .iter()
    .filter(open)
    .map(|one| Object::tuple([text(one.about()), word(one, 1), word(one, 2), word(one, 3)]));
  Ok(Object::dict([
    (text("record"), text(&path.display().to_string())),
    (text("entries"), Object::int(i64::try_from(told.entries).unwrap_or(i64::MAX))),
    (text("digest"), told.last.as_deref().map_or_else(Object::none, text)),
    (text("logged"), Object::bool(told.logged)),
    (text("turns"), Object::list(turns)),
    (text("prompts"), Object::list(prompts)),
  ]))
}

/// A word of a fact that is text, and an empty text when it is none.
fn fact_text(fact: &Fact, at: usize) -> String {
  fact.word(at).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default()
}

/// What the transcript of a chain holds at the end of the facts of a life, as the engine holds it: each question on
/// it, each other fact about an act of it, and what the last prefix said on it holds in place of what came before;
/// and each act that is done. Nothing when no chain of that name was made.
fn held(facts: &[Object], chain: &str) -> Option<(Vec<Fact>, HashSet<String>)> {
  let mut scopes: HashMap<String, String> = HashMap::new();
  let (mut held, mut done) = (Vec::new(), HashSet::new());
  for fact in facts.iter().filter_map(|one| Fact::of(one.as_ref())) {
    let at = if fact.question() {
      let at = match fact.kind() {
        "chain" => fact.about().to_owned(),
        _ => scopes.get(fact.on()).cloned().unwrap_or_default(),
      };
      scopes.insert(fact.about().to_owned(), at.clone());
      at
    } else {
      if fact.kind() == "done" {
        done.insert(fact.about().to_owned());
      }
      scopes.get(fact.about()).cloned().unwrap_or_default()
    };
    if at != chain {
      continue;
    }
    if fact.kind() == "prefix" {
      let said = fact.word(0).and_then(|one| one.items()).unwrap_or_default();
      held = said.into_iter().filter_map(Fact::of).collect();
    } else {
      held.push(fact);
    }
  }
  (scopes.get(chain).map(String::as_str) == Some(chain)).then_some((held, done))
}

#[cfg(test)]
#[path = "compaction.test.rs"]
mod tests;
