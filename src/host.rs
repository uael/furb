//! The host side of the boundary, which answers what the sandbox asks of it.
//!
//! `src/preamble.py` runs in the sandbox and stands in for the two generators the engine takes. Every call it
//! makes comes here: one name for the World, which hears facts, and one for the Kernel, which asks the one
//! question the Kernel cannot answer for itself.
//!
//! The two speak differently, because they ask differently. The World hears a fact and answers with facts to say
//! or with a question of its own, so its answer is a reply. The Kernel asks whether a word may run, which is a
//! question with a value for an answer, so its answer is the findings.
//!
//! What a host said into its [`Voice`] since the last call rides out with the next answer, so the facts of a
//! command that is running reach the life without the host calling into it.

use crate::{
  fact::{Fact, Value},
  voice::Ears,
  world::{Reply, World},
};

/// The name the World hears under, which is the name the engine takes it under.
pub const WORLD: &str = "world";
/// The name the Kernel of the crate asks under.
pub const KERNEL: &str = "kernel";

/// What reads a word before it runs.
///
/// The Kernel runs the word of a rung where the engine runs, so the one thing it cannot answer for itself is
/// whether the word may run at all: what a word is read against is the host's to decide.
pub trait Gate {
  /// What the gate finds against a word, read against the ladder of its chain and the shape it must give.
  ///
  /// Nothing at all means the word may run.
  fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String>;
}

/// The outside of one life: the World it hears through, the gate it is read by, and what its host has said.
#[derive(Debug)]
pub struct Outside<W, G> {
  /// The World of this life.
  pub world: W,
  /// What reads the word of every rung of this life.
  pub gate: G,
  /// What the host has said and the life has not heard.
  ears: Ears,
}

impl<W: World, G: Gate> Outside<W, G> {
  /// The outside of one life.
  pub fn new(world: W, gate: G, ears: Ears) -> Self {
    Outside { world, gate, ears }
  }

  /// One call of the sandbox, answered.
  ///
  /// `said` is the plain form of what the sandbox handed over, and what comes back is plain the same way.
  pub fn called(&mut self, name: &str, said: &Value) -> Value {
    let held = Value::of_plain(said);
    let words = held.as_entries().unwrap_or_default().to_vec();
    if name == KERNEL {
      return self.gated(&words);
    }
    let reply = match words.first().and_then(Value::as_str) {
      Some("answered") => self.world.answered(words.get(1).unwrap_or(&Value::None)),
      _ => self.world.hears(&Fact(words)),
    };
    self.replied(reply)
  }

  /// Everything the host has said and the life has not heard, which rides out with the next answer.
  pub fn drained(&mut self) -> Vec<Fact> {
    self.ears.drained()
  }

  /// What the gate found against a word, which is the answer to the one question the Kernel asks.
  fn gated(&mut self, words: &[Value]) -> Value {
    let word = words.get(1).and_then(Value::as_str).unwrap_or_default();
    let ladder: Vec<String> = words
      .get(2)
      .and_then(Value::as_entries)
      .unwrap_or_default()
      .iter()
      .map(|one| one.as_str().unwrap_or_default().to_owned())
      .collect();
    let shape = words.get(3).and_then(Value::as_str).unwrap_or_default();
    let found = self.gate.gate(word, &ladder, shape);
    Value::List(found.into_iter().map(Value::Str).collect()).plain()
  }

  /// One reply of the World, as the sandbox reads it, with whatever the host said meanwhile said after it.
  fn replied(&mut self, reply: Reply) -> Value {
    let mut said = match reply {
      Reply::Say(held) => held,
      Reply::Nothing => Vec::new(),
      Reply::Ask { kind, on, words } => {
        let asked = vec![Value::Str("ask".to_owned()), Value::Str(kind), Value::Str(on), Value::List(words)];
        return Value::Tuple(asked).plain();
      }
    };
    said.extend(self.ears.drained());
    let held = said.into_iter().map(|one| Value::List(one.0)).collect();
    Value::Tuple(vec![Value::Str("say".to_owned()), Value::List(held)]).plain()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::voice::Voice;

  /// A World of the test: it answers a stand, asks where a read resolves, and keeps what it heard.
  #[derive(Default)]
  struct Sand {
    heard: Vec<String>,
    answered: Vec<Value>,
  }

  impl World for Sand {
    fn hears(&mut self, fact: &Fact) -> Reply {
      self.heard.push(fact.kind().to_owned());
      match fact.kind() {
        "stand" => Reply::say(Fact::new(
          "done",
          fact.about(),
          WORLD,
          vec![Value::Tuple(vec![
            Value::Tuple(vec![]),
            Value::Str("/w".to_owned()),
            Value::Str("m/low".to_owned()),
          ])],
        )),
        "read" => Reply::ask("cwd", fact.on().unwrap_or_default()),
        _ => Reply::Nothing,
      }
    }

    fn answered(&mut self, got: &Value) -> Reply {
      self.answered.push(got.clone());
      Reply::say(Fact::new("done", "read://operator.1.1", WORLD, vec![Value::text("/w/a.txt", "one\n")]))
    }
  }

  /// A gate of the test: it refuses a word that holds BAD.
  #[derive(Default)]
  struct Strict {
    read: Vec<(String, usize, String)>,
  }

  impl Gate for Strict {
    fn gate(&mut self, word: &str, ladder: &[String], shape: &str) -> Vec<String> {
      self.read.push((word.to_owned(), ladder.len(), shape.to_owned()));
      if word.contains("BAD") { vec!["BAD in rung".to_owned()] } else { Vec::new() }
    }
  }

  /// One fact, as the sandbox hands it over.
  fn said(kind: &str, about: &str, words: Vec<Value>) -> Value {
    let mut held = vec![Value::Str(kind.to_owned()), Value::Str(about.to_owned()), Value::Str("chain".to_owned())];
    held.extend(words);
    Value::Tuple(held).plain()
  }

  #[test]
  fn the_world_hears_a_fact_and_what_it_says_goes_back_plain() {
    let (_, ears) = Ears::made();
    let mut outside = Outside::new(Sand::default(), Strict::default(), ears);
    let got = outside.called(WORLD, &said("stand", "stand://operator.1.1", vec![Value::Str("chain".to_owned())]));
    let held = Value::of_plain(&got);
    let words = held.as_entries().unwrap();
    assert_eq!(words[0], Value::Str("say".to_owned()));
    let facts = words[1].as_entries().unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].as_entries().unwrap()[0], Value::Str("done".to_owned()));
    assert_eq!(outside.world.heard, vec!["stand".to_owned()]);
  }

  #[test]
  fn a_world_that_must_ask_the_engine_says_so_and_is_given_the_answer() {
    let (_, ears) = Ears::made();
    let mut outside = Outside::new(Sand::default(), Strict::default(), ears);
    let asked = outside.called(
      WORLD,
      &said("read", "read://operator.1.1", vec![Value::Str("chain://operator.1".to_owned())]),
    );
    let held = Value::of_plain(&asked);
    let words = held.as_entries().unwrap();
    assert_eq!(words[0], Value::Str("ask".to_owned()));
    assert_eq!(words[1], Value::Str("cwd".to_owned()));
    assert_eq!(words[2], Value::Str("chain://operator.1".to_owned()));
    let back = outside.called(
      WORLD,
      &Value::Tuple(vec![Value::Str("answered".to_owned()), Value::Str("/w".to_owned())]).plain(),
    );
    assert_eq!(outside.world.answered, vec![Value::Str("/w".to_owned())]);
    let done = Value::of_plain(&back);
    let facts = done.as_entries().unwrap()[1].as_entries().unwrap();
    assert_eq!(facts[0].as_entries().unwrap()[3].field("path"), Some(&Value::Str("/w/a.txt".to_owned())));
  }

  #[test]
  fn the_kernel_asks_what_the_gate_finds_and_is_answered_with_the_findings() {
    let (_, ears) = Ears::made();
    let mut outside = Outside::new(Sand::default(), Strict::default(), ears);
    let asked = Value::Tuple(vec![
      Value::Str("gate".to_owned()),
      Value::Str("close(BAD)".to_owned()),
      Value::List(vec![Value::Str("k = 1".to_owned())]),
      Value::Str("int".to_owned()),
    ])
    .plain();
    let got = outside.called(KERNEL, &asked);
    assert_eq!(got, Value::List(vec![Value::Str("BAD in rung".to_owned())]));
    assert_eq!(outside.gate.read, vec![("close(BAD)".to_owned(), 1, "int".to_owned())]);
  }

  #[test]
  fn what_a_host_said_meanwhile_rides_out_with_the_next_answer() {
    let (voice, ears) = Ears::made();
    let mut outside = Outside::new(Sand::default(), Strict::default(), ears);
    voice.say(Fact::new("out", "bash://operator.1.1", WORLD, vec![Value::Str("one\n".to_owned())]));
    let got = outside.called(WORLD, &said("keep", "", vec![]));
    let held = Value::of_plain(&got);
    let facts = held.as_entries().unwrap()[1].as_entries().unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].as_entries().unwrap()[0], Value::Str("out".to_owned()));
  }
}
