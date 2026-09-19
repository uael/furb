//! The World and the Kernel, as a host outside python gives them.
//!
//! The engine takes the World and the Kernel as generators, each of which hears every fact and may say facts of
//! its own. A host that is not python is no generator, so the crate runs `src/preamble.py` in a module of its own
//! and gives the engine the two generators that stand in their place. What a host implements is [`World`], and
//! what it answers with is a [`Reply`].
//!
//! A World that must ask the engine something while it answers, as it does for the working directory of a chain
//! before it opens a path, answers [`Reply::Ask`] and is given the answer through [`World::answered`]. It never
//! calls into a life that stands waiting for it, and it never says a fact itself: it hands back what it would say,
//! and the boundary says each of them in order, which is the one way the outside speaks.

use crate::fact::{Fact, Value};

/// What a World or a Kernel answers when it has heard a fact.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
  /// Nothing at all, which is the answer to every fact the outside only hears.
  Nothing,
  /// These facts, said in this order, each as the bus makes it whole.
  Say(Vec<Fact>),
  /// A question of the engine, which the boundary puts and hands back through [`World::answered`].
  Ask {
    /// The kind of the question, such as `cwd` or `merged`.
    kind: String,
    /// The chain the question is on.
    on: String,
    /// The words of the question.
    words: Vec<Value>,
  },
}

impl Reply {
  /// One fact and nothing else.
  pub fn say(one: Fact) -> Self {
    Reply::Say(vec![one])
  }

  /// A question of the engine on a chain, with no words but the chain.
  pub fn ask(kind: impl Into<String>, on: impl Into<String>) -> Self {
    Reply::Ask { kind: kind.into(), on: on.into(), words: Vec::new() }
  }
}

/// The interface to the disk, the machine, the actors and the record.
///
/// The World hears every fact. It answers what a chain stands on, a reading of the clock, a number it draws, and
/// a read or a write of a path nobody of the engine serves; it starts what it is started to do, which is a
/// command, a wait and a prompt of the operator; it answers an ask with the turn of a model; and it keeps what
/// the journal says to keep.
///
/// Everything it answers is plain, and everything it hears is plain, so a World holds nothing of python.
pub trait World {
  /// One fact, heard. What the World would say of it, or the question it must ask first.
  fn hears(&mut self, fact: &Fact) -> Reply;

  /// The answer to the question the World last asked, and what it says now that it holds it.
  ///
  /// A World that never answers [`Reply::Ask`] never hears this, so it stands answered by nothing.
  fn answered(&mut self, got: &Value) -> Reply {
    let _ = got;
    Reply::Nothing
  }
}

/// The interface that gates and runs the words of a chain.
///
/// The crate carries one of these, which gates with the type checker and runs the word in the module of its chain,
/// so a host needs none of its own. It stands here all the same, since the engine names the Kernel beside the
/// World, and a host that wants its own gate gives one.
pub trait Kernel {
  /// One fact, heard: a gate to answer, a run to begin, a sent to carry forward, or a control that drops a frame.
  ///
  /// A gate is a question like any other, so it comes here and is answered here, and the Kernel has one way in.
  fn hears(&mut self, fact: &Fact) -> Reply;
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A World of the test that answers a stand and asks the engine where a read resolves.
  struct Sand {
    asked: Vec<String>,
  }

  impl World for Sand {
    fn hears(&mut self, fact: &Fact) -> Reply {
      match fact.kind() {
        "stand" => Reply::say(Fact::new(
          "done",
          fact.about(),
          "world",
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
      self.asked.push(got.as_str().unwrap_or_default().to_owned());
      Reply::Nothing
    }
  }

  #[test]
  fn a_world_answers_a_stand_and_asks_the_chain_where_its_paths_resolve() {
    let mut sand = Sand { asked: Vec::new() };
    let stand = Fact::new(
      "stand",
      "stand://operator.1.1",
      "operator",
      vec![Value::Str("chain://operator.1".to_owned())],
    );
    assert!(matches!(sand.hears(&stand), Reply::Say(said) if said.len() == 1));
    let read = Fact::new(
      "read",
      "read://operator.1.2",
      "operator",
      vec![Value::Str("chain://operator.1".to_owned()), Value::Str("a.txt".to_owned())],
    );
    assert_eq!(sand.hears(&read), Reply::ask("cwd", "chain://operator.1"));
    assert_eq!(sand.answered(&Value::Str("/w".to_owned())), Reply::Nothing);
    assert_eq!(sand.asked, vec!["/w".to_owned()]);
  }
}
