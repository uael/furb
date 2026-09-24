//! The World, as a host writes one: what the engine asks of the machine it runs on.
//!
//! The contract makes the World one generator that hears every fact. A host that is Rust does not write a
//! generator; it writes this trait, and the crate stands in the generator's place inside the sandbox and turns
//! each question of the engine into one call here. What the World answers at once it answers by returning; what
//! takes time, a model's turn, a command, a wait, a prompt of the operator, it gives back as a future, which the
//! life drives and whose value it says into the engine as the fact the engine waits for.
//!
//! A command is the one thing that speaks while it runs: it says what it wrote as it writes it, and its code when
//! it ends. So a World is given a [`Voice`] once, when the life opens, and a command speaks through it from
//! wherever it runs.

use std::{
  collections::VecDeque,
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::Waker,
};

use crate::{
  fact::Fact,
  value::{Fault, Object, ObjectRef, Text},
};

/// What a World gives back for work that takes time: a future the life drives, on the thread of the life.
pub type Later<T> = Pin<Box<dyn Future<Output = T>>>;

/// One actor of the roster: its name, the efforts it may be asked at, and the window it reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Actor {
  pub name: String,
  pub efforts: Vec<String>,
  pub window: i64,
}

/// What a chain stands on: the roster, the directory its paths resolve in, and the actor it asks by default.
#[derive(Debug, Clone, PartialEq)]
pub struct Standing {
  pub roster: Vec<Actor>,
  pub directory: String,
  pub actor: String,
}

impl Standing {
  /// The standing as the engine reads it: a list of the roster, the directory and the actor.
  pub fn object(&self) -> Object {
    let roster = self.roster.iter().map(|one| {
      Object::list([
        Object::string(one.name.clone()),
        Object::list(one.efforts.iter().map(|e| Object::string(e.clone()))),
        Object::int(one.window),
      ])
    });
    Object::list([
      Object::list(roster),
      Object::string(self.directory.clone()),
      Object::string(self.actor.clone()),
    ])
  }
}

/// One command the engine started, as the World is given it.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
  /// The act, which is what the command's facts are about.
  pub about: String,
  /// The directory the command runs in, which the chain said.
  pub here: String,
  /// The command line.
  pub command: String,
  /// Whether the engine will feed its stdin.
  pub fed: bool,
  /// The seconds it may run.
  pub timeout: f64,
  /// Whether its stderr goes with its stdout.
  pub merged: bool,
}

/// A command that runs: what the engine may do to it while it does.
pub trait Running {
  /// One text into its stdin, and nothing to close it.
  fn feed(&mut self, text: Option<String>);
  /// The command ended before its time, which is a cancel.
  fn slay(&mut self);
}

/// One thing a World said while nothing asked it to.
#[derive(Debug, Clone)]
pub enum Said {
  /// One fact, said as the World: what a command wrote, or its code when it ended.
  Fact(Fact),
  /// One act closed with a value, which is how a prompt of the operator is answered.
  Closed { id: String, value: Object },
  /// One chain paused, which a World does when a model answers nothing twice.
  Paused(String),
}

impl Said {
  /// The saying as the stand-in reads it: its mark, then what it carries.
  pub(crate) fn object(&self) -> Object {
    match self {
      Said::Fact(one) => Object::tuple([Object::string("fact"), one.0.clone()]),
      Said::Closed { id, value } => {
        Object::tuple([Object::string("close"), Object::string(id.clone()), value.clone()])
      }
      Said::Paused(id) => Object::tuple([Object::string("pause"), Object::string(id.clone())]),
    }
  }
}

/// What a life drains of its Voice, and the waker of whoever drives the life.
#[derive(Default)]
struct Waiting {
  said: VecDeque<Said>,
  waker: Option<Waker>,
}

/// What a World speaks into a life with, for what it says unasked.
///
/// It is cheap to clone and may be carried to any thread, so a command speaks from where it runs. What is said
/// is heard in the order it was said, the next time the life is driven, and saying it wakes whoever drives.
#[derive(Clone, Default)]
pub struct Voice {
  waiting: Arc<Mutex<Waiting>>,
}

impl Voice {
  /// One fact, said as the World.
  pub fn say(&self, fact: Fact) {
    self.holds(Said::Fact(fact));
  }

  /// What a command wrote on one of its streams.
  pub fn out(&self, about: &str, text: &str, stream: &str) {
    self.say(Fact::says("out", about, [Object::string(text), Object::string(stream)]));
  }

  /// The code a command ended with, or nothing for one ended at its timeout.
  pub fn exited(&self, about: &str, code: Option<i64>) {
    self.say(Fact::says("exited", about, [code.map_or_else(Object::none, Object::int)]));
  }

  /// One act, closed with a value.
  pub fn close(&self, id: impl Into<String>, value: Object) {
    self.holds(Said::Closed { id: id.into(), value });
  }

  /// One chain, paused.
  pub fn pause(&self, id: impl Into<String>) {
    self.holds(Said::Paused(id.into()));
  }

  fn holds(&self, one: Said) {
    let waker = {
      let Ok(mut held) = self.waiting.lock() else { return };
      held.said.push_back(one);
      held.waker.take()
    };
    if let Some(waker) = waker {
      waker.wake();
    }
  }

  /// Everything said and not yet heard, and the waker to wake when more is said.
  /// Whether nothing said is still to be heard.
  pub(crate) fn quiet(&self) -> bool {
    self.waiting.lock().map_or(true, |held| held.said.is_empty())
  }

  pub(crate) fn drained(&self, waker: &Waker) -> Vec<Said> {
    let Ok(mut held) = self.waiting.lock() else { return Vec::new() };
    held.waker = Some(waker.clone());
    held.said.drain(..).collect()
  }
}

impl std::fmt::Debug for Voice {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("Voice")
  }
}

/// The World: the machine a life runs on, as the engine asks of it.
///
/// Everything the engine asks of a World is here, and nothing else: what a chain stands on, a reading of the
/// clock, a number drawn, a read or a write of a path, the turn of a model, a command, a wait, a prompt put to the
/// operator, and what the journal says to keep. Where a path resolves is the chain's to say, so a read and a
/// write are given the directory the chain stands in.
pub trait World {
  /// The life opened, and the Voice this World speaks with when nothing asked it to.
  fn opened(&mut self, voice: Voice) {
    let _ = voice;
  }

  /// What a chain stands on.
  fn stand(&mut self) -> Standing;

  /// The text at a path, resolved from where the chain stands, or the refusal.
  fn read(&mut self, here: &str, path: &str) -> Result<Text, Fault>;

  /// The content onto the path, resolved from where the chain stands, and the text as it stands after.
  fn write(&mut self, here: &str, path: &str, content: &str) -> Result<Text, Fault>;

  /// A reading of the clock.
  fn clock(&mut self) -> f64;

  /// A number drawn between zero and one.
  fn chance(&mut self) -> f64;

  /// One entry of the record, kept.
  fn keep(&mut self, entry: ObjectRef<'_>);

  /// The turn of a model asked on a chain, given what it reads: the answer, as the engine reads a turn.
  fn ask(&mut self, rung: &str, on: &str, actor: &str, turns: ObjectRef<'_>) -> Later<Object>;

  /// A command started: it speaks through the Voice as it runs, and this is what the engine may do to it.
  fn run(&mut self, command: Command) -> Box<dyn Running>;

  /// A wait of this many seconds.
  fn wait(&mut self, seconds: f64) -> Later<()>;

  /// A prompt put to the operator: what the operator answered, or the refusal of a shape the World cannot ask.
  fn prompt(&mut self, about: &str, shape: &str, message: &str) -> Later<Result<Object, Fault>>;
}
