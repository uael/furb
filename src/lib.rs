//! The core of an AI harness: the engine in a sandbox, behind an async API, with a World of your own.
//!
//! The engine is one python file, `src/furb/engine.py`, which the contract beside it specifies: it is the whole
//! system prompt of a model, and all it takes from the outside is ears: generators that hear every fact, each
//! under a name, the World, the Kernel and the gate among them. This crate runs that file inside monty, a python
//! interpreter written in rust for untrusted code, and gives a host three things.
//!
//! [`Life`] is one life of the engine. Its methods are the verbs of the contract, `rung`, `prompt` and the rest,
//! with the same names and the same words, and what an act comes to is awaited as an [`Act`]. A verb of an
//! extension is said by its name, through [`Life::verb`].
//!
//! [`World`] is what a host writes: what the engine asks of the machine it runs on, as one trait, and every other
//! fact of the life, which the parts of its extensions hear. What the World answers at once, it returns; what takes
//! time, it gives back as a future, which the life drives. An act of an extension speaks while it runs through a
//! [`Voice`].
//!
//! [`extension`] reads the extensions a host plays: the config, the cache, the manifests, the order and the words,
//! the same for every host.
//!
//! The Kernel is the crate's. The word of a rung is gated by the type checker of monty, reading the word on
//! the sheet of `furb.sheet`, every name of the engine bound first, and it runs where the engine runs, in the module
//! of its chain.
//!
//! What crosses is what monty carries, [`Object`], and an exception reads as [`Fault`]. Inside the sandbox,
//! [`PREAMBLE`] stands in for every ear of the host, in a module of its own, so the globals of a chain hold what
//! the engine defines and nothing more.

pub mod ear;
pub mod extension;
pub mod fact;
pub mod gate;
pub mod life;
mod sand;
pub mod value;
pub mod world;

pub mod binding;

pub use crate::{
  ear::{Ears, Reply},
  fact::Fact,
  life::{Act, Came, Life, Opening},
  extension::Extension,
  value::{Fault, Object, ObjectRef},
  world::{Actor, Later, Said, Standing, Voice, World},
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
    assert!(PREAMBLE.contains("def called("));
    assert!(PREAMBLE.contains(&format!("IS = {:?}", value::IS)));
    assert!(SHEET.contains("def gate("));
  }
}
