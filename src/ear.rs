//! The ears of a host, by name: everything a host gives a life, since an ear is the one thing the life hears by.
//!
//! The contract lets `boot`, `act` and `drive` take any generator: it hears the facts of a life and the questions
//! offered to it, and speaks by say and act while it hears. The World is one of them, and so is every other ear of
//! the outside. Each is heard here, by the name the engine hears it under.
//!
//! An ear answers where it is asked. An ear of the host hears from outside the sandbox, which cannot run a verb
//! while it waits for the ear, so the ear says a verb as data, [`Reply::Calls`], say among them, and the value comes
//! back through [`Ears::answered`]. What it says is said under its name, since it speaks while it hears.
//!
//! The work an ear begins, a command, a wait, a model's turn, outlives its hearing. It speaks later through the
//! [`Voice`] of that ear, which says under the name of the ear, from any thread.

use std::{
  collections::VecDeque,
  sync::{Arc, Mutex},
  task::Waker,
};

use crate::{
  fact::Fact,
  value::{Fault, Object, ObjectRef},
};

/// What an ear answers when it has heard a fact.
#[derive(Debug, Clone)]
pub enum Reply {
  /// Nothing more: the ear is done hearing, and waits for the next fact.
  Nothing,
  /// What the ear raised, which is raised in the ear where it stands in the sandbox.
  Raised(Fault),
  /// The ear is over: it hears nothing more, as a generator that returned.
  Over,
  /// One verb of the engine, said by its name with its words, whose value comes back through
  /// [`Ears::answered`].
  Calls { name: String, args: Vec<Object>, kwargs: Vec<(String, Object)> },
}

/// The ears of a host, by name.
///
/// An ear may come to be while the life lives, as a generator a verb was given does, so a name is heard for the
/// first time when the engine first hears it, with nothing: that is its birth.
pub trait Ears {
  /// The life opened, and the Voice its ears speak with once a hearing is over, which [`Voice::of`] gives to
  /// each ear under its name.
  fn opened(&mut self, voice: Voice) {
    let _ = voice;
  }

  /// One fact, heard by the ear of this name, or nothing at the birth of that ear.
  fn hears(&mut self, name: &str, fact: Option<&Fact>) -> Reply;

  /// The value of the verb the ear of this name said, and what it says now that it holds it.
  fn answered(&mut self, name: &str, got: ObjectRef<'_>) -> Reply;

  /// One callable of the host, called back by the name it crossed under, with these words, and what it gave.
  /// The ears of a host that hands over no callable never hear this.
  fn called(
    &mut self,
    name: &str,
    args: Vec<Object>,
    kwargs: Vec<(String, Object)>,
  ) -> Result<Object, Fault> {
    let _ = (args, kwargs);
    Err(Fault::refused(format!("{name} is no callable of the host")))
  }
}

/// One verb the work of an ear said once its hearing was over: who says it, the verb, and its words.
#[derive(Debug, Clone)]
pub(crate) struct Said {
  pub by: String,
  pub name: String,
  pub args: Vec<Object>,
}

impl Said {
  /// The saying as the stand-in reads it: who says it, the verb, and its words.
  pub(crate) fn object(&self) -> Object {
    Object::tuple([
      Object::string(self.by.clone()),
      Object::string(self.name.clone()),
      Object::list(self.args.clone()),
    ])
  }
}

/// What a life drains of its Voice, and the waker of whoever drives the life.
#[derive(Default)]
struct Waiting {
  said: VecDeque<Said>,
  waker: Option<Waker>,
}

/// What the work of an ear speaks into a life with, once the hearing that began it is over.
///
/// It says under the name of one ear, which [`Voice::of`] gives, and the operator's when it gives none. It is cheap
/// to clone and may be carried to any thread, so a command speaks from where it runs. What is said is heard in the
/// order it was said, the next time the life is driven, and saying it wakes whoever drives.
#[derive(Clone)]
pub struct Voice {
  waiting: Arc<Mutex<Waiting>>,
  by: String,
}

impl Default for Voice {
  fn default() -> Self {
    Voice { waiting: Arc::default(), by: "operator".to_owned() }
  }
}

impl Voice {
  /// The voice of one ear, which says under its name.
  #[must_use]
  pub fn of(&self, ear: &str) -> Voice {
    Voice { waiting: Arc::clone(&self.waiting), by: ear.to_owned() }
  }

  /// One fact: its kind, what it is about, and its words, which the bus makes whole under the name of the ear.
  pub fn say(&self, kind: &str, about: &str, words: impl IntoIterator<Item = Object>) {
    let mut args = vec![Object::string(kind), Object::string(about)];
    args.extend(words);
    self.verb("say", args);
  }

  /// One verb of the engine with its words, as a close of a prompt the operator answered, or a pause of a chain.
  pub fn verb(&self, name: &str, args: Vec<Object>) {
    let waker = {
      let Ok(mut held) = self.waiting.lock() else { return };
      held.said.push_back(Said { by: self.by.clone(), name: name.to_owned(), args });
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
    f.debug_tuple("Voice").field(&self.by).finish()
  }
}
