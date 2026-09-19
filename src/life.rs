//! One life of the engine, driven from a host.
//!
//! A life runs in a sandbox: the preamble first, in a module of its own, then the engine, then `boot`, which is
//! given the two generators the preamble makes. From then the host drives it by running one word at a time in
//! that sandbox, the way an operator calls a verb, and by answering what the sandbox asks of it.
//!
//! What a sandbox is, this does not say. [`Session`] is the whole of it: run this code, and answer the host while
//! it runs. A sandbox of monty is one of those, and a fake one is another, which is how the loop below is held to
//! its behaviour without a sandbox at all.

use crate::{
  ENGINE, PREAMBLE,
  fact::Value,
  host::{Gate, Outside},
  record::Entry,
  voice::{Ears, Said},
  world::World,
};

/// What answers the sandbox while its code runs.
pub trait Host {
  /// One call of the sandbox, answered: the name of the generator that made it, and what it handed over, plain.
  fn called(&mut self, name: &str, said: &Value) -> Value;
}

/// Where the engine runs.
///
/// One of these serves one life. Everything inside it is the life's own: the engine, the module of every chain,
/// and every word a model wrote. Nothing of the host is in there, and the only way out is [`Host`].
pub trait Session {
  /// One word, run in the sandbox, and what it gave.
  ///
  /// The host answers every call the word makes while it runs, which is how the World and the gate are reached.
  fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, String>;
}

/// What a life could not do.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal(pub String);

impl std::fmt::Display for Refusal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::error::Error for Refusal {}

/// One life: a sandbox with the engine in it, and the outside it reaches.
#[derive(Debug)]
pub struct Life<S, W, G> {
  /// Where the engine runs.
  session: S,
  /// The World it hears through and the gate its words are read by.
  outside: Outside<W, G>,
  /// What its host has said and it has not heard.
  ears: Ears,
  /// The root chain, which every life opens under the one name.
  root: String,
}

impl<S: Session, W: World, G: Gate> Life<S, W, G> {
  /// A life, opened from what a World kept of the life before it.
  ///
  /// The preamble runs in a module of its own, so the globals of a chain hold what the engine defines and nothing
  /// more. The engine runs next, and `boot` is given the two generators the preamble makes.
  pub fn boot(mut session: S, world: W, gate: G, ears: Ears, record: &[Entry]) -> Result<Self, Refusal> {
    let mut outside = Outside::new(world, gate);
    session.run(PREAMBLE, &mut outside).map_err(Refusal)?;
    let kept: Vec<Value> = record.iter().map(Entry::as_value).collect();
    let root = session.run(&opening(&Value::List(kept).plain()), &mut outside).map_err(Refusal)?;
    let root = root.as_str().unwrap_or_default().to_owned();
    Ok(Life { session, outside, ears, root })
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// One word of the operator, run on a chain, and what it gave.
  ///
  /// This is the one way in: a verb of the engine is called by running a word that calls it, which is what an
  /// operator does from python today.
  pub fn word(&mut self, word: &str) -> Result<Value, Refusal> {
    let asked = format!("asked(__engine, {word:?})");
    let said = self.session.run(&asked, &mut self.outside).map_err(Refusal)?;
    Ok(Value::of_plain(&said))
  }

  /// Everything the host has said into its Voice, done in the life, in the order it was said.
  ///
  /// A command says what it wrote while it runs, a model answers an ask long after the ask was heard, and the
  /// operator answers a prompt whenever the operator answers it. Nothing of the life hears any of that until it
  /// is said in there, which is what this does, and it is the one door they come through, so what a host said
  /// first is done first.
  pub fn heard(&mut self) -> Result<usize, Refusal> {
    let said = self.ears.drained();
    if said.is_empty() {
      return Ok(0);
    }
    let held: Vec<Value> = said.iter().map(Said::plain).collect();
    let word = format!("does(__engine, {})", shown(&Value::List(held)));
    self.session.run(&word, &mut self.outside).map_err(Refusal)?;
    Ok(said.len())
  }

  /// What an act came to, and nothing at all while it waits.
  ///
  /// A host says what it owes first, since a fact it is holding may be the very one that settles the act, and
  /// then asks. It never waits in here: how long to wait for a command or a model is the host's to decide, and
  /// what this gives is one honest look.
  pub fn came(&mut self, act: &str) -> Result<Option<Value>, Refusal> {
    self.heard()?;
    let asked = format!("({act:?} in outcomes, peek({act:?}))");
    let said = self.word(&asked)?;
    let held = said.as_entries().unwrap_or_default();
    match held.first() {
      Some(Value::Bool(true)) => Ok(Some(held.get(1).cloned().unwrap_or(Value::None))),
      _ => Ok(None),
    }
  }

  /// The World of this life, which a host reads what it kept of the life off.
  pub fn world(&self) -> &W {
    &self.outside.world
  }
}

impl<W: World, G: Gate> Host for Outside<W, G> {
  fn called(&mut self, name: &str, said: &Value) -> Value {
    Outside::called(self, name, said)
  }
}

/// The word that opens a life: the engine in a module of its own, and boot.
///
/// The preamble is the session's own namespace, so the engine is given one of its own beside it, and the globals
/// of a chain hold what the engine defines and nothing of the boundary.
fn opening(record: &Value) -> String {
  format!("__engine = module({ENGINE:?})\n__root = opened(__engine, {}, host)\n", shown(record))
}

/// One value as the word that says it again, which is how a host hands a value to the sandbox.
fn shown(value: &Value) -> String {
  match value {
    Value::None => "None".to_owned(),
    Value::Bool(held) => if *held { "True" } else { "False" }.to_owned(),
    Value::Int(held) => held.to_string(),
    Value::Float(held) => format!("{held:?}"),
    Value::Str(held) => format!("{held:?}"),
    Value::List(held) | Value::Tuple(held) => {
      let each: Vec<String> = held.iter().map(shown).collect();
      format!("[{}]", each.join(", "))
    }
    Value::Map(held) => {
      let each: Vec<String> = held.iter().map(|(key, one)| format!("{key:?}: {}", shown(one))).collect();
      format!("{{{}}}", each.join(", "))
    }
    Value::Shape { .. } | Value::Error { .. } | Value::Show | Value::Held(_) => shown(&value.plain()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    fact::Fact,
    host::{KERNEL, WORLD},
    voice::Voice,
    world::Reply,
  };

  /// A sandbox of the test: it keeps every word it was given and answers by a script.
  #[derive(Default)]
  struct Spoke {
    ran: Vec<String>,
    gives: Vec<Value>,
    asks: Vec<(String, Value)>,
  }

  impl Session for Spoke {
    fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, String> {
      self.ran.push(code.to_owned());
      for (name, said) in std::mem::take(&mut self.asks) {
        host.called(&name, &said);
      }
      Ok(if self.gives.is_empty() { Value::None } else { self.gives.remove(0) })
    }
  }

  /// A World of the test that keeps what it heard.
  #[derive(Default)]
  struct Sand {
    heard: Vec<String>,
  }

  impl World for Sand {
    fn hears(&mut self, fact: &Fact) -> Reply {
      self.heard.push(fact.kind().to_owned());
      Reply::Nothing
    }
  }

  /// A gate of the test that finds nothing.
  struct Open;

  impl Gate for Open {
    fn gate(&mut self, _word: &str, _ladder: &[String], _shape: &str) -> Vec<String> {
      Vec::new()
    }
  }

  /// A life whose sandbox is the fake one, with the root it was scripted to give.
  fn life(said: Spoke) -> (Life<Spoke, Sand, Open>, Voice) {
    let (voice, ears) = Ears::made();
    let held = Life::boot(said, Sand::default(), Open, ears, &[]).unwrap();
    (held, voice)
  }

  #[test]
  fn a_life_opens_the_preamble_in_a_module_of_its_own_then_the_engine_then_boot() {
    let said = Spoke { gives: vec![Value::None, Value::Str("chain://operator.1".to_owned())], ..Spoke::default() };
    let (held, _) = life(said);
    assert_eq!(held.root(), "chain://operator.1");
    assert!(held.session.ran[0].contains("def outside("), "the preamble is the session's own namespace");
    let opened = &held.session.ran[1];
    assert!(opened.contains("__engine = module("), "{opened}");
    assert!(opened.contains("__root = opened(__engine, [], host)"), "{opened}");
  }

  #[test]
  fn a_word_of_the_operator_runs_in_the_sandbox_and_what_it_gave_comes_back() {
    let said = Spoke {
      gives: vec![Value::None, Value::Str("chain://operator.1".to_owned()), Value::Int(42).plain()],
      ..Spoke::default()
    };
    let (mut held, _) = life(said);
    assert_eq!(held.word("close(42)").unwrap(), Value::Int(42));
    assert_eq!(held.session.ran[2], "asked(__engine, \"close(42)\")");
  }

  #[test]
  fn what_the_sandbox_asks_while_a_word_runs_is_answered_by_the_world() {
    let heard =
      Value::Tuple(vec![Value::Str("keep".to_owned()), Value::Str(String::new()), Value::Str("journal".to_owned())])
        .plain();
    let said = Spoke {
      gives: vec![Value::None, Value::Str("chain://operator.1".to_owned()), Value::None],
      asks: vec![(WORLD.to_owned(), heard)],
      ..Spoke::default()
    };
    let (mut held, _) = life(said);
    held.word("prompt(int, 'work')").unwrap();
    assert_eq!(held.world().heard, vec!["keep".to_owned()]);
  }

  #[test]
  fn what_a_host_says_into_its_voice_is_done_in_the_life_in_the_order_it_was_said() {
    let said = Spoke { gives: vec![Value::None, Value::Str("chain://operator.1".to_owned())], ..Spoke::default() };
    let (mut held, voice) = life(said);
    assert_eq!(held.heard().unwrap(), 0);
    voice.send(Fact::new("out", "bash://operator.1.1", WORLD, vec![Value::Str("one\n".to_owned())]));
    voice.close("prompt://operator.2", Value::Int(3));
    assert_eq!(held.heard().unwrap(), 2);
    let word = held.session.ran.last().unwrap();
    assert!(word.starts_with("does(__engine, "), "{word}");
    assert!(word.find("\"out\"").unwrap() < word.find("\"close\"").unwrap(), "{word}");
    assert_eq!(held.heard().unwrap(), 0);
  }

  #[test]
  fn what_an_act_came_to_is_nothing_at_all_while_it_waits() {
    let waiting = Value::Tuple(vec![Value::Bool(false), Value::None]).plain();
    let over = Value::Tuple(vec![Value::Bool(true), Value::Int(3)]).plain();
    let said = Spoke {
      gives: vec![Value::None, Value::Str("chain://operator.1".to_owned()), waiting, over],
      ..Spoke::default()
    };
    let (mut held, _) = life(said);
    assert_eq!(held.came("prompt://operator.2").unwrap(), None);
    assert_eq!(held.came("prompt://operator.2").unwrap(), Some(Value::Int(3)));
    let asked = held.session.ran.last().unwrap();
    assert!(asked.contains("\\\"prompt://operator.2\\\" in outcomes"), "{asked}");
  }

  #[test]
  fn the_gate_is_the_one_question_the_kernel_asks_of_the_host() {
    let asked = Value::Tuple(vec![
      Value::Str("gate".to_owned()),
      Value::Str("close(1)".to_owned()),
      Value::List(vec![]),
      Value::Str("int".to_owned()),
    ])
    .plain();
    let said = Spoke {
      gives: vec![Value::None, Value::Str("chain://operator.1".to_owned()), Value::None],
      asks: vec![(KERNEL.to_owned(), asked)],
      ..Spoke::default()
    };
    let (mut held, _) = life(said);
    held.word("rung('close(1)')").unwrap();
    assert!(held.world().heard.is_empty());
  }
}
