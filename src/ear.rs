//! What a host writes: an ear, which hears every fact of a life and says what it will.
//!
//! The engine takes the World, and any other generator of the outside, as an ear: it hears every fact that is
//! said and speaks by yielding a saying, or by calling a verb of the engine. A host that is not python is no
//! generator, so [`PREAMBLE`](crate::PREAMBLE) stands in its place inside the sandbox and reaches the host
//! through one call: the name of the ear and one plain value go in, and one plain value comes back. That call is
//! [`Host`], and [`Ear`] is the typed way to answer it.
//!
//! An ear answers where it is asked. What it says of a fact is one saying, which the bus says whole and hands
//! back to the ear as the next fact it hears, so an ear that has two things to say says the second on hearing
//! the first. What an ear must read of the engine while it answers, as the World reads where the paths of a
//! chain resolve before it opens a path, it reads by [`Reply::Reads`]: one word, run in the names of the
//! engine, whose value comes back through [`Ear::answered`]. An ear never calls into a life that stands waiting
//! for it.

use crate::fact::{Fact, Value};

/// What an ear answers when it has heard a fact.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
  /// Nothing at all, which is the answer to every fact the ear only hears.
  Nothing,
  /// One saying, which the bus makes whole and hands back to the ear as the next fact it hears.
  Say(Fact),
  /// One word, read in the names of the engine, whose value comes back through [`Ear::answered`].
  Reads(String),
}

impl Reply {
  /// One word, read in the names of the engine.
  pub fn reads(word: impl Into<String>) -> Self {
    Reply::Reads(word.into())
  }
}

/// A generator of the outside, as a host that is not python writes one.
///
/// The World is one: it answers what a chain stands on, a reading of the clock, a number it draws, and a read or
/// a write of a path nobody of the engine serves; it starts what it is started to do, which is a command, a wait
/// and a prompt of the operator; it answers an ask with the turn of a model; and it keeps what the journal says
/// to keep. Everything it hears and everything it answers is plain, so an ear holds nothing of python.
pub trait Ear {
  /// One fact, heard. What the ear says of it, or the word it must read first.
  fn hears(&mut self, fact: &Fact) -> Reply;

  /// The value of the word the ear last read, and what it says now that it holds it.
  ///
  /// An ear that never reads never hears this, so it stands answered by nothing.
  fn answered(&mut self, got: &Value) -> Reply {
    let _ = got;
    Reply::Nothing
  }
}

/// What answers the sandbox while its code runs: the name of the ear that is asked, and what it was handed.
///
/// This is the whole of the boundary, and [`Ears`] is the one the crate provides. A host that reaches the
/// sandbox from another language writes its own, since what it is handed is plain and so is what it answers.
pub trait Host {
  /// One call of the sandbox, answered: the name of the ear it is for, and what it handed over, plain.
  fn called(&mut self, name: &str, said: &Value) -> Value;
}

/// The ears of a host, each under the name the engine hears it by.
///
/// The name is the one the engine binds the ear under, so the World is `world`, and a fact the ear says is said
/// by that name.
#[derive(Default)]
pub struct Ears {
  held: Vec<(String, Box<dyn Ear>)>,
}

impl Ears {
  /// No ear at all, which is a life the host only drives and never hears.
  #[must_use]
  pub fn new() -> Self {
    Ears::default()
  }

  /// One more ear, under the name the engine hears it by.
  #[must_use]
  pub fn with(mut self, name: impl Into<String>, ear: impl Ear + 'static) -> Self {
    self.held.push((name.into(), Box::new(ear)));
    self
  }

  /// The names of the ears, in the order the engine hears them.
  pub fn names(&self) -> Vec<String> {
    self.held.iter().map(|(name, _)| name.clone()).collect()
  }
}

impl std::fmt::Debug for Ears {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_list().entries(self.names()).finish()
  }
}

impl Host for Ears {
  fn called(&mut self, name: &str, said: &Value) -> Value {
    let Some((_, ear)) = self.held.iter_mut().find(|(held, _)| held == name) else {
      return raised(&format!("no ear of the host hears as {name}"));
    };
    let held = Value::of_plain(said);
    let reply = match held.field("answered") {
      Some(got) => ear.answered(got),
      None => match held.as_entries() {
        Some(words) => ear.hears(&Fact(words.to_vec())),
        None => Reply::Nothing,
      },
    };
    match reply {
      Reply::Nothing => Value::None,
      Reply::Say(one) => Value::Map(vec![("say".to_owned(), Value::List(one.said()).plain())]),
      Reply::Reads(word) => Value::Map(vec![("reads".to_owned(), Value::Str(word))]),
    }
  }
}

/// What the sandbox raises in the ear that spoke when the host cannot answer the call.
fn raised(why: &str) -> Value {
  Value::Map(vec![("raised".to_owned(), Value::refused(why).plain())])
}

#[cfg(test)]
mod tests {
  use super::*;

  /// An ear of the test: it answers a stand, and it reads where a chain stands before it answers a read.
  #[derive(Default)]
  struct Sand {
    heard: Vec<String>,
    read: Vec<Value>,
  }

  impl Ear for Sand {
    fn hears(&mut self, fact: &Fact) -> Reply {
      self.heard.push(fact.kind().to_owned());
      match fact.kind() {
        "stand" => Reply::Say(Fact::says(
          "done",
          fact.about(),
          vec![Value::Tuple(vec![Value::Tuple(vec![]), Value::Str("/w".to_owned()), Value::Str("m/low".to_owned())])],
        )),
        "read" => Reply::reads(format!("cwd(on={:?})", fact.on().unwrap_or_default())),
        _ => Reply::Nothing,
      }
    }

    fn answered(&mut self, got: &Value) -> Reply {
      self.read.push(got.clone());
      Reply::Say(Fact::says("done", "read://operator.1.1", vec![Value::text("/w/a.txt", "one\n")]))
    }
  }

  /// One fact, as the sandbox hands it over.
  fn said(kind: &str, about: &str, words: Vec<Value>) -> Value {
    let mut held = vec![Value::Str(kind.to_owned()), Value::Str(about.to_owned()), Value::Str("chain".to_owned())];
    held.extend(words);
    Value::Tuple(held).plain()
  }

  #[test]
  fn an_ear_hears_a_fact_and_what_it_says_goes_back_plain() {
    let mut ears = Ears::new().with("world", Sand::default());
    let got = ears.called("world", &said("stand", "stand://operator.1.1", vec![Value::Str("chain".to_owned())]));
    let saying = Value::of_plain(got.field("say").unwrap());
    let words = saying.as_entries().unwrap();
    assert_eq!(words[0], Value::Str("done".to_owned()));
    assert_eq!(words[1], Value::Str("stand://operator.1.1".to_owned()));
    assert!(matches!(words[2], Value::Tuple(_)));
    assert_eq!(ears.names(), vec!["world".to_owned()]);
  }

  #[test]
  fn an_ear_that_must_read_the_engine_says_the_word_and_is_given_its_value() {
    let mut ears = Ears::new().with("world", Sand::default());
    let asked =
      ears.called("world", &said("read", "read://operator.1.1", vec![Value::Str("chain://operator.1".to_owned())]));
    assert_eq!(asked.field("reads"), Some(&Value::Str("cwd(on=\"chain://operator.1\")".to_owned())));
    let back = ears.called("world", &Value::Map(vec![("answered".to_owned(), Value::Str("/w".to_owned()))]));
    let saying = Value::of_plain(back.field("say").unwrap());
    assert_eq!(saying.as_entries().unwrap()[2].field("path"), Some(&Value::Str("/w/a.txt".to_owned())));
  }

  #[test]
  fn a_call_for_no_ear_of_the_host_is_raised_in_the_sandbox() {
    let mut ears = Ears::new();
    let got = ears.called("nobody", &said("stand", "stand://operator.1.1", vec![]));
    assert!(got.field("raised").is_some());
  }
}
