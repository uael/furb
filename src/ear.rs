//! The ears of a host, by name: every ear the contract lets a host give a life, as the contract has them.
//!
//! The contract lets `boot`, `act` and `drive` take any generator: it hears every fact and speaks by yielding a
//! saying. The World is one, and it has a trait of its own, [`World`](crate::World), since what the engine asks of
//! it is known. Every other ear is heard here, by the name the engine hears it under, and so is the World when a
//! host hands it over as a generator, which a host that is python does.
//!
//! An ear answers where it is asked. What it says of a fact is one saying, which the bus says whole and hands
//! back to the ear as the next fact it hears, so an ear that has two things to say says the second on hearing
//! the first. An ear that answers from a thread of its own, as a generator of python does, cannot say a verb of
//! the engine itself: it says the verb as data, [`Reply::Calls`], and the value comes back through
//! [`Ears::answered`].

use crate::{
  fact::Fact,
  value::{Fault, Object, ObjectRef},
};

/// What an ear answers when it has heard a fact.
#[derive(Debug, Clone)]
pub enum Reply {
  /// Nothing at all, which is the answer to every fact the ear only hears.
  Nothing,
  /// One saying, which the bus makes whole and hands back to the ear as the next fact it hears.
  Say(Fact),
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
