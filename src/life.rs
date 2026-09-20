//! One life of the engine, driven from a host.
//!
//! A life runs in a sandbox: the preamble first, in a module of its own, then the engine, then `boot`, which is
//! given the two generators the preamble makes. From then the host drives it by running one word at a time in
//! that sandbox, the way an operator calls a verb, and by answering what the sandbox asks of it.
//!
//! The sandbox is monty, and it is [`Sand`]. There is no other: a life runs where the engine runs, and the whole
//! of the crate is the engine in that sandbox.

use crate::{
  ENGINE, PREAMBLE,
  fact::Value,
  host::{Gate, Outside},
  record::Entry,
  sand::Sand,
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
      Refusal::Raised(Value::Error { name, args }) => {
        let each: Vec<&str> = args.iter().filter_map(Value::as_str).collect();
        if each.is_empty() { f.write_str(name) } else { write!(f, "{name}: {}", each.join(" ")) }
      }
      Refusal::Raised(held) => write!(f, "{held:?}"),
      Refusal::Read(held) => f.write_str(held),
    }
  }
}

impl std::error::Error for Refusal {}

/// One life: a sandbox with the engine in it, and the outside it reaches.
#[derive(Debug)]
pub struct Life<W, G> {
  /// Where the engine runs, which is monty.
  sand: Sand,
  /// The World it hears through and the gate its words are read by.
  outside: Outside<W, G>,
  /// What its host has said and it has not heard.
  ears: Ears,
  /// The root chain, which every life opens under the one name.
  root: String,
}

impl<W: World, G: Gate> Life<W, G> {
  /// A life, opened from what a World kept of the life before it.
  ///
  /// The preamble runs in a module of its own, so the globals of a chain hold what the engine defines and nothing
  /// more. The engine runs next, and `boot` is given the two generators the preamble makes.
  pub fn boot(mut sand: Sand, world: W, gate: G, ears: Ears, record: &[Entry]) -> Result<Self, Refusal> {
    let mut outside = Outside::new(world, gate);
    sand.run(PREAMBLE, &mut outside).map_err(one)?;
    let kept: Vec<Value> = record.iter().map(Entry::as_value).collect();
    let got = sand.run(&opening(&Value::List(kept).plain()), &mut outside).map_err(one)?;
    let Some(root) = got.as_str().map(str::to_owned) else {
      return Err(Refusal::Read(format!("a life opens on a chain, and {got:?} is none")));
    };
    Ok(Life { sand, outside, ears, root })
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
    let said = self.sand.run(&asked, &mut self.outside).map_err(one)?;
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
    self.sand.run(&word, &mut self.outside).map_err(one)?;
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

  /// The gate of this life, which a host reads what its gate held off.
  pub fn gate(&self) -> &G {
    &self.outside.gate
  }

  /// The gate of this life, to change: a host that holds state in its gate reaches it here.
  pub fn gate_mut(&mut self) -> &mut G {
    &mut self.outside.gate
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
