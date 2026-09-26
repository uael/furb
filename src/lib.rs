//! The core of an AI harness: the engine in a sandbox, behind an async API, on ears of your own.
//!
//! The engine is one python file, `src/furb/engine.py`, which the contract beside it specifies: it is the whole
//! system prompt of a model, and all it takes from the outside is ears: generators that hear every fact, each under
//! a name, the ears of the World, the Kernel and the gate among them. This crate runs that file inside monty, a
//! python interpreter written in rust for untrusted code, and gives a host two things.
//!
//! [`Engine`] is one life of the engine. Its methods are the verbs of the contract, `read`, `bash`, `prompt` and the
//! rest, with the same names and the same words, made from the contract when the crate is built, and what an act
//! comes to is awaited as an [`Act`].
//!
//! [`Ear`] is what a host writes, and the one thing an engine hears by: a generator, which in rust is a coroutine
//! that [`ear::ear`] makes. The World is ears of the outside that serve the machine, one or many, and [`world`] holds
//! the ones the crate writes: files, commands, time and the store of the record. A host gives them to an engine
//! beside its own, in the order the engine offers them a question, so an ear of its own that comes first takes a
//! question in their place.
//!
//! The Kernel is the crate's. The word of a rung is gated by the type checker of monty, reading the word on the sheet
//! of `furb.sheet`, every name of the engine bound first, and it runs where the engine runs, in the module of its
//! chain.
//!
//! What crosses is what monty carries, [`Object`], and the engine's own classes read as [`Text`], [`Exit`] and
//! [`Fault`]. Inside the sandbox, [`PREAMBLE`] stands in for every ear of the host, in a module of its own, so the
//! globals of a chain hold what the engine defines and nothing more.

pub mod ear;
pub mod engine;
pub mod fact;
pub mod gate;
mod sand;
pub mod value;
mod wire;
pub mod world;

pub mod binding;

pub use crate::{
  ear::{Ear, Heard, Step, Voice},
  engine::{Act, Came, Engine, verbs},
  fact::Fact,
  value::{Exit, Fault, Object, ObjectRef, Text},
};

/// The engine: the one file the sandbox runs, and the whole system prompt of a model.
///
/// The crate carries the same file the python package ships, so the engine a host runs and the engine a model
/// reads are one thing.
pub const ENGINE: &str = include_str!("furb/engine.py");

/// The sheet the gate reads a word on, which the Kernel of the sandbox writes the same way the python package does.
pub const SHEET: &str = include_str!("furb/sheet.py");

/// The stand-in, which runs in the sandbox in a module of its own.
pub const PREAMBLE: &str = include_str!("preamble.py");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_engine_the_crate_carries_is_the_engine_the_package_ships() {
    let shipped =
      std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/furb/engine.py")).unwrap();
    assert_eq!(ENGINE, shipped);
    assert!(ENGINE.contains("def boot(record=(), **outside):"));
  }

  #[test]
  fn the_stand_in_carries_the_one_mark_of_the_crossing() {
    assert!(PREAMBLE.contains("def opened("));
    assert!(PREAMBLE.contains("def crossing("));
    assert!(PREAMBLE.contains(&format!("IS = {:?}", value::IS)));
    assert!(SHEET.contains("def gate("));
  }
}
