//! The core of an AI harness: the engine in a sandbox, and an ear of your own.
//!
//! The engine is one python file, `src/furb/engine.py`, which the contract beside it specifies. This crate runs
//! that file inside monty, a python interpreter written in rust for untrusted code, and gives a host that is not
//! python two things: [`Life`], which is one life of the engine, and [`Ear`], which is what the host writes.
//!
//! The engine takes the World and the Kernel as generators. The crate takes care of the Kernel: the word of a
//! rung is gated by ty, reading it against the contract, and runs where the engine runs, in the module of its
//! chain. A host writes the World, and any other ear it wants a life to have, and every value that crosses to
//! it is plain: [`Value`] is the shape of one.
//!
//! [`PREAMBLE`] is the boundary. It runs in the sandbox, in a module of its own, and stands in for every ear of
//! the host: it hears a fact, hands it over plain, and says back what the host answers. Nothing of the boundary
//! is bound in the engine, so the globals of a chain hold what the file defines and nothing more.

pub mod ear;
pub mod fact;
pub mod gate;
pub mod life;
pub mod sand;

pub use crate::{
  ear::{Ear, Ears, Host, Reply},
  fact::{Fact, Value},
  life::{Life, Refusal},
  sand::Sand,
};

/// The engine: the one file the sandbox runs, and the whole system prompt of a model.
///
/// The crate carries the same file the python package ships, so the engine a host runs and the engine a model
/// reads are one thing.
pub const ENGINE: &str = include_str!("furb/engine.py");

/// The contract: the typed surface of the engine, whose docstrings hold every law.
///
/// The gate reads the word of a rung against this, since what a word may say is what the contract declares.
pub const CONTRACT: &str = include_str!("furb/engine.pyi");

/// The boundary, which runs in the sandbox in a module of its own.
pub const PREAMBLE: &str = include_str!("preamble.py");

/// The sheet the gate reads a word on, which the Kernel of the sandbox writes the same way the python package does.
pub const SHEET: &str = include_str!("furb/sheet.py");

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_engine_the_crate_carries_is_the_engine_the_package_ships() {
    let shipped = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/furb/engine.py")).unwrap();
    assert_eq!(ENGINE, shipped);
    assert!(ENGINE.contains("def boot(record=(), **outside):"));
  }

  #[test]
  fn the_preamble_carries_the_marks_of_the_plain_form() {
    assert!(PREAMBLE.contains("def opened("));
    assert!(SHEET.contains("def gate("));
    assert!(PREAMBLE.contains(&format!("TUPLE = {:?}", fact::TUPLE)));
    assert!(PREAMBLE.contains(&format!("SHOW = {:?}", fact::SHOW)));
  }
}
