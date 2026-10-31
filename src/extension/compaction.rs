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
  world::{Retold, files, here, retold},
};

/// The name that each fact that a carry gives is about: a reply and a number, as the turns of a chain read a reply,
/// which no act of a life takes, so no carried fact stands for an act of this life.
pub const CARRIED: &str = "reply0";

/// The ear of the compaction: it answers a carry of a record and of a chain, at a path that the chain of the carry
/// resolves, with a dict: `record`, the path, with `~` for the home of the user; `entries`, the count of its entries; `digest`, the sha256 of its last
/// entry; `logged`, whether its log gave the facts; `turns`, each tell and each reply of the chain with its done, as
/// its model read them after the last prefix said on it; and `prompts`, each open prompt of a model on that chain, as
/// `(name, shape, message, to)`. When the log does not give the facts, the record shows no close that a word gave, so
/// `prompts` holds only each open prompt that no rung of a model worked on. Each carried fact is about [`CARRIED`] and said by the carry, so a fork keeps the
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
      let got = match files::placed(&here(&mut co, a.on()).await?, &record) {
        Ok(path) => carry(&path, &chain, &about, a.on()),
        Err(no) => Err(no),
      };
      say(&mut co, Fact::says("done", &about, [got.unwrap_or_else(|no| no.object())]));
    }
  })
}

/// A path of a record as the World writes it for a model.
fn shown(path: &Path) -> String {
  files::tilded(path, std::env::home_dir().as_deref())
}

/// What a carry under the name about, asked on a chain on, gives of a chain of the record at a path.
pub(crate) fn carry(path: &Path, chain: &str, about: &str, on: &str) -> Result<Object, Fault> {
  if !path.is_file() {
    return Err(Fault::refused(format!("No record at {}.", shown(path))));
  }
  let failed = |no: Fault| Fault::refused(format!("{}: {}", shown(path), no.message()));
  let told = retold(path).map_err(failed)?;
  let mut fold = Held::new(chain);
  told.each(&mut |one| fold.hear(one)).map_err(failed)?;
  carried(&told, fold, path, chain, about, on)
}

/// What a carry under the name about, asked on a chain on, gives of the chain of a record that the reader told.
fn carried(
  told: &Retold,
  fold: Held,
  path: &Path,
  chain: &str,
  about: &str,
  on: &str,
) -> Result<Object, Fault> {
  let (held, done) = fold
    .ended()
    .ok_or_else(|| Fault::refused(format!("the record {} holds no chain {chain}", shown(path))))?;
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
  let worked: HashSet<&str> = if told.logged {
    HashSet::new()
  } else {
    held.iter().filter(|one| one.question() && one.kind() == "rung").map(Fact::by).collect()
  };
  let open = |one: &&Fact| {
    one.question()
      && one.kind() == "prompt"
      && !done.contains(one.about())
      && !worked.contains(one.about())
      && fact_text(one, 3) != "operator"
  };
  let prompts = held
    .iter()
    .filter(open)
    .map(|one| Object::tuple([text(one.about()), word(one, 1), word(one, 2), word(one, 3)]));
  Ok(Object::dict([
    (text("record"), text(&shown(path))),
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

/// What the transcript of a chain holds, as the engine holds it, of the facts of a life that it heard in order: each
/// question on it, each other fact about an act of it, and what the last prefix said on it holds in place of what
/// came before; and each act that is done.
struct Held {
  chain: String,
  scopes: HashMap<String, String>,
  held: Vec<Fact>,
  done: HashSet<String>,
}

impl Held {
  fn new(chain: &str) -> Held {
    Held { chain: chain.to_owned(), scopes: HashMap::new(), held: Vec::new(), done: HashSet::new() }
  }

  /// The next fact of the life.
  fn hear(&mut self, one: Object) {
    let Some(fact) = Fact::of(one.as_ref()) else { return };
    let at = if fact.question() {
      let at = match fact.kind() {
        "chain" => fact.about().to_owned(),
        _ => self.scopes.get(fact.on()).cloned().unwrap_or_default(),
      };
      self.scopes.insert(fact.about().to_owned(), at.clone());
      at
    } else {
      if fact.kind() == "done" {
        self.done.insert(fact.about().to_owned());
      }
      self.scopes.get(fact.about()).cloned().unwrap_or_default()
    };
    if at != self.chain {
      return;
    }
    if fact.kind() == "prefix" {
      let said = fact.word(0).and_then(|one| one.items()).unwrap_or_default();
      self.held = said.into_iter().filter_map(Fact::of).collect();
    } else {
      self.held.push(fact);
    }
  }

  /// What the transcript holds at the end of the facts, and each act that is done, or nothing when no chain of that
  /// name was made.
  fn ended(self) -> Option<(Vec<Fact>, HashSet<String>)> {
    let made = self.scopes.get(&self.chain) == Some(&self.chain);
    made.then_some((self.held, self.done))
  }
}

#[cfg(test)]
#[path = "compaction.test.rs"]
mod tests;
