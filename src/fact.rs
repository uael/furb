//! A fact, as the engine says one: a tuple whose first entries are its kind, what it is about and who said it.
//!
//! The contract says a fact is `(kind, about, by, *words)`, and a question is `(kind, id, by, on, *words)`. The
//! crate reads a fact as the tuple it is and nothing more: what a kind means is the engine's, and a host that
//! hears facts reads the ones it knows by position, as the contract declares them.

use crate::value::{Object, ObjectRef, entry};

/// One fact of a life, as the tuple the engine said.
#[derive(Debug, Clone)]
pub struct Fact(pub Object);

impl Fact {
  /// A saying: a kind, what it is about, and its words, with no slot for who said it, since the bus says that.
  /// It is what an ear yields and what a Voice says, and it is read by the engine and never by the host.
  pub fn says(kind: &str, about: &str, words: impl IntoIterator<Item = Object>) -> Self {
    let mut held = vec![Object::string(kind), Object::string(about)];
    held.extend(words);
    Fact(Object::tuple(held))
  }

  /// One fact or saying from an object, which must be a tuple whose kind and about are text: a fact has who said
  /// it third, and a saying has its first word there.
  pub fn of(said: ObjectRef<'_>) -> Option<Fact> {
    let held = said.items()?;
    (held.len() >= 2 && held[..2].iter().all(|one| one.as_str().is_some()))
      .then(|| Fact(said.to_owned()))
  }

  /// The kind of the fact, which is its first entry.
  pub fn kind(&self) -> &str {
    self.at(0)
  }

  /// What the fact is about: the name of an act, of a question, or of a chain.
  pub fn about(&self) -> &str {
    self.at(1)
  }

  /// Who said the fact: the operator, the World, or the act that spoke.
  pub fn by(&self) -> &str {
    self.at(2)
  }

  /// One word of the fact, by its place after who said it.
  pub fn word(&self, at: usize) -> Option<ObjectRef<'_>> {
    entry(&self.0.as_ref(), 3 + at)
  }

  /// The chain a question is on, which is its first word.
  pub fn on(&self) -> &str {
    self.word(0).and_then(|one| one.as_str()).unwrap_or_default()
  }

  /// Whether the fact is a question: one whose name is its kind and a number, as every act is named.
  pub fn question(&self) -> bool {
    self
      .about()
      .strip_prefix(self.kind())
      .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|one| one.is_ascii_digit()))
  }

  fn at(&self, i: usize) -> &str {
    entry(&self.0.as_ref(), i).and_then(|one| one.as_str()).unwrap_or_default()
  }
}
