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
  verb::{Verb, shown},
  voice::{Ears, Said},
  world::World,
};

/// The name the sandbox reaches its host by, which the preamble calls and nothing else does.
pub const HOST: &str = "host";

/// What answers the sandbox while its code runs.
pub trait Host {
  /// One call of the sandbox, answered: the name of the generator that made it, and what it handed over, plain.
  fn called(&mut self, name: &str, said: &Value) -> Value;
}

/// Where the engine runs.
///
/// One of these serves one life. Everything inside it is the life's own: the engine, the module of every chain,
/// and every word a model wrote. Nothing of the host is in there, and the only way out is [`Host`].
///
/// A session does three things, and nothing else:
///
/// - It runs python in one namespace, which stands from one call to the next, so what one call binds a later
///   call reads.
/// - It binds [`HOST`] to a call of the host: one name and one plain value go in, and one plain value comes
///   back. The preamble calls it, and that is how the World and the gate of the host are reached from inside.
/// - It gives back the value of the last expression of the code, plain, and nothing for code that ends in none.
pub trait Session {
  /// One piece of code, run in the sandbox, and what the last expression of it gave.
  ///
  /// The host answers every call the code makes while it runs, which is how the World and the gate are reached.
  /// What the code raises is the fault: the exception, plain, which is its name and what it was made with.
  fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, Value>;
}

/// What a life could not do.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
  /// What the engine raised, as the exception it is: its name, and what it was made with.
  ///
  /// A call the engine will not make raises `Refused` in the one that made it, and a host reads which it was by
  /// the name, so that it tells a call it may not make from one it made wrong.
  Raised(Value),
  /// What the crate could not read of what the engine gave.
  Read(String),
}

impl Refusal {
  /// The name of what the engine raised, and nothing for a fault of the reading.
  pub fn name(&self) -> &str {
    match self {
      Refusal::Raised(Value::Error { name, .. }) => name,
      _ => "",
    }
  }

  /// Whether the engine refused the call, which is the one fault a word of a model makes on purpose.
  pub fn refused(&self) -> bool {
    self.name() == "Refused"
  }
}

impl std::fmt::Display for Refusal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Refusal::Raised(held) => f.write_str(&crate::turn::repr(held)),
      Refusal::Read(held) => f.write_str(held),
    }
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
    session.run(PREAMBLE, &mut outside).map_err(one)?;
    let kept: Vec<Value> = record.iter().map(Entry::as_value).collect();
    let got = session.run(&opening(&Value::List(kept).plain()), &mut outside).map_err(one)?;
    let Some(root) = got.as_str().map(str::to_owned) else {
      return Err(Refusal::Read(format!("a life opens on a chain, and {got:?} is none")));
    };
    Ok(Life { session, outside, ears, root })
  }

  /// The root chain of the life, which is the first act of any record.
  pub fn root(&self) -> &str {
    &self.root
  }

  /// One verb of the engine, called, and what the verb gave.
  ///
  /// This is the way a host works a life: [`crate::verb`] holds one shape for each verb of the contract, and a
  /// field of it left unsaid is a word the call never carries, so the engine takes its own.
  pub fn calls<V: Verb>(&mut self, verb: V) -> Result<V::Gave, Refusal> {
    let got = self.word(&verb.word())?;
    V::gave(&got)
  }

  /// One word of the operator, run on a chain, and what it gave.
  ///
  /// A verb of the engine is called by running a word that calls it, which is what an operator does from python
  /// today. [`Life::calls`] makes the word of every verb the contract declares, and this takes any other.
  pub fn word(&mut self, word: &str) -> Result<Value, Refusal> {
    let asked = format!("asked(__engine, {})", shown(&Value::Str(word.to_owned())));
    let said = self.session.run(&asked, &mut self.outside).map_err(one)?;
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
    self.session.run(&word, &mut self.outside).map_err(one)?;
    Ok(said.len())
  }

  /// Wait until the host says something, or until this long has passed, and say whether anything waits.
  ///
  /// A life goes on when a fact is said in it, so while it waits for a model, a command or a person, nothing of
  /// it moves until its host speaks. This is how a host waits for its own work without asking over and over.
  pub fn waits(&mut self, how_long: std::time::Duration) -> bool {
    self.ears.waits(how_long)
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

  /// The World of this life, to change: a host that holds state in its World reaches it here.
  pub fn world_mut(&mut self) -> &mut W {
    &mut self.outside.world
  }
}

impl<W: World, G: Gate> Host for Outside<W, G> {
  fn called(&mut self, name: &str, said: &Value) -> Value {
    Outside::called(self, name, said)
  }
}

/// One fault of the sandbox, as the life reads it.
fn one(raised: Value) -> Refusal {
  Refusal::Raised(Value::of_plain(&raised))
}

/// The word that opens a life: the engine in a module of its own, and boot.
///
/// The preamble is the session's own namespace, so the engine is given one of its own beside it, and the globals
/// of a chain hold what the engine defines and nothing of the boundary.
fn opening(record: &Value) -> String {
  format!(
    "__engine = module({})\n__root = opened(__engine, {}, {HOST})\n__root\n",
    shown(&Value::Str(ENGINE.to_owned())),
    shown(record)
  )
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
    raises: Option<Value>,
  }

  impl Session for Spoke {
    fn run(&mut self, code: &str, host: &mut dyn Host) -> Result<Value, Value> {
      self.ran.push(code.to_owned());
      for (name, said) in std::mem::take(&mut self.asks) {
        host.called(&name, &said);
      }
      if let Some(held) = self.raises.take() {
        return Err(held);
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
    assert!(opened.ends_with("__root\n"), "a life opens on the chain the last expression gives");
  }

  #[test]
  fn what_the_engine_raised_is_what_the_call_could_not_do() {
    let said = Spoke { gives: vec![Value::None, Value::Str("chain://operator.1".to_owned())], ..Spoke::default() };
    let (mut held, _) = life(said);
    held.session.raises = Some(Value::refused("a close of str is no int").plain());
    let no = held.word("close('one')").unwrap_err();
    assert!(no.refused(), "{no}");
    assert_eq!(no.name(), "Refused");
    assert_eq!(no.to_string(), "Refused('a close of str is no int')");
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
