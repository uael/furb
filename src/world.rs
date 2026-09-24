//! The World, as a host writes one: what the engine asks of the machine it runs on.
//!
//! The contract makes the World one generator that hears every fact. A host that is Rust does not write a
//! generator; it writes this trait, and the crate stands in the generator's place inside the sandbox and turns
//! each question of the engine into one call here. What the World answers at once it answers by returning; what
//! takes time, a model's turn, a wait, a prompt of the operator, it gives back as a future, which the life drives
//! and whose value it says into the engine as the fact the engine waits for.
//!
//! What the engine asks of every World is a method here. Every other fact reaches [`World::hears`]: a question of an
//! extension, the start of an act of a kind the World says it does, a control, and what the World answers it answers
//! as an ear does. An act of an extension may speak while it runs, as a command writes what it writes, so a World is
//! given a [`Voice`] once, when the life opens, and it speaks through it from wherever the work runs.

use std::{
  collections::VecDeque,
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex},
  task::Waker,
};

use crate::{
  ear::Reply,
  fact::Fact,
  value::{Fault, Object, ObjectRef},
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

/// One thing a World said while nothing asked it to.
#[derive(Debug, Clone)]
pub enum Said {
  /// One fact, said as the World: what an act of an extension says as it runs, or how it ended.
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
/// It is cheap to clone and may be carried to any thread, so work speaks from where it runs. What is said
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
/// What the engine asks of every World is a method here: what a chain stands on, a reading of the clock, a number
/// drawn, the turn of a model, a wait, a prompt put to the operator, and what the journal says to keep. Every other
/// fact of the life reaches [`World::hears`], which answers it as an ear answers: a question of an extension with
/// plain data, a start of an act of a kind in [`World::kinds`] by doing it. A start of a kind no World does is
/// closed with a refusal, so no word waits for it.
pub trait World {
  /// The life opened, and the Voice this World speaks with when nothing asked it to.
  fn opened(&mut self, voice: Voice) {
    let _ = voice;
  }

  /// The kinds of act this World does beside a wait and a prompt, which it hears the start of.
  fn kinds(&self) -> Vec<String> {
    Vec::new()
  }

  /// What a chain stands on.
  fn stand(&mut self) -> Standing;

  /// A reading of the clock.
  fn clock(&mut self) -> f64;

  /// A number drawn between zero and one.
  fn chance(&mut self) -> f64;

  /// One entry of the record, kept.
  fn keep(&mut self, entry: ObjectRef<'_>);

  /// The turn of a model asked on a chain, given what it reads: the answer, as the engine reads a turn.
  fn ask(&mut self, rung: &str, on: &str, actor: &str, turns: ObjectRef<'_>) -> Later<Object>;

  /// A wait of this many seconds.
  fn wait(&mut self, seconds: f64) -> Later<()>;

  /// A prompt put to the operator: what the operator answered, or the refusal of a shape the World cannot ask.
  fn prompt(&mut self, about: &str, shape: &str, message: &str) -> Later<Result<Object, Fault>>;

  /// Every other fact of the life, as the engine said it: a question of an extension, the start of an act of one
  /// of its kinds, a control, and whatever an act says. It is answered as an ear answers, and a verb it says
  /// comes back through [`World::answered`].
  fn hears(&mut self, fact: &Fact) -> Reply {
    let _ = fact;
    Reply::Nothing
  }

  /// The value of the verb the World said, as `("value", v)` or `("raised", fault)`, and what it says now.
  fn answered(&mut self, got: ObjectRef<'_>) -> Reply {
    let _ = got;
    Reply::Nothing
  }
}
